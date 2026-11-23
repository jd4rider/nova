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
  mode: 'terminal'
  reason: string
}

export interface TerminalInfo {
  id: number
  pty: boolean
}

export type ThemeName = 'gruvbox-dark' | 'gruvbox-light' | 'gruvbox-hc'

export interface NovaApi {
  workspace: {
    choose(): Promise<{ root: string; hasManifest: boolean } | null>
    root(): Promise<string | null>
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
  freebuff: {
    status(): Promise<FreeBuffStatus>
  }
  app: {
    version(): Promise<string>
    platform: string
    onMenu(cb: (command: string) => void): () => void
  }
}
