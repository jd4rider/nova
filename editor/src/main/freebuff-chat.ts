import { spawn, type ChildProcess } from 'node:child_process'
import type { FreeBuffChatEvent } from '../shared/types'
import { Terminal } from '@xterm/headless'
import { assistantText, screenLines, transcriptLines } from './freebuff-screen'

let ptyModule: any = null
try {
  ptyModule = require('node-pty')
} catch {
  ptyModule = null
}

/**
 * FreeBuff has no headless mode. This session runs the real TUI in a PTY,
 * reconstructs the screen with a headless terminal, and turns screen snapshots
 * into structured messages. The prompt/screen layout is heuristic and is
 * reported as such in the UI; no fake transcript is generated.
 */
export class FreeBuffChat {
  private pty: any = null
  private child: ChildProcess | null = null
  private term: Terminal | null = null
  private root: string | null = null
  private timer: ReturnType<typeof setTimeout> | null = null
  private lastText = ''
  private lastDataAt = 0
  private busy = false
  private ready = false
  private stopped = false
  private userPrompt = ''
  private baseline: string[] = []
  private readonly onEvent: (event: FreeBuffChatEvent) => void

  constructor(onEvent: (event: FreeBuffChatEvent) => void) {
    this.onEvent = onEvent
  }

  start(root: string): void {
    this.stop()
    this.root = root
    this.stopped = false
    this.busy = false
    this.ready = false
    this.lastText = ''
    this.userPrompt = ''
    this.baseline = []
    this.term = new Terminal({ cols: 110, rows: 36, allowProposedApi: true, scrollback: 2000 })
    this.emit({ type: 'status', status: 'starting' })

    if (ptyModule) {
      try {
        this.pty = ptyModule.spawn('freebuff', ['--cwd', root], {
          cwd: root,
          cols: 110,
          rows: 36,
          name: 'xterm-256color',
          env: { ...process.env, TERM: 'xterm-256color' }
        })
        this.pty.onData((data: string) => this.feed(data))
        this.pty.onExit(({ exitCode }: { exitCode: number }) => this.exited(exitCode))
        this.emit({ type: 'status', status: 'started', detail: 'FreeBuff PTY' })
        return
      } catch (error) {
        this.emit({ type: 'error', message: `Could not start FreeBuff: ${(error as Error).message}` })
      }
    }

    this.child = spawn('freebuff', ['--cwd', root], {
      cwd: root,
      env: { ...process.env, TERM: 'xterm-256color' },
      stdio: ['pipe', 'pipe', 'pipe']
    })
    this.child.stdout?.on('data', (chunk: Buffer) => this.feed(chunk.toString('utf8')))
    this.child.stderr?.on('data', (chunk: Buffer) => this.feed(chunk.toString('utf8')))
    this.child.on('exit', (code) => this.exited(code ?? 1))
    this.child.on('error', (error) => this.emit({ type: 'error', message: error.message }))
    this.emit({
      type: 'status',
      status: 'started',
      detail: 'pipe mode: install node-pty for the full TUI'
    })
  }

  send(text: string): void {
    const clean = text.trim()
    if (!clean || !this.term) return
    this.baseline = transcriptLines(screenLines(this.term))
    this.userPrompt = clean
    if (this.pty) {
      this.pty.write(clean.replace(/\r?\n/g, ' '))
      setTimeout(() => { try { this.pty?.write('\r') } catch { /* exited */ } }, 120)
    }
    else this.child?.stdin?.write(clean.replace(/\r?\n/g, ' ') + '\n')
    this.busy = true
    this.ready = false
    this.lastText = ''
    this.lastDataAt = Date.now()
    this.emit({ type: 'user', text: clean })
    this.emit({ type: 'status', status: 'busy', detail: 'FreeBuff is working' })
    this.schedule()
  }

  reset(): void {
    if (!this.root) return
    this.start(this.root)
  }

  stop(): void {
    this.stopped = true
    if (this.timer) clearTimeout(this.timer)
    this.timer = null
    try {
      if (this.pty) {
        try { process.kill(-this.pty.pid, 'SIGKILL') } catch { /* no process group */ }
        this.pty.kill()
      }
    } catch {
      // Already gone.
    }
    this.pty = null
    try {
      this.child?.kill()
    } catch {
      // Already gone.
    }
    this.child = null
    this.term?.dispose()
    this.term = null
  }

  private feed(data: string): void {
    if (!this.term || this.stopped) return
    this.lastDataAt = Date.now()
    this.term.write(data, () => this.schedule())
  }

  private schedule(): void {
    if (this.timer) clearTimeout(this.timer)
    this.timer = setTimeout(() => {
      this.timer = null
      this.scan()
    }, 450)
  }

  private scan(): void {
    if (!this.term || this.stopped) return
    const lines = screenLines(this.term)
    const promptVisible = lines.some((line) => /Enter a coding task/.test(line))
    const text = assistantText(lines, this.userPrompt, this.baseline)
    const quiet = Date.now() - this.lastDataAt

    if (promptVisible && !this.busy) {
      if (!this.ready) {
        this.ready = true
        this.emit({ type: 'status', status: 'ready', detail: 'FreeBuff is ready' })
      }
    } else if (promptVisible && this.busy && quiet > 1_500) {
      this.busy = false
      this.ready = true
      this.emit({ type: 'status', status: 'ready', detail: 'FreeBuff finished this turn' })
    }
    if (text && text !== this.lastText) {
      this.lastText = text
      this.emit({ type: 'screen', text })
    }
  }

  private exited(code: number): void {
    if (this.stopped) return
    this.stopped = true
    this.emit({ type: 'exit', message: `FreeBuff exited (${code}).` })
    this.term?.dispose()
    this.term = null
  }

  private emit(event: FreeBuffChatEvent): void {
    this.onEvent(event)
  }
}
