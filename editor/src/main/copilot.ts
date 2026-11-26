import { app, shell } from 'electron'
import { spawn, type ChildProcessWithoutNullStreams } from 'node:child_process'
import path from 'node:path'
import type { AiContext, AiMessage } from '../shared/types'

export interface CopilotState {
  available: boolean
  authenticated: boolean
  user: string | null
  message: string
  busy: boolean
  deviceCode: string | null
}

interface Pending {
  resolve: (value: any) => void
  reject: (error: Error) => void
  timer: NodeJS.Timeout
}

interface ChatTurn {
  onDelta: (text: string) => void
  resolve: () => void
  reject: (error: Error) => void
  text: string
  settled: boolean
  requestId: number | null
}

const DEFAULT_MODEL = 'gpt-4o'

function copilotServerScript(): string {
  const entry = require.resolve('@github/copilot-language-server')
  return path.join(path.dirname(entry), 'language-server.js')
}

function workspaceUri(root: string): string {
  return 'file://' + encodeURI(root)
}

function textFromProgress(value: any): string {
  const chunks: string[] = []
  const seen = new Set<string>()
  const ignored = new Set([
    'kind',
    'title',
    'id',
    'conversationId',
    'turnId',
    'error',
    'code',
    'status',
    'model',
    'modelName',
    'agentSlug',
    'billingMultiplier'
  ])
  const wanted = new Set(['text', 'delta', 'content', 'markdown', 'reply', 'message', 'value'])
  const walk = (node: any, key: string): void => {
    if (typeof node === 'string') {
      if (!wanted.has(key) || !node.trim() || seen.has(node)) return
      seen.add(node)
      chunks.push(node)
      return
    }
    if (Array.isArray(node)) {
      for (const item of node) walk(item, key)
      return
    }
    if (!node || typeof node !== 'object') return
    for (const [childKey, child] of Object.entries(node)) {
      if (ignored.has(childKey)) continue
      walk(child, childKey)
    }
  }
  walk(value, '')
  return chunks.join('')
}

function turnsFrom(messages: AiMessage[]): Array<{ request: string; response?: string }> {
  const turns: Array<{ request: string; response?: string }> = []
  let current: { request: string; response?: string } | null = null
  for (const message of messages) {
    if (message.role === 'user') {
      if (current) turns.push(current)
      current = { request: message.content }
    } else if (current) {
      current.response = message.content
    }
  }
  if (current) turns.push(current)
  return turns.length > 0 ? turns : [{ request: 'Hello' }]
}

export class CopilotService {
  private child: ChildProcessWithoutNullStreams | null = null
  private buffer = Buffer.alloc(0)
  private nextId = 1
  private pending = new Map<number, Pending>()
  private chats = new Map<string, ChatTurn>()
  private docs = new Map<string, { version: number; text: string }>()
  private root: string | null = null
  private starting: Promise<void> | null = null
  private state: CopilotState = {
    available: true,
    authenticated: false,
    user: null,
    message: 'GitHub Copilot starts when you open a folder or sign in.',
    busy: false,
    deviceCode: null
  }

  constructor(private readonly onStatus: (state: CopilotState) => void) {}

  status(): CopilotState {
    return { ...this.state }
  }

  private publish(patch: Partial<CopilotState>): void {
    this.state = { ...this.state, ...patch }
    this.onStatus(this.status())
  }

  async start(root: string | null): Promise<void> {
    if (root) this.root = root
    if (this.child) return
    if (this.starting) return this.starting
    this.starting = this.launch()
    try {
      await this.starting
    } finally {
      this.starting = null
    }
  }

