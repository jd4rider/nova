/**
 * FreeBuff 0.2.36 has no headless mode. Its TUI is captured with a headless
 * terminal, and this module turns screen rows into a chat transcript. The
 * layout is version-specific and deliberately heuristic; callers must label it
 * as a scrape.
 */

const CHROME = [
  /^\s*[╭╰╮╯┌└┐┘│─═]+\s*$/,
  /^\s*[│╭╰╮╯]/,
  /█/,
  /You started Freebuff in a subdirectory/,
  /Switch to git root/,
  /May use data for AI training/,
  /Your first message starts the session/,
  /day streak/,
  /Refer friends/,
  /Copy invite link/,
  /Learn More/,
  /Your Freebuff chats, in your pocket/,
  /Free on the App Store/,
  /Get the mobile app/,
  /Ad \[Report/,
  /Enter a coding task/,
  /for history/,
  /\? for help/,
  /\/model to change/,
  /Chat: New chat/,
  /^\s*[▍❯>]\s*$/,
  /^\s*[⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏]/,
  /^\s*(esc to interrupt|thinking|working)[.…]*$/i,
  /^\s*[•·]\s/, 
  /^\s*working\.\.\./i,
  /^\s*[─]{3,}/, 
  /^\s*Stop typing prompts\. Start talking\./i,
  /^\s*Give your coding agent the full context/i,
  /^\s*DeepSeek .* · /i
]

function isChrome(line: string): boolean {
  return !line.trim() || CHROME.some((pattern) => pattern.test(line))
}

function compress(value: string): string {
  return value.replace(/\s+/g, ' ').trim()
}

/** Return the rows of a headless terminal, excluding blank scrollback padding. */
export function screenLines(term: { buffer: { active: { length: number; getLine(index: number): { translateToString(trimRight?: boolean): string } | undefined } } }): string[] {
  const buffer = term.buffer.active
  const lines: string[] = []
  for (let index = 0; index < buffer.length; index++) {
    lines.push(buffer.getLine(index)?.translateToString(true) ?? '')
  }
  while (lines.length > 0 && !lines[lines.length - 1].trim()) lines.pop()
  return lines
}

/** Locate the composer and return the transcript rows above it. */
export function transcriptLines(lines: string[]): string[] {
  let prompt = -1
  for (let index = lines.length - 1; index >= 0; index--) {
    if (/Enter a coding task|^\s*[│]?\s*▍/.test(lines[index])) {
      prompt = index
      break
    }
  }
  if (prompt < 0) return lines
  // The composer's own top border is within a few rows of its placeholder.
  // Do not walk back to an unrelated box in the transcript.
  let top = prompt
  for (let index = prompt - 1; index >= Math.max(0, prompt - 5); index--) {
    if (/^\s*[╭┌]/.test(lines[index])) {
      top = index
      break
    }
  }
  return lines.slice(0, top)
}

function contentLines(lines: string[]): string[] {
  const kept: string[] = []
  let fenced = false
  for (const line of lines) {
    if (/^\s*```/.test(line)) {
      fenced = !fenced
      kept.push(line)
      continue
    }
    if (!fenced && isChrome(line)) continue
    kept.push(line)
  }
  return kept
}

function trimLines(lines: string[]): string {
  const text = lines.join('\n').replace(/[ \t]+$/gm, '')
  return text.replace(/\n{3,}/g, '\n\n').trim()
}

/**
 * Extract the assistant response after the known user prompt. If the prompt
 * wrapped or the TUI redrew, fall back to rows added after the pre-send
 * baseline.
 */
export function assistantText(lines: string[], prompt: string, baseline: string[] = []): string {
  const transcript = transcriptLines(lines)
  const needle = compress(prompt).slice(0, 28)
  let start = -1
  if (needle) {
    for (let index = transcript.length - 1; index >= 0; index--) {
      if (compress(transcript[index]).includes(needle)) {
        start = index + 1
        break
      }
    }
  }

  if (start >= 0) {
    const after = transcript.slice(start)
    // Drop the remainder of a wrapped prompt and the next blank separator.
    while (after.length > 0 && !after[0].trim()) after.shift()
    return trimLines(contentLines(after))
  }

  if (baseline.length > 0) {
    let common = 0
    while (common < baseline.length && common < transcript.length && baseline[common] === transcript[common]) common++
    return trimLines(contentLines(transcript.slice(common)))
  }

  return trimLines(contentLines(transcript))
}
