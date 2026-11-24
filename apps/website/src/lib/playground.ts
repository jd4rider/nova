import { basicSetup } from 'codemirror'
import { EditorState } from '@codemirror/state'
import { EditorView, keymap } from '@codemirror/view'
import {
  HighlightStyle,
  StreamLanguage,
  syntaxHighlighting
} from '@codemirror/language'
import { tags } from '@lezer/highlight'

interface PlaygroundExample {
  id: string
  name: string
  description: string
  source: string
}

interface Point {
  line: number
  character: number
}

interface DiagnosticLabel {
  message: string
  start: Point
  end: Point
}

interface Diagnostic {
  code: string
  severity: 'error' | 'warning' | 'note'
  message: string
  start: Point
  end: Point
  notes: string[]
  labels: DiagnosticLabel[]
}

interface CheckResponse {
  ok: boolean
  diagnostics: Diagnostic[]
  stats: {
    lines: number
    bytes: number
    functions: number
    structs: number
    errors: number
    warnings: number
  }
  version: string
}

interface NovaFmtExports {
  memory: WebAssembly.Memory
  nova_input_ptr: () => number | bigint
  nova_input_set: (len: number | bigint) => void
  nova_output_ptr: () => number | bigint
  nova_output_len: () => number | bigint
  nova_reset_output: () => void
  main: () => void
}

interface NovaWasmExports {
  memory: WebAssembly.Memory
  nova_alloc: (len: number) => number
  nova_free: (ptr: number, len: number) => void
  nova_check: (ptr: number, len: number) => number
  nova_result_len: () => number
}

type NovaStreamState = {
  blockComment: boolean
}

const keywords = new Set([
  'fn',
  'let',
  'var',
  'return',
  'if',
  'else',
  'while',
  'for',
  'in',
  'match',
  'break',
  'continue',
  'struct',
  'enum',
  'impl',
  'trait',
  'import',
  'as',
  'pub',
  'defer',
  'unsafe',
  'extern',
  'async',
  'await'
])

const primitiveTypes = new Set([
  'Bool',
  'Int',
  'Float',
  'Char',
  'String',
  'Unit',
  'List',
  'Option',
  'Result'
])

const novaStream = StreamLanguage.define<NovaStreamState>({
  startState: () => ({ blockComment: false }),
  token(stream, state) {
    if (state.blockComment) {
      if (stream.skipTo('*/')) {
        stream.match('*/')
        state.blockComment = false
      } else {
        stream.skipToEnd()
      }
      return 'comment'
    }
    if (stream.eatSpace()) return null
    if (stream.match('//')) {
      stream.skipToEnd()
      return 'comment'
    }
    if (stream.match('/*')) {
      state.blockComment = true
      if (stream.skipTo('*/')) {
        stream.match('*/')
        state.blockComment = false
      } else {
        stream.skipToEnd()
      }
      return 'comment'
    }
    if (stream.match(/^"(?:[^"\\]|\\.)*"?/)) return 'string'
    if (stream.match(/^'(?:[^'\\]|\\.)*'?/)) return 'string'
    if (stream.match(/^0[xX][0-9a-fA-F_]+/)) return 'number'
    if (stream.match(/^\d[\d_]*(?:\.\d[\d_]*)?(?:[eE][+-]?\d+)?/)) return 'number'
    if (stream.match(/^[A-Za-z_][A-Za-z0-9_]*/)) {
      const word = stream.current()
      if (keywords.has(word)) return 'keyword'
      if (primitiveTypes.has(word)) return 'typeName'
      if (word === 'true' || word === 'false' || word === 'None') return 'bool'
      if (/^[A-Z]/.test(word)) return 'typeName'
      return 'variableName'
    }
    if (
      stream.match(
        /^(?:==|!=|<=|>=|&&|\|\||\.\.=?|->|=>|[+\-*/%=<>!?])/
      )
    ) {
      return 'operator'
    }
    if (stream.match(/^[{}\[\]();,.:@]/)) return 'punctuation'
    stream.next()
    return null
  }
})

const novaHighlight = HighlightStyle.define([
  { tag: tags.comment, color: '#928374', fontStyle: 'italic' },
  { tag: tags.keyword, color: '#fb4934' },
  { tag: tags.string, color: '#b8bb26' },
  { tag: tags.number, color: '#d3869b' },
  { tag: tags.bool, color: '#fabd2f' },
  { tag: tags.typeName, color: '#83a598' },
  { tag: tags.variableName, color: '#ebdbb2' },
  { tag: tags.operator, color: '#fe8019' },
  { tag: tags.punctuation, color: '#a89984' }
])