  private async launch(): Promise<void> {
    const root = this.root ?? process.cwd()
    let script: string
    try {
      script = copilotServerScript()
    } catch {
      this.publish({
        available: false,
        message: 'The Copilot language server is not installed in this build.'
      })
      return
    }
    this.publish({ available: true, message: 'Starting GitHub Copilot…' })
    const child = spawn(process.execPath, [script, '--stdio'], {
      cwd: root,
      env: { ...process.env, ELECTRON_RUN_AS_NODE: '1' },
      stdio: ['pipe', 'pipe', 'pipe']
    })
    this.child = child
    child.stdout.on('data', (chunk: Buffer) => this.receive(chunk))
    child.stderr.on('data', (chunk: Buffer) => {
      const text = chunk.toString('utf8').trim()
      if (text) this.publish({ message: text.slice(0, 300) })
    })
    child.on('error', (error) => {
      if (this.child === child) this.child = null
      this.failAll(error)
      this.publish({ authenticated: false, message: `Copilot failed to start: ${error.message}` })
    })
    child.on('exit', (code) => {
      if (this.child !== child) return
      this.child = null
      this.failAll(new Error(`Copilot language server exited (${code ?? 1})`))
      this.publish({ authenticated: false, busy: false, message: `Copilot stopped (${code ?? 1}).` })
    })
    try {
      await this.request('initialize', {
        processId: process.pid,
        workspaceFolders: [{ uri: workspaceUri(root), name: path.basename(root) || 'workspace' }],
        capabilities: {
          workspace: { workspaceFolders: true },
          window: { workDoneProgress: true, showDocument: { support: true } }
        },
        initializationOptions: {
          editorInfo: { name: 'Nova Studio', version: app.getVersion() },
          editorPluginInfo: { name: 'Nova Studio', version: app.getVersion() }
        }
      })
      this.notify('initialized', {})
      this.notify('workspace/didChangeConfiguration', {
        settings: { telemetry: { telemetryLevel: 'off' } }
      })
      this.publish({ message: 'Connected to GitHub Copilot.' })
    } catch (error) {
      this.publish({ message: `Copilot initialization failed: ${(error as Error).message}` })
    }
  }

  async signIn(): Promise<{ userCode: string; verificationUri: string }> {
    await this.start(this.root)
    if (!this.child) throw new Error(this.state.message || 'Copilot is not available.')
    const result: any = await this.request('signIn', {})
    const userCode = typeof result?.userCode === 'string' ? result.userCode : ''
    const command = result?.command
    if (command?.command) {
      void this.request('workspace/executeCommand', {
        command: command.command,
        arguments: Array.isArray(command.arguments) ? command.arguments : []
      }).catch(() => undefined)
    }
    this.publish({
      deviceCode: userCode || null,
      message: userCode
        ? `Enter code ${userCode} at github.com/login/device. Waiting for GitHub…`
        : 'Complete the GitHub sign-in in your browser.'
    })
    return { userCode, verificationUri: 'https://github.com/login/device' }
  }

  async signOut(): Promise<CopilotState> {
    if (this.child) await this.request('signOut', {}).catch(() => undefined)
    this.publish({ authenticated: false, user: null, deviceCode: null, message: 'Signed out of GitHub Copilot.' })
    return this.status()
  }

  async models(): Promise<Array<{ id: string; label: string; default?: boolean }>> {
    await this.start(this.root)
    if (!this.child) return []
    const result: any = await this.request('copilot/models', {})
    const list = Array.isArray(result) ? result : Array.isArray(result?.models) ? result.models : []
    return list.map((model: any) => ({
      id: String(model.id ?? model.modelFamily ?? ''),
      label: String(model.modelName ?? model.name ?? model.id ?? ''),
      default: Boolean(model.isChatDefault)
    })).filter((model: any) => model.id)
  }

  async chat(
    requestId: string,
    messages: AiMessage[],
    context: AiContext | null,
    model: string,
    onDelta: (text: string) => void,
    signal: AbortSignal
  ): Promise<void> {
    await this.start(this.root)
    if (!this.child) throw new Error(this.state.message || 'Copilot is not running.')
    if (!this.state.authenticated) {
      throw new Error('Sign in with GitHub Copilot in the AI panel first.')
    }
    if (signal.aborted) throw new Error('aborted')

    const turn: ChatTurn = {
      onDelta,
      resolve: () => undefined,
      reject: () => undefined,
      text: '',
      settled: false,
      requestId: null
    }
    const done = new Promise<void>((resolve, reject) => {
      turn.resolve = resolve
      turn.reject = reject
    })
    this.chats.set(requestId, turn)
    const abort = () => {
      this.notify('window/workDoneProgress/cancel', { token: requestId })
      if (turn.requestId) {
        const message = JSON.stringify({ jsonrpc: '2.0', method: '$/cancelRequest', params: { id: turn.requestId } })
        void this.write(message).catch(() => undefined)
      }
      if (!turn.settled) {
        turn.settled = true
        this.chats.delete(requestId)
        turn.reject(new Error('aborted'))
      }
    }
    signal.addEventListener('abort', abort, { once: true })

    const uri = context?.path ? workspaceUri(context.path) : null
    if (uri && context) this.syncDocument(uri, context.content, 1)

    const params: any = {
      workDoneToken: requestId,
      turns: turnsFrom(messages),
      workspaceFolder: this.root ? workspaceUri(this.root) : undefined,
      workspaceFolders: this.root ? [{ uri: workspaceUri(this.root), name: path.basename(this.root) }] : undefined,
      chatMode: 'Ask',
      source: 'panel',
      computeSuggestions: false,
      modelInfo: { id: model || DEFAULT_MODEL }
    }
    if (uri) {
      params.textDocument = { uri }
      if (context?.selection) params.selection = undefined
    }

    const id = this.nextId++
    turn.requestId = id
    const message = JSON.stringify({ jsonrpc: '2.0', id, method: 'conversation/create', params })
    await this.write(message).catch((error) => {
      this.chats.delete(requestId)
      throw error
    })
    const timer = setTimeout(() => {
      if (turn.settled) return
      turn.settled = true
      this.chats.delete(requestId)
      turn.reject(new Error('Copilot chat timed out.'))
    }, 180_000)
    const pending: Pending = {
      resolve: () => {
        if (turn.settled) return
        turn.settled = true
        this.chats.delete(requestId)
        turn.resolve()
      },
      reject: (error) => {
        if (turn.settled) return
        turn.settled = true
        this.chats.delete(requestId)
        turn.reject(error)
      },
      timer
    }
    this.pending.set(id, pending)

    try {
      await done
    } finally {
      clearTimeout(timer)
      signal.removeEventListener('abort', abort)
      this.chats.delete(requestId)
      this.pending.delete(id)
    }
  }

