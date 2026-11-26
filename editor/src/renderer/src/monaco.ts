import * as monaco from 'monaco-editor'
import EditorWorker from 'monaco-editor/esm/vs/editor/editor.worker?worker'

;(self as any).MonacoEnvironment = {
  getWorker: () => new EditorWorker()
}

monaco.languages.register({ id: 'nova', extensions: ['.nova'] })

monaco.languages.setMonarchTokensProvider('nova', {
  defaultToken: '',
  tokenPostfix: '.nova',
  keywords: [
    'fn', 'let', 'var', 'return', 'if', 'else', 'while', 'for', 'in', 'match',
    'struct', 'enum', 'impl', 'trait', 'import', 'as', 'pub', 'true', 'false',
    'break', 'continue', 'defer', 'unsafe', 'extern', 'async', 'await'
  ],
  typeKeywords: ['Bool', 'Int', 'Float', 'Char', 'String', 'Unit', 'List', 'Option', 'Result'],
  operators: ['=', '==', '!=', '<', '<=', '>', '>=', '&&', '||', '+', '-', '*', '/', '%', '+=', '-=', '*=', '/=', '%=', '->', '=>', '..', '..=', '?', '!'],
  symbols: /[=><!~?:&|+\-*/^%]+/,
  escapes: /\\(?:[abfnrtv\\"']|x[0-9A-Fa-f]{2}|u\{[0-9A-Fa-f]+\})/,
  tokenizer: {
    root: [
      [/[A-Z][A-Za-z0-9_]*/, { cases: { '@typeKeywords': 'type.identifier', '@default': 'type.identifier' } }],
      [/[a-z_][A-Za-z0-9_]*/, { cases: { '@keywords': 'keyword', '@default': 'identifier' } }],
      { include: '@whitespace' },
      [/@[A-Za-z_][A-Za-z0-9_]*/, 'annotation'],
      [/[{}()[\]]/, '@brackets'],
      [/[<>](?!@symbols)/, '@brackets'],
      [/[0-9][0-9_]*(\.[0-9][0-9_]*)?([eE][+-]?[0-9]+)?/, 'number'],
      [/0[xX][0-9a-fA-F_]+/, 'number.hex'],
      [/"([^"\\]|\\.)*$/, 'string.invalid'],
      [/"/, { token: 'string.quote', bracket: '@open', next: '@string' }],
      [/'([^'\\]|\\.)'/, 'string.char'],
      [/\/\/.*$/, 'comment'],
      [/\/\*/, 'comment', '@comment'],
      [/@symbols/, { cases: { '@operators': 'operator', '@default': '' } }],
      [/[,.;:]/, 'delimiter']
    ],
    comment: [
      [/[^*/]+/, 'comment'],
      [/\*\//, 'comment', '@pop'],
      [/[*/]/, 'comment']
    ],
    string: [
      [/\$\{/, { token: 'delimiter.bracket', next: '@interpolation' }],
      [/@escapes/, 'string.escape'],
      [/\\./, 'string.escape.invalid'],
      [/[^\\$"]+/, 'string'],
      [/"/, { token: 'string.quote', bracket: '@close', next: '@pop' }]
    ],
    interpolation: [
      [/\}/, { token: 'delimiter.bracket', next: '@pop' }],
      { include: 'root' }
    ],
    whitespace: [
      [/[ \t\r\n]+/, ''],
      [/\/\/.*$/, 'comment'],
      [/\/\*/, 'comment', '@comment']
    ]
  }
})

monaco.languages.setLanguageConfiguration('nova', {
  comments: { lineComment: '//', blockComment: ['/*', '*/'] },
  brackets: [
    ['{', '}'],
    ['(', ')'],
    ['[', ']']
  ],
  autoClosingPairs: [
    { open: '{', close: '}' },
    { open: '(', close: ')' },
    { open: '[', close: ']' },
    { open: '"', close: '"' },
    { open: "'", close: "'" }
  ],
  surroundingPairs: [
    { open: '{', close: '}' },
    { open: '(', close: ')' },
    { open: '[', close: ']' },
    { open: '"', close: '"' },
    { open: "'", close: "'" }
  ],
  indentationRules: {
    increaseIndentPattern: /\{[^}"']*$/,
    decreaseIndentPattern: /^\s*\}/
  }
})

const darkRules: monaco.editor.ITokenThemeRule[] = [
  { token: 'keyword', foreground: 'FB4934' },
  { token: 'keyword.control', foreground: 'FB4934' },
  { token: 'type.identifier', foreground: 'FABD2F' },
  { token: 'identifier', foreground: 'EBDAB2' },
  { token: 'number', foreground: 'D3869B' },
  { token: 'number.hex', foreground: 'D3869B' },
  { token: 'string', foreground: 'B8BB26' },
  { token: 'string.quote', foreground: 'B8BB26' },
  { token: 'string.char', foreground: 'B8BB26' },
  { token: 'string.escape', foreground: 'FE8019' },
  { token: 'comment', foreground: '928374', fontStyle: 'italic' },
  { token: 'operator', foreground: '8EC07C' },
  { token: 'delimiter', foreground: 'EBDBB2' },
  { token: 'annotation', foreground: '83A598' },
  { token: 'function', foreground: '83A598' }
]

monaco.editor.defineTheme('gruvbox-dark', {
  base: 'vs-dark',
  inherit: true,
  rules: darkRules,
  colors: {
    'editor.background': '#282828',
    'editor.foreground': '#ebdbb2',
    'editorLineNumber.foreground': '#665c54',
    'editorLineNumber.activeForeground': '#fabd2f',
    'editorCursor.foreground': '#fe8019',
    'editor.selectionBackground': '#504945',
    'editor.inactiveSelectionBackground': '#3c3836',
    'editor.lineHighlightBackground': '#32302f',
    'editorIndentGuide.background1': '#3c3836',
    'editorIndentGuide.activeBackground1': '#665c54',
    'editorWhitespace.foreground': '#504945',
    'editorBracketMatch.background': '#504945',
    'editorBracketMatch.border': '#83a598',
    'editorWidget.background': '#32302f',
    'editorWidget.border': '#504945',
    'editorSuggestWidget.background': '#32302f',
    'editorSuggestWidget.selectedBackground': '#504945',
    'editor.findMatchBackground': '#fabd2f44',
    'editor.findMatchHighlightBackground': '#fabd2f22',
    'editorError.foreground': '#fb4934',
    'editorWarning.foreground': '#fabd2f',
    'editorGutter.background': '#282828',
    'minimap.background': '#282828',
    'scrollbarSlider.background': '#50494580',
    'scrollbarSlider.hoverBackground': '#665c5490'
  }
})

monaco.editor.defineTheme('gruvbox-light', {
  base: 'vs',
  inherit: true,
  rules: darkRules.map((rule) => ({ ...rule, foreground: rule.foreground === 'EBDAB2' ? '3C3836' : rule.foreground })),
  colors: {
    'editor.background': '#fbf1c7',
    'editor.foreground': '#3c3836',
    'editorLineNumber.foreground': '#a89984',
    'editorCursor.foreground': '#af3a03',
    'editor.selectionBackground': '#ebdbb2',
    'editor.lineHighlightBackground': '#f2e5bc',
    'editorWidget.background': '#f2e5bc',
    'editorError.foreground': '#9d0006'
  }
})

monaco.editor.defineTheme('gruvbox-hc', {
  base: 'hc-black',
  inherit: true,
  rules: darkRules,
  colors: {
    'editor.background': '#1d2021',
    'editor.foreground': '#fbf1c7',
    'editorLineNumber.foreground': '#a89984',
    'editorCursor.foreground': '#fe8019',
    'editor.selectionBackground': '#665c54'
  }
})

type Request = (method: string, params: unknown) => Promise<unknown>

function kindFromLsp(kind: number | undefined): monaco.languages.CompletionItemKind {
  switch (kind) {
    case 2: return monaco.languages.CompletionItemKind.Method
    case 3: return monaco.languages.CompletionItemKind.Function
    case 6: return monaco.languages.CompletionItemKind.Variable
    case 7: return monaco.languages.CompletionItemKind.Class
    case 8: return monaco.languages.CompletionItemKind.Interface
    case 9: return monaco.languages.CompletionItemKind.Module
    case 10: return monaco.languages.CompletionItemKind.Enum
    case 13: return monaco.languages.CompletionItemKind.Enum
    case 14: return monaco.languages.CompletionItemKind.Keyword
    case 15: return monaco.languages.CompletionItemKind.Snippet
    case 22: return monaco.languages.CompletionItemKind.Struct
    case 23: return monaco.languages.CompletionItemKind.TypeParameter
    default: return monaco.languages.CompletionItemKind.Text
  }
}

function rangeFromLsp(range: any): monaco.IRange {
  return {
    startLineNumber: range.start.line + 1,
    startColumn: range.start.character + 1,
    endLineNumber: range.end.line + 1,
    endColumn: range.end.character + 1
  }
}

export function registerLspProviders(request: Request): void {
  monaco.languages.registerCompletionItemProvider('nova', {
    triggerCharacters: ['.', '$'],
    provideCompletionItems: async (model, position) => {
      const result: any = await request('textDocument/completion', {
        textDocument: { uri: model.uri.toString() },
        position: { line: position.lineNumber - 1, character: position.column - 1 }
      }).catch(() => null)
      if (!result?.items) return { suggestions: [] }
      const word = model.getWordUntilPosition(position)
      const range = new monaco.Range(position.lineNumber, word.startColumn, position.lineNumber, word.endColumn)
      const suggestions = result.items.slice(0, 200).map((item: any) => ({
        label: item.label,
        kind: kindFromLsp(item.kind),
        detail: item.detail,
        documentation: item.documentation?.value ?? item.documentation,
        insertText: item.insertText ?? item.label,
        insertTextRules: item.insertTextFormat === 2 ? monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet : undefined,
        sortText: item.sortText,
        range
      }))
      return { suggestions }
    }
  })

  monaco.languages.registerHoverProvider('nova', {
    provideHover: async (model, position) => {
      const result: any = await request('textDocument/hover', {
        textDocument: { uri: model.uri.toString() },
        position: { line: position.lineNumber - 1, character: position.column - 1 }
      }).catch(() => null)
      if (!result?.contents) return null
      return {
        range: result.range ? rangeFromLsp(result.range) : undefined,
        contents: [{ value: result.contents.value ?? String(result.contents) }]
      }
    }
  })

  monaco.languages.registerDefinitionProvider('nova', {
    provideDefinition: async (model, position) => {
      const result: any = await request('textDocument/definition', {
        textDocument: { uri: model.uri.toString() },
        position: { line: position.lineNumber - 1, character: position.column - 1 }
      }).catch(() => null)
      if (!result) return null
      return {
        uri: monaco.Uri.parse(result.uri),
        range: rangeFromLsp(result.range)
      }
    }
  })

  monaco.languages.registerDocumentFormattingEditProvider('nova', {
    provideDocumentFormattingEdits: async (model) => {
      const result: any = await request('textDocument/formatting', {
        textDocument: { uri: model.uri.toString() },
        options: { tabSize: 4, insertSpaces: true }
      }).catch(() => null)
      if (!Array.isArray(result)) return []
      return result.map((edit: any) => ({ range: rangeFromLsp(edit.range), text: edit.newText }))
    }
  })
}

let copilotCompletionRegistered = false

/**
 * Ghost text from the Copilot language server. The request is a file URI and a
 * line/character position; the main process owns the authenticated LSP.
 */
export function registerCopilotCompletion(): void {
  if (copilotCompletionRegistered) return
  copilotCompletionRegistered = true
  monaco.languages.registerInlineCompletionsProvider('nova', {
    provideInlineCompletions: async (model, position, _context, token) => {
      if (model.uri.scheme !== 'file') return { items: [] }
      const input = {
        uri: model.uri.toString(),
        version: model.getVersionId(),
        text: model.getValue(),
        line: position.lineNumber - 1,
        character: position.column - 1
      }
      const result = await window.nova.copilot.complete(input).catch(() => null)
      if (token.isCancellationRequested || !result?.insertText) return { items: [] }
      const range = result.range
        ? {
            startLineNumber: result.range.start.line + 1,
            startColumn: result.range.start.character + 1,
            endLineNumber: result.range.end.line + 1,
            endColumn: result.range.end.character + 1
          }
        : new monaco.Range(position.lineNumber, position.column, position.lineNumber, position.column)
      return { items: [{ insertText: result.insertText, range }] }
    },
    freeInlineCompletions: () => undefined
  })
}

export { monaco }
