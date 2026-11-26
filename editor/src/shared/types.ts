export interface FileEntry {
  name: string
  path: string
  isDir: boolean
}

export interface ReadResult {
  content: string
  mtimeMs: number
}

export interface WriteResult {
  mtimeMs: number
  conflict?: boolean
}

export interface TaskResult {
  code: number
  output: string
}

export interface Position {
  line: number
  character: number
}

export interface Range {
  start: Position
  end: Position
}

export interface LspDiagnostic {
  range: Range
  severity?: number
  code?: string | number
  source?: string
  message: string
}

export interface FreeBuffStatus {
  available: boolean
  version?: string
  mode: 'terminal' | 'scraped-chat'
  /** True when the installed CLI documents a non-interactive mode. 0.2.36 does not. */
  headless: boolean
  /** True when node-pty is available and the TUI can be driven in a real PTY. */
  pty: boolean
  reason: string
}

export interface FreeBuffChatEvent {
  type: 'status' | 'screen' | 'user' | 'error' | 'exit'
  status?: 'starting' | 'started' | 'busy' | 'ready'
  detail?: string
  text?: string
  message?: string
}

export interface CopilotStatus {
  available: boolean
  authenticated: boolean
  user: string | null
  message: string
  busy: boolean
  deviceCode: string | null
}

export interface CopilotModel {
  id: string
  label: string
  default?: boolean
}

export interface CopilotCompletion {
  insertText: string
  range?: Range
}

export type AiProviderId = 'anthropic' | 'openai' | 'copilot'

export interface AiProviderSettings {
  provider: AiProviderId
  model: string
  baseUrl: string
  hasApiKey: boolean
  keySource: 'stored' | 'environment' | 'github' | 'none'
}

export interface AiConfigurePatch {
  provider?: AiProviderId
  model?: string
  baseUrl?: string
  /** Omit to keep the stored key, pass null to delete it. */
  apiKey?: string | null
}

export interface AiContext {
  path: string
  language: string
  content: string
  selection?: string
}

export interface AiMessage {
  role: 'user' | 'assistant'
  content: string
}

export interface AiStreamEvent {
  requestId: string
  type: 'start' | 'delta' | 'done' | 'error'
  text?: string
  message?: string
  model?: string
}

export interface TerminalInfo {
  id: number
  pty: boolean
}

export type ThemeName = 'vs-dark' | 'gruvbox-dark' | 'gruvbox-light' | 'gruvbox-hc'

export interface NovaApi {
  workspace: {
    choose(): Promise<{ root: string; hasManifest: boolean } | null>
    /** The folder opened in the previous session, if it still exists. */
    restore(): Promise<{ root: string; hasManifest: boolean } | null>
    root(): Promise<string | null>
    /** Fires when files inside the open workspace change on disk. */
    onChanged(cb: (event: { paths: string[] }) => void): () => void
  }
  fs: {
    list(dir: string): Promise<FileEntry[]>
    read(file: string): Promise<ReadResult>
    write(file: string, content: string, expectedMtimeMs: number | null): Promise<WriteResult>
    create(file: string, content: string): Promise<void>
    mkdir(dir: string): Promise<void>
    rename(from: string, to: string): Promise<void>
    remove(target: string): Promise<void>
    reveal(target: string): Promise<void>
    files(): Promise<string[]>
  }
  search: {
    run(query: string, options: { regex: boolean; caseSensitive: boolean; wholeWord: boolean }): Promise<Array<{ file: string; line: number; column: number; text: string }>>
  }
  tasks: {
    run(kind: 'build' | 'run' | 'check' | 'test'): Promise<TaskResult>
    cancel(): Promise<void>
  }
  terminal: {
    create(cwd: string, cols: number, rows: number, command?: string): Promise<TerminalInfo>
    write(id: number, data: string): Promise<void>
    resize(id: number, cols: number, rows: number): Promise<void>
    dispose(id: number): Promise<void>
    onData(cb: (event: { id: number; data: string }) => void): () => void
  }
  lsp: {
    start(root: string): Promise<{ ok: boolean; error?: string }>
    stop(): Promise<void>
    request(method: string, params: unknown): Promise<unknown>
    notify(method: string, params: unknown): Promise<void>
    onDiagnostics(cb: (event: { uri: string; diagnostics: LspDiagnostic[] }) => void): () => void
    onStatus(cb: (status: string) => void): () => void
  }
  ai: {
    settings(): Promise<AiProviderSettings>
    configure(patch: AiConfigurePatch): Promise<AiProviderSettings>
    send(requestId: string, messages: AiMessage[], context: AiContext | null): Promise<void>
    cancel(requestId: string): Promise<void>
    onEvent(cb: (event: AiStreamEvent) => void): () => void
  }
  freebuff: {
    status(): Promise<FreeBuffStatus>
    start(root: string): Promise<void>
    send(text: string): Promise<void>
    reset(): Promise<void>
    stop(): Promise<void>
    complete(input: {
      uri: string
      version: number
      text: string
      line: number
      character: number
    }): Promise<CopilotCompletion | null>
    onEvent(cb: (event: FreeBuffChatEvent) => void): () => void
  }
  copilot: {
    status(): Promise<CopilotStatus>
    signIn(): Promise<{ userCode: string; verificationUri: string }>
    signOut(): Promise<CopilotStatus>
    models(): Promise<CopilotModel[]>
    complete(input: {
      uri: string
      version: number
      text: string
      line: number
      character: number
    }): Promise<CopilotCompletion | null>
    onStatus(cb: (status: CopilotStatus) => void): () => void
  }
  app: {
    version(): Promise<string>
    platform: string
    onMenu(cb: (command: string) => void): () => void
  }
}