  cancelChat(requestId: string): void {
    const turn = this.chats.get(requestId)
    if (!turn) return
    this.notify('window/workDoneProgress/cancel', { token: requestId })
    if (turn.requestId) {
      const message = JSON.stringify({ jsonrpc: '2.0', method: '$/cancelRequest', params: { id: turn.requestId } })
      void this.write(message).catch(() => undefined)
    }
    turn.settled = true
    this.chats.delete(requestId)
    turn.resolve()
  }

  async complete(input: {
    uri: string
    version: number
    text: string
    line: number
    character: number
  }): Promise<{ insertText: string; range?: any; command?: any } | null> {
    await this.start(this.root)
    if (!this.child || !this.state.authenticated) return null
    this.syncDocument(input.uri, input.text, input.version)
    this.notify('textDocument/didFocus', { textDocument: { uri: input.uri } })
    const result: any = await this.request('textDocument/inlineCompletion', {
      textDocument: { uri: input.uri, version: input.version },
      position: { line: input.line, character: input.character },
      context: { triggerKind: 1 },
      formattingOptions: { tabSize: 4, insertSpaces: true }
    }, 12_000)
    const item = result?.items?.[0]
    if (!item?.insertText) return null
    this.notify('textDocument/didShowCompletion', { item })
    return { insertText: item.insertText, range: item.range, command: item.command }
  }

  stop(): void {
    const child = this.child
    this.child = null
    this.root = null
    this.docs.clear()
    for (const [, turn] of this.chats) {
      turn.settled = true
      turn.reject(new Error('Copilot stopped'))
    }
    this.chats.clear()
    this.failAll(new Error('Copilot stopped'))
    if (child) {
      try {
        child.kill()
      } catch {
        // Already exited.
      }
    }
  }

  private syncDocument(uri: string, text: string, version: number): void {
    const existing = this.docs.get(uri)
    if (!existing) {
      this.notify('textDocument/didOpen', {
        textDocument: { uri, languageId: 'nova', version, text }
      })
    } else if (existing.text !== text) {
      this.notify('textDocument/didChange', {
        textDocument: { uri, version },
        contentChanges: [{ text }]
      })
    }
    this.docs.set(uri, { version, text })
  }

  private write(payload: string): Promise<void> {
    const child = this.child
    if (!child) return Promise.reject(new Error('Copilot is not running.'))
    return new Promise((resolve, reject) => {
      child.stdin.write(`Content-Length: ${Buffer.byteLength(payload)}\r\n\r\n${payload}`, (error) => {
        if (error) reject(error)
        else resolve()
      })
    })
  }

  private request(method: string, params: unknown, timeout = 20_000): Promise<any> {
    const id = this.nextId++
    const message = JSON.stringify({ jsonrpc: '2.0', id, method, params })
    return this.write(message).then(
      () =>
        new Promise<any>((resolve, reject) => {
          const timer = setTimeout(() => {
            this.pending.delete(id)
            reject(new Error(`Copilot request timed out: ${method}`))
          }, timeout)
          this.pending.set(id, { resolve, reject, timer })
        })
    )
  }

  private notify(method: string, params: unknown): void {
    const message = JSON.stringify({ jsonrpc: '2.0', method, params })
    void this.write(message).catch(() => undefined)
  }