const novaTheme = EditorView.theme(
  {
    '&': {
      backgroundColor: '#1d2021',
      color: '#ebdbb2',
      fontSize: '13px'
    },
    '.cm-content': {
      fontFamily:
        "'JetBrains Mono', 'SFMono-Regular', Menlo, Consolas, monospace",
      padding: '18px 0'
    },
    '.cm-gutters': {
      backgroundColor: '#1d2021',
      color: '#7c6f64',
      border: 'none'
    },
    '.cm-activeLine': { backgroundColor: '#282828' },
    '.cm-activeLineGutter': { backgroundColor: '#282828', color: '#fabd2f' },
    '.cm-selectionBackground, ::selection': { backgroundColor: '#504945' },
    '.cm-cursor': { borderLeftColor: '#fabd2f' },
    '.cm-scroller': { overflow: 'auto', maxHeight: '500px' },
    '&.cm-focused': { outline: 'none' },
    '.cm-tooltip': {
      backgroundColor: '#282828',
      color: '#ebdbb2',
      border: '1px solid #504945'
    }
  },
  { dark: true }
)

function byId<T extends HTMLElement>(id: string): T {
  const element = document.getElementById(id)
  if (!element) throw new Error(`Missing playground element #${id}`)
  return element as T
}

function encodeSource(source: string): string {
  const bytes = new TextEncoder().encode(source)
  let binary = ''
  for (const byte of bytes) binary += String.fromCharCode(byte)
  return btoa(binary)
}

function decodeSource(value: string): string | null {
  try {
    const binary = atob(value)
    const bytes = Uint8Array.from(binary, (character) =>
      character.charCodeAt(0)
    )
    return new TextDecoder().decode(bytes)
  } catch {
    return null
  }
}

