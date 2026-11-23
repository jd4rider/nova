/// <reference types="vite/client" />

import type { NovaApi } from '../../shared/types'

declare global {
  interface Window {
    nova: NovaApi
  }
}

declare module 'monaco-vim' {
  import type * as monaco from 'monaco-editor'
  export function initVimMode(editor: monaco.editor.IStandaloneCodeEditor, statusNode: HTMLElement): { dispose: () => void }
  export const Vim: {
    defineEx(name: string, shortName: string, callback: (editor: monaco.editor.IStandaloneCodeEditor, params: string) => void): void
    map(lhs: string, rhs: string): void
    mapCommand(line: string, shortName?: string): void
  }
}

export {}
