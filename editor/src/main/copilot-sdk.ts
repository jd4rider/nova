import { app } from 'electron'
import type { AiContext, AiMessage } from '../shared/types'

export interface CopilotState {
  available: boolean
  authenticated: boolean
  user: string | null
  message: string
  busy: boolean
  deviceCode: string | null
}

const SYSTEM = [
  'You are the coding assistant inside Nova Studio, a desktop IDE for the Nova programming language.',
  'Nova is statically typed with a Rust front end, an LLVM backend and a clang link step.',
  'Supported: functions, structs, enums, exhaustive match, generics, static traits, dyn Trait objects, impl methods, collections, closures, macros and extern "C".',
  'Not supported: trait impls on generic types, real async scheduling/channels, package resolution. Say so instead of inventing syntax.',
  'Prefer complete, compilable Nova. Keep explanations short. Tag code blocks as nova.'
].join('\n')

function promptFor(messages: AiMessage[], context: AiContext | null): string {
  const history = messages
    .map((message) => `${message.role === 'user' ? 'User' : 'Assistant'}: ${message.content}`)
    .join('\n\n')
  const attached = context
    ? `\n\nThe user is editing ${context.path}.${context.selection ? `\nSelected code:\n\`\`\`nova\n${context.selection.slice(0, 12_000)}\n\`\`\`` : ''}\nCurrent file:\n\`\`\`nova\n${context.content.slice(0, 60_000)}\n\`\`\``
    : '\n\nNo file is attached.'
  return `${history}${attached}`
}

/**
 * Chat, auth and model discovery through the official Copilot SDK. Inline
 * completion stays on the Copilot language server, which owns the low-latency
 * completion protocol.
 */