  private respond(id: number | string, result: unknown): void {
    const message = JSON.stringify({ jsonrpc: '2.0', id, result })
    void this.write(message).catch(() => undefined)
  }

  private receive(chunk: Buffer): void {
    this.buffer = Buffer.concat([this.buffer, chunk])
    for (;;) {
      const separator = this.buffer.indexOf('\r\n\r\n')
      if (separator < 0) return
      const header = this.buffer.subarray(0, separator).toString('utf8')
      const match = /Content-Length:\s*(\d+)/i.exec(header)
      if (!match) {
        this.buffer = this.buffer.subarray(separator + 4)
        continue
      }
      const length = Number(match[1])
      const start = separator + 4
      if (this.buffer.length < start + length) return
      const body = this.buffer.subarray(start, start + length).toString('utf8')
      this.buffer = this.buffer.subarray(start + length)
      try {
        this.handle(JSON.parse(body))
      } catch {
        // Ignore malformed protocol frames.
      }
    }
  }

  private handle(message: any): void {
    if (typeof message.id === 'number' && ('result' in message || 'error' in message)) {
      const pending = this.pending.get(message.id)
      if (!pending) return
      this.pending.delete(message.id)
      clearTimeout(pending.timer)
      if (message.error) pending.reject(new Error(message.error.message ?? 'Copilot request failed'))
      else pending.resolve(message.result)
      return
    }
    if (typeof message.id === 'number' || typeof message.id === 'string') {
      if (message.method === 'workspace/configuration') {
        this.respond(message.id, [{}])
      } else if (message.method === 'window/showDocument') {
        const uri = String(message.params?.uri ?? '')
        if (uri.startsWith('https://')) void shell.openExternal(uri)
        this.respond(message.id, { success: Boolean(uri) })
      } else if (message.method === 'workspace/applyEdit') {
        this.respond(message.id, { applied: false })
      } else {
        this.respond(message.id, null)
      }
      return
    }

    if (message.method === 'didChangeStatus/v2') {
      const statuses: any[] = Array.isArray(message.params?.statuses) ? message.params.statuses : []
      const auth = statuses.find((status) => status?.category === 'auth')
      const completion = statuses.find((status) => status?.category === 'completion')
      if (auth) {
        const user = typeof auth.result?.user === 'string' ? auth.result.user : null
        const ok = auth.kind !== 'Error' && (auth.result?.status === 'OK' || Boolean(user))
        this.publish({
          authenticated: ok,
          user,
          busy: Boolean(completion?.busy),
          deviceCode: ok ? null : this.state.deviceCode,
          message: ok
            ? `Signed in as ${user ?? 'GitHub'}.`
            : auth.message || 'GitHub Copilot is not signed in.'
        })
      } else if (completion) {
        this.publish({ busy: Boolean(completion.busy) })
      }
      return
    }
    if (message.method === 'statusNotification') {
      const status = message.params ?? {}
      this.publish({
        busy: Boolean(status.busy),
        message: typeof status.message === 'string' && status.message ? status.message : this.state.message
      })
      return
    }
    if (message.method === 'conversation/preconditionsNotification') {
      if (message.params?.status && message.params.status !== 'ok') {
        this.publish({ message: `Copilot is not ready: ${message.params.status}.` })
      }
      return
    }
    if (message.method === '$/progress') {
      this.handleProgress(message.params)
      return
    }
    if (message.method === 'window/logMessage') {
      const text = String(message.params?.message ?? '')
      if (message.params?.type === 1 && text) this.publish({ message: text.slice(0, 300) })
    }
  }

  private handleProgress(params: any): void {
    const token = String(params?.token ?? '')
    const turn = this.chats.get(token)
    if (!turn) return
    const value = params?.value ?? {}
    if (value.kind === 'end') {
      if (!turn.settled) {
        if (value.error?.message) {
          turn.settled = true
          this.chats.delete(token)
          turn.reject(new Error(String(value.error.message)))
        } else {
          turn.settled = true
          this.chats.delete(token)
          turn.resolve()
        }
      }
      return
    }
    const text = textFromProgress(value)
    if (!text) return
    if (text === turn.text) return
    if (text.startsWith(turn.text)) {
      const delta = text.slice(turn.text.length)
      turn.text = text
      if (delta) turn.onDelta(delta)
    } else {
      turn.text += text
      turn.onDelta(text)
    }
  }

  private failAll(error: Error): void {
    for (const [, pending] of this.pending) {
      clearTimeout(pending.timer)
      pending.reject(error)
    }
    this.pending.clear()
  }
}
