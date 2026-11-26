import type { CopilotCompletion } from '../shared/types'
import { Terminal } from '@xterm/headless'
import { assistantText, screenLines } from './freebuff-screen'

let ptyModule: any = null
try {
  ptyModule = require('node-pty')
} catch {
  ptyModule = null
}

const delay = (ms: number) => new Promise<void>((resolve) => setTimeout(resolve, ms))

function hasComposer(term: Terminal): boolean {
  // "Add to the current task" is the busy composer and must not count.
  return screenLines(term).some((line) => /Enter a coding task/.test(line))
}

function extract(text: string): string | null {
  const blocks = [...text.matchAll(/```[^\n]*\n([\s\S]*?)```/g)]
  const raw = (blocks.length > 0 ? blocks[blocks.length - 1][1] : text)
    .replace(/^\s*(here(?:'s| is)[^\n]*|sure[^\n]*|completion:?)\s*\n/i, '')
    .replace(/\n\s*$/, '')
  const trimmed = raw.trim()
  if (!trimmed) return null
  // A completion is an insertion, not an explanation.
  if (!blocks.length && /^(I |Here|Sure|This |The completion)/i.test(trimmed)) return null
  return raw.length <= 800 ? raw : null
}

/**
 * FreeBuff 0.2.36 only exposes a chat TUI. This drives a dedicated PTY (not the
 * user's chat), asks for an insertion-only reply, and reads the answer back
 * from the screen. It is intentionally serialized and slow; callers debounce.
 */
export class FreeBuffCompletion {
  private active = false

  async complete(input: {
    uri: string
    text: string
    line: number
    character: number
    root: string
  }): Promise<CopilotCompletion | null> {
    if (this.active || !ptyModule) return null
    this.active = true
    const term = new Terminal({ cols: 110, rows: 36, allowProposedApi: true, scrollback: 200 })
    let pty: any = null
    try {
      pty = ptyModule.spawn('freebuff', ['--cwd', input.root], {
        cwd: input.root,
        cols: 110,
        rows: 36,
        name: 'xterm-256color',
        env: { ...process.env, TERM: 'xterm-256color' }
      })
      pty.onData((data: string) => term.write(data))

      for (let waited = 0; waited < 15_000; waited += 250) {
        if (hasComposer(term)) break
        await delay(250)
      }
      if (!hasComposer(term)) return null
      // The composer placeholder paints before the TUI accepts input. Without
      // this settle window the prompt is silently dropped.
      await delay(3_000)
      if (!hasComposer(term)) return null

      const lines = input.text.split('\n')
      const before = lines.slice(Math.max(0, input.line - 80), input.line).join('\n') +
        '\n' + (lines[input.line] ?? '').slice(0, input.character)
      const after = (lines[input.line] ?? '').slice(input.character) +
        '\n' + lines.slice(input.line + 1, input.line + 40).join('\n')
      const prompt = [
        'Do not use tools or inspect files. This is a pure autocomplete request.',
        'Reply with exactly one block: <<<NOVA_COMPLETION then the exact code to insert at CURSOR then NOVA_COMPLETION>>>.',
        'PREFIX:', before.slice(-4_000), 'CURSOR', 'SUFFIX:', after.slice(0, 2_000)
      ].join(' ').replace(/\r/g, '')
      pty.write(prompt)
      await delay(220)
      pty.write('\r')
      await delay(1_200)
      // Some TUI states swallow the first Enter while the composer is settling.
      if (screenLines(term).some((line) => /│.*<<<NOVA_COMPLETION/.test(line))) {
        pty.write('\r')
      }

      const start = Date.now()
      while (Date.now() - start < 30_000) {
        await delay(400)
        const joined = screenLines(term).join('\n')
        const open = joined.indexOf('<<<NOVA_COMPLETION')
        const close = joined.lastIndexOf('NOVA_COMPLETION>>>')
        const match = /<<<NOVA_COMPLETION[^\S\r\n]*\r?\n([\s\S]*?)\r?\n[^\S\r\n]*NOVA_COMPLETION>>>/.exec(joined)
        const raw = match?.[1] ?? (open >= 0 && close > open
          ? joined.slice(open + '<<<NOVA_COMPLETION'.length, close)
          : '')
        if (raw) {
          const code = raw
            .replace(/^\s*\r?\n/, '')
            .split('\n')
            .map((line) => line.replace(/^ {0,2}/, ''))
            .join('\n')
            .replace(/\s+$/, '')
          if (code.trim()) return { insertText: code }
        }
      }
      if (process.env.NOVA_FREEBUFF_DEBUG) {
        console.error('FREEBUFF_SCREEN\n' + screenLines(term).join('\n'))
      }
      return null
    } catch {
      return null
    } finally {
      try {
        if (pty) {
          try { process.kill(-pty.pid, 'SIGKILL') } catch { /* no process group */ }
          pty.kill()
        }
      } catch { /* already gone */ }
      term.dispose()
      this.active = false
    }
  }
}