export class CopilotSdkService {
  private client: any = null
  private starting: Promise<void> | null = null
  private root: string | null = null
  private loginFlow: string | null = null
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
    if (this.client) return
    if (this.starting) return this.starting
    this.starting = this.launch()
    try {
      await this.starting
    } finally {
      this.starting = null
    }
  }

  private async launch(): Promise<void> {
    try {
      this.publish({ available: true, message: 'Starting GitHub Copilot SDK…' })
      const mod: any = await import('@microsoft/copilot-sdk')
      const client = new mod.CopilotClient({
        mode: 'coding-agent',
        github: { useLoggedInUser: true },
        clientInfo: { name: 'Nova Studio', version: app.getVersion() }
      })
      await client.start()
      this.client = client
      const auth = await client.getAuthStatus()
      this.publish({
        authenticated: Boolean(auth?.isAuthenticated),
        user: typeof auth?.login === 'string' ? auth.login : null,
        message: auth?.isAuthenticated
          ? `Signed in as ${auth.login ?? 'GitHub'} (Copilot SDK).`
          : 'GitHub Copilot is installed but not signed in.'
      })
    } catch (error) {
      this.client = null
      this.publish({
        available: false,
        authenticated: false,
        message: `Copilot SDK unavailable: ${(error as Error).message}`
      })
    }
  }

  async refresh(): Promise<CopilotState> {
    await this.start(this.root)
    if (this.client) {
      try {
        const auth = await this.client.getAuthStatus()
        this.publish({
          authenticated: Boolean(auth?.isAuthenticated),
          user: typeof auth?.login === 'string' ? auth.login : null,
          message: auth?.isAuthenticated
            ? `Signed in as ${auth.login ?? 'GitHub'}.`
            : auth?.statusMessage ?? 'Not signed in to GitHub Copilot.'
        })
      } catch {
        // Keep the last known status.
      }
    }
    return this.status()
  }

  async signIn(): Promise<{ userCode: string; verificationUri: string }> {
    await this.start(this.root)
    if (!this.client) throw new Error(this.state.message)
    this.publish({ message: 'Starting GitHub device sign-in…' })
    const begun = await this.client.rpc.accounts.login.begin({
      kind: 'githubDotCom',
      method: 'device-code'
    })
    const flowId = String(begun?.flowId ?? '')
    const step = begun?.step
    this.loginFlow = flowId
    if (step?.kind === 'device-code') {
      this.publish({
        deviceCode: String(step.userCode ?? ''),
        message: `Enter code ${step.userCode} at ${step.verificationUri}. Waiting for GitHub…`
      })
      void this.finishLogin(flowId).catch((error) => {
        this.publish({ message: `Copilot sign-in failed: ${(error as Error).message}` })
      })
      return {
        userCode: String(step.userCode ?? ''),
        verificationUri: String(step.verificationUri ?? 'https://github.com/login/device')
      }
    }
    if (step?.kind === 'open-url' && step.url) {
      const { shell } = await import('electron')
      void shell.openExternal(String(step.url))
      void this.finishLogin(flowId).catch(() => undefined)
      return { userCode: '', verificationUri: String(step.url) }
    }
    void this.finishLogin(flowId).catch(() => undefined)
    return { userCode: '', verificationUri: 'https://github.com/login/device' }
  }

  private async finishLogin(flowId: string): Promise<void> {
    let step: any = { kind: 'awaiting' }
    while (this.client && this.loginFlow === flowId) {
      step = await this.client.rpc.accounts.login.advance({ flowId })
      if (step?.kind === 'device-code') {
        this.publish({
          deviceCode: String(step.userCode ?? ''),
          message: `Enter code ${step.userCode} at ${step.verificationUri}.`
        })
        continue
      }
      if (step?.kind === 'open-url' && step.url) {
        const { shell } = await import('electron')
        void shell.openExternal(String(step.url))
        continue
      }
      if (step?.kind === 'error') throw new Error(String(step.message ?? 'Sign-in failed'))
      if (step?.kind === 'completed') break
    }
    this.loginFlow = null
    await this.refresh()
  }

  async signOut(): Promise<CopilotState> {
    const client = this.client
    this.client = null
    this.loginFlow = null
    if (client) await client.stop().catch(() => undefined)
    this.publish({
      authenticated: false,
      user: null,
      deviceCode: null,
      busy: false,
      message: 'Disconnected from the Copilot runtime. Credentials stay in the Copilot CLI store.'
    })
    return this.status()
  }

  async models(): Promise<Array<{ id: string; label: string; default?: boolean }>> {
    await this.start(this.root)
    if (!this.client) return []
    const models: any[] = await this.client.listModels()
    return models.map((model) => ({
      id: String(model.id),
      label: String(model.name ?? model.id),
      default: model.id === 'auto'
    }))
  }

  async chat(
    messages: AiMessage[],
    context: AiContext | null,
    model: string,
    onDelta: (text: string) => void,
    signal: AbortSignal
  ): Promise<void> {
    await this.start(this.root)
    if (!this.client) throw new Error(this.state.message)
    if (!this.state.authenticated) throw new Error('Sign in with GitHub Copilot first.')
    if (signal.aborted) throw new Error('aborted')
    this.publish({ busy: true })
    const session = await this.client.createSession({
      model: model || 'auto',
      workingDirectory: this.root ?? undefined,
      events: { streaming: true },
      permissions: {
        onRequest: async () => ({ kind: 'reject', feedback: 'Nova Studio chat is read-only.' })
      },
      systemMessage: { mode: 'append', content: SYSTEM }
    })
    let sawDelta = false
    let settled = false
    const unsubs: Array<() => void> = []
    const done = new Promise<void>((resolve, reject) => {
      const finish = (error?: Error) => {
        if (settled) return
        settled = true
        if (error) reject(error)
        else resolve()
      }
      unsubs.push(session.on('assistant.message_delta', (event: any) => {
        const delta = String(event?.data?.deltaContent ?? '')
        if (delta) {
          sawDelta = true
          onDelta(delta)
        }
      }))
      unsubs.push(session.on('assistant.message', (event: any) => {
        const text = String(event?.data?.content ?? '')
        if (!sawDelta && text) onDelta(text)
      }))
      unsubs.push(session.on('session.error', (event: any) => {
        finish(new Error(String(event?.data?.message ?? 'Copilot session failed.')))
      }))
      unsubs.push(session.on('session.idle', () => finish()))
    })
    const abort = () => {
      void session.abort().catch(() => undefined)
      settled = true
    }
    signal.addEventListener('abort', abort, { once: true })
    try {
      await session.send({ prompt: promptFor(messages, context) })
      let timer: NodeJS.Timeout | null = null
      await Promise.race([
        done,
        new Promise<void>((resolve) => {
          timer = setTimeout(resolve, 180_000)
        })
      ])
      if (timer) clearTimeout(timer)
      if (signal.aborted) throw new Error('aborted')
    } finally {
      signal.removeEventListener('abort', abort)
      for (const off of unsubs) off()
      await session.disconnect().catch(() => undefined)
      this.publish({ busy: false })
    }
  }

  stop(): void {
    const client = this.client
    this.client = null
    this.loginFlow = null
    if (client) void client.stop().catch(() => undefined)
  }
}