export function mountPlayground(): void {
  const root = byId<HTMLElement>('playground')
  const examples = JSON.parse(root.dataset.examples ?? '[]') as PlaygroundExample[]
  if (!examples.length) throw new Error('No playground examples were provided')

  const base = (root.dataset.base ?? '/').replace(/\/?$/, '/')
  const wasmUrl = `${base}playground/nova.wasm`
  const fmtUrl = `${base}playground/nova-fmt.wasm`
  const initial = readInitialSource(examples)
  const tabs = Array.from(
    document.querySelectorAll<HTMLButtonElement>('[data-example]')
  )
  const runButton = byId<HTMLButtonElement>('playground-run')
  const formatButton = byId<HTMLButtonElement>('playground-format')
  const shareButton = byId<HTMLButtonElement>('playground-share')
  const resetButton = byId<HTMLButtonElement>('playground-reset')
  const status = byId<HTMLElement>('playground-wasm-status')
  const resultState = byId<HTMLElement>('playground-result-state')
  const diagnosticsNode = byId<HTMLElement>('playground-diagnostics')
  const statsNode = byId<HTMLElement>('playground-stats')
  const editorNode = byId<HTMLElement>('playground-editor')

  let wasm: NovaWasmExports | null = null
  let lastResult: CheckResponse | null = null
  let formatter: NovaFmtExports | null = null
  let checkSequence = 0
  let debounce: number | undefined

  const state = EditorState.create({
    doc: initial,
    extensions: [
      basicSetup,
      novaStream,
      novaTheme,
      syntaxHighlighting(novaHighlight),
      EditorView.lineWrapping,
      keymap.of([
        {
          key: 'Mod-Enter',
          run: () => {
            void check()
            return true
          }
        }
      ]),
      EditorView.updateListener.of((update) => {
        if (update.docChanged) scheduleCheck()
      })
    ]
  })
  const view = new EditorView({ state, parent: editorNode })

  function setStatus(message: string, kind: 'loading' | 'ready' | 'error' = 'loading') {
    status.textContent = message
    status.dataset.kind = kind
  }

  // The formatter is the Nova formatter compiled to wasm: the playground hands
  // it the buffer, and it returns text through its output buffer.
  function formatSource(source: string): string {
    if (!formatter) throw new Error('The Nova formatter is not ready')
    const bytes = new TextEncoder().encode(source)
    if (bytes.length > 1024 * 1024) throw new Error('Source is too large to format')
    const ptr = Number(formatter.nova_input_ptr())
    new Uint8Array(formatter.memory.buffer, ptr, bytes.length).set(bytes)
    formatter.nova_input_set(BigInt(bytes.length))
    formatter.nova_reset_output()
    formatter.main()
    const start = Number(formatter.nova_output_ptr())
    const length = Number(formatter.nova_output_len())
    return new TextDecoder().decode(new Uint8Array(formatter.memory.buffer, start, length))
  }

  async function loadFormatter(url: string): Promise<NovaFmtExports> {
    const response = await fetch(url)
    if (!response.ok) throw new Error(`${response.status} ${response.statusText} for ${url}`)
    const bytes = await response.arrayBuffer()
    const { instance } = await WebAssembly.instantiate(bytes, {})
    const exports = instance.exports as unknown as NovaFmtExports
    if (
      !(exports.memory instanceof WebAssembly.Memory) ||
      typeof exports.main !== 'function' ||
      typeof exports.nova_input_ptr !== 'function' ||
      typeof exports.nova_input_set !== 'function' ||
      typeof exports.nova_output_ptr !== 'function' ||
      typeof exports.nova_output_len !== 'function' ||
      typeof exports.nova_reset_output !== 'function'
    ) {
      throw new Error('The formatter WASM module has an unexpected ABI')
    }
    return exports
  }

  function setSource(source: string): void {
    view.dispatch({
      changes: { from: 0, to: view.state.doc.length, insert: source }
    })
    try {
      localStorage.setItem('nova-playground-source', source)
    } catch {
      // Private browsing can reject storage; the playground still works.
    }
  }

  function setActiveExample(id: string): void {
    for (const tab of tabs) {
      const active = tab.dataset.example === id
      tab.classList.toggle('active', active)
      tab.setAttribute('aria-selected', active ? 'true' : 'false')
    }
  }

  function scheduleCheck(): void {
    if (!wasm) return
    if (debounce !== undefined) window.clearTimeout(debounce)
    debounce = window.setTimeout(() => {
      debounce = undefined
      void check()
    }, 360)
  }

  function checkSource(source: string): CheckResponse {
    if (!wasm) throw new Error('The WebAssembly compiler is not ready')
    const input = new TextEncoder().encode(source)
    const allocation = Math.max(input.length, 1)
    const ptr = wasm.nova_alloc(allocation)
    if (!ptr) throw new Error('Nova could not allocate source memory')
    if (input.length) {
      new Uint8Array(wasm.memory.buffer, ptr, input.length).set(input)
    }
    try {
      const resultPtr = wasm.nova_check(ptr, input.length)
      const resultLen = wasm.nova_result_len()
      const json = new TextDecoder().decode(
        new Uint8Array(wasm.memory.buffer, resultPtr, resultLen)
      )
      return JSON.parse(json) as CheckResponse
    } finally {
      wasm.nova_free(ptr, allocation)
    }
  }

  async function check(): Promise<void> {
    if (!wasm) return
    const sequence = ++checkSequence
    const source = view.state.doc.toString()
    const started = performance.now()
    resultState.textContent = 'Checking…'
    try {
      const result = checkSource(source)
      if (sequence !== checkSequence) return
      const elapsed = performance.now() - started
      lastResult = result
      try {
        localStorage.setItem('nova-playground-source', source)
      } catch {
        // Storage is optional.
      }
      renderResult(result, elapsed)
    } catch (error) {
      if (sequence !== checkSequence) return
      renderFailure(error instanceof Error ? error.message : String(error))
    }
  }

  function renderResult(result: CheckResponse, elapsed: number): void {
    lastResult = result
    formatButton.disabled = false
    shareButton.disabled = false
    const errors = result.diagnostics.filter(
      (diagnostic) => diagnostic.severity === 'error'
    ).length
    resultState.textContent = result.ok ? 'All checks pass' : `${errors} error${errors === 1 ? '' : 's'}`
    resultState.dataset.kind = result.ok ? 'ok' : 'error'

    diagnosticsNode.replaceChildren()
    if (!result.diagnostics.length) {
      const empty = document.createElement('div')
      empty.className = 'playground-empty'
      const mark = document.createElement('span')
      mark.className = 'playground-empty-mark'
      mark.textContent = '✓'
      const text = document.createElement('div')
      const title = document.createElement('strong')
      title.textContent = 'No diagnostics'
      const detail = document.createElement('span')
      detail.textContent = 'The parser and type checker are happy.'
      text.append(title, detail)
      empty.append(mark, text)
      diagnosticsNode.append(empty)
    } else {
      for (const diagnostic of result.diagnostics) {
        diagnosticsNode.append(renderDiagnostic(diagnostic))
      }
    }

    const parts = [
      `${result.stats.functions} function${result.stats.functions === 1 ? '' : 's'}`,
      `${result.stats.structs} struct${result.stats.structs === 1 ? '' : 's'}`,
      `${result.stats.lines} lines`,
      `${elapsed.toFixed(1)} ms`
    ]
    statsNode.textContent = parts.join(' · ')
  }

  function renderDiagnostic(diagnostic: Diagnostic): HTMLButtonElement {
    const item = document.createElement('button')
    item.type = 'button'
    item.className = `playground-diagnostic ${diagnostic.severity}`

    const head = document.createElement('span')
    head.className = 'playground-diagnostic-head'
    const code = document.createElement('span')
    code.className = 'playground-diagnostic-code'
    code.textContent = diagnostic.code
    const location = document.createElement('span')
    location.className = 'playground-diagnostic-location'
    location.textContent = `${diagnostic.start.line + 1}:${diagnostic.start.character + 1}`
    const severity = document.createElement('span')
    severity.className = 'playground-diagnostic-severity'
    severity.textContent = diagnostic.severity
    head.append(code, severity, location)

    const message = document.createElement('span')
    message.className = 'playground-diagnostic-message'
    message.textContent = diagnostic.message
    item.append(head, message)

    for (const note of [...diagnostic.notes, ...diagnostic.labels.map((label) => label.message)]) {
      const noteNode = document.createElement('span')
      noteNode.className = 'playground-diagnostic-note'
      noteNode.textContent = note
      item.append(noteNode)
    }

    item.addEventListener('click', () => {
      focusPosition(diagnostic.start.line, diagnostic.start.character)
    })
    return item
  }

  function focusPosition(line: number, character: number): void {
    const lineNumber = Math.min(line + 1, view.state.doc.lines)
    const lineInfo = view.state.doc.line(lineNumber)
    const position = lineInfo.from + Math.min(character, lineInfo.length)
    view.dispatch({
      selection: { anchor: position },
      scrollIntoView: true
    })
    view.focus()
  }

  function renderFailure(message: string): void {
    resultState.textContent = 'Check failed'
    resultState.dataset.kind = 'error'
    diagnosticsNode.replaceChildren()
    const error = document.createElement('div')
    error.className = 'playground-empty error'
    const title = document.createElement('strong')
    title.textContent = 'Could not load the Nova front end'
    const detail = document.createElement('span')
    detail.textContent = message
    error.append(title, detail)
    diagnosticsNode.append(error)
    statsNode.textContent = ''
    lastResult = null
    formatButton.disabled = true
    shareButton.disabled = true
  }

  async function loadWasm(): Promise<void> {
    setStatus('Loading the Nova front end…')
    runButton.disabled = true
    formatButton.disabled = true
    shareButton.disabled = true
    const response = await fetch(wasmUrl)
    if (!response.ok) throw new Error(`${response.status} ${response.statusText} for ${wasmUrl}`)
    const bytes = await response.arrayBuffer()
    const { instance } = await WebAssembly.instantiate(bytes, {})
    const candidate = instance.exports as unknown as NovaWasmExports
    if (
      !(candidate.memory instanceof WebAssembly.Memory) ||
      typeof candidate.nova_alloc !== 'function' ||
      typeof candidate.nova_free !== 'function' ||
      typeof candidate.nova_check !== 'function' ||
      typeof candidate.nova_result_len !== 'function'
    ) {
      throw new Error('The WASM module has an unexpected ABI')
    }
    formatter = await loadFormatter(fmtUrl)
    wasm = candidate
    setStatus('Nova front end ready', 'ready')
    runButton.disabled = false
    void check()
  }

  async function share(): Promise<void> {
    const source = view.state.doc.toString()
    const url = new URL(window.location.href)
    url.hash = new URLSearchParams({ code: encodeSource(source) }).toString()
    try {
      await navigator.clipboard.writeText(url.toString())
      setStatus('Share link copied', 'ready')
    } catch {
      window.prompt('Copy this playground link:', url.toString())
    }
    window.history.replaceState(null, '', url)
  }

  function reset(): void {
    setSource(examples[0].source)
    setActiveExample(examples[0].id)
    window.history.replaceState(null, '', window.location.pathname)
    void check()
  }

  function loadHash(): void {
    const value = new URLSearchParams(window.location.hash.slice(1)).get('code')
    if (!value) return
    const source = decodeSource(value)
    if (!source) return
    setSource(source)
    void check()
  }

  tabs.forEach((tab) => {
    tab.addEventListener('click', () => {
      const example = examples.find((item) => item.id === tab.dataset.example)
      if (!example) return
      setSource(example.source)
      setActiveExample(example.id)
      window.history.replaceState(null, '', window.location.pathname)
      void check()
    })
  })
  runButton.addEventListener('click', () => void check())
  formatButton.addEventListener('click', () => {
    if (!lastResult) return
    setSource(formatSource(view.state.doc.toString()))
    void check()
  })
  shareButton.addEventListener('click', () => void share())
  resetButton.addEventListener('click', reset)
  window.addEventListener('hashchange', loadHash)

  setActiveExample(examples[0].id)
  loadWasm().catch((error: unknown) => {
    const message = error instanceof Error ? error.message : String(error)
    setStatus('Front end unavailable', 'error')
    renderFailure(message)
  })
}

function readInitialSource(examples: PlaygroundExample[]): string {
  const value = new URLSearchParams(window.location.hash.slice(1)).get('code')
  if (value) {
    const decoded = decodeSource(value)
    if (decoded) return decoded
  }
  try {
    const stored = localStorage.getItem('nova-playground-source')
    if (stored) return stored
  } catch {
    // Storage is optional.
  }
  return examples[0].source
}
