import type { ChildProcessWithoutNullStreams } from 'node:child_process'
import { spawn } from 'node:child_process'
import type { LspDiagnostic } from '../shared/types'
import { novaExecutable } from './toolchain'

interface Pending {
  resolve: (value: unknown) => void
  reject: (error: Error) => void
  timer: NodeJS.Timeout
}

export class LspClient {
  private process: ChildProcessWithoutNullStreams | null = null
  private buffer = Buffer.alloc(0)
  private nextId = 1
  private pending = new Map<number, Pending>()

  constructor(
    private readonly onDiagnostics: (uri: string, diagnostics: LspDiagnostic[]) => void,
    private readonly onStatus: (status: string) => void
  ) {}

  async start(root: string): Promise<void> {
    this.stop()
    const executable = novaExecutable()
    this.onStatus('starting')
    const child = spawn(executable, ['lsp', '--stdio'], {
      cwd: root,
      env: process.env,
      stdio: ['pipe', 'pipe', 'pipe']
    })
    this.process = child
    child.stdout.on('data', (chunk: Buffer) => this.receive(chunk))
    child.stderr.on('data', (chunk: Buffer) => {
      const text = chunk.toString('utf8').trim()
      if (text) this.onStatus(`error: ${text.slice(0, 300)}`)
    })
    child.on('error', (error) => {
      this.onStatus(`error: ${error.message}`)
      this.failAll(error)
    })
    child.on('exit', (code) => {
      if (this.process === child) {
        this.process = null
        this.onStatus(code === 0 ? 'stopped' : `exited (${code})`)
        this.failAll(new Error(`language server exited with code ${code}`))
      }
    })
    try {
      await this.request('initialize', {
        processId: process.pid,
        rootUri: 'file://' + encodeURI(root),
        capabilities: {},
        workspaceFolders: [{ uri: 'file://' + encodeURI(root), name: 'workspace' }]
      })
      this.notify('initialized', {})
      this.onStatus('ready')
    } catch (error) {
      this.onStatus(`error: ${(error as Error).message}`)
      throw error
    }
  }

  async request(method: string, params: unknown): Promise<unknown> {
    const process = this.process
    if (!process) throw new Error('language server is not running')
    const id = this.nextId++
    const message = JSON.stringify({ jsonrpc: '2.0', id, method, params })
    await new Promise<void>((resolve, reject) => {
      process.stdin.write(`Content-Length: ${Buffer.byteLength(message)}\r\n\r\n${message}`, (error) => {
        if (error) reject(error)
        else resolve()
      })
    })
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => {
        this.pending.delete(id)
        reject(new Error(`language server request timed out: ${method}`))
      }, 15_000)
      this.pending.set(id, { resolve, reject, timer })
    })
  }

  notify(method: string, params: unknown): void {
    const process = this.process
    if (!process) return
    const message = JSON.stringify({ jsonrpc: '2.0', method, params })
    process.stdin.write(`Content-Length: ${Buffer.byteLength(message)}\r\n\r\n${message}`)
  }

  stop(): void {
    const process = this.process
    this.process = null
    if (!process) return
    this.failAll(new Error('language server stopped'))
    try {
      process.kill()
    } catch {
      // Process already gone.
    }
  }

  private failAll(error: Error): void {
    for (const [, pending] of this.pending) {
      clearTimeout(pending.timer)
      pending.reject(error)
    }
    this.pending.clear()
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
        // Ignore malformed messages from the child process.
      }
    }
  }

  private handle(message: any): void {
    if (typeof message.id === 'number' && (message.result !== undefined || message.error !== undefined)) {
      const pending = this.pending.get(message.id)
      if (!pending) return
      this.pending.delete(message.id)
      clearTimeout(pending.timer)
      if (message.error) pending.reject(new Error(message.error.message ?? 'language server error'))
      else pending.resolve(message.result)
      return
    }
    if (message.method === 'textDocument/publishDiagnostics') {
      const params = message.params ?? {}
      if (typeof params.uri === 'string' && Array.isArray(params.diagnostics)) {
        this.onDiagnostics(params.uri, params.diagnostics as LspDiagnostic[])
      }
      return
    }
    if (message.method === 'window/logMessage' && message.params?.message) {
      this.onStatus(String(message.params.message).slice(0, 300))
    }
  }
}
