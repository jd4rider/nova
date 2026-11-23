declare module 'monaco-vim' {
  export interface VimAdapter {
    dispose(): void
  }
  export const Vim: {
    defineEx(name: string, prefix: string, handler: (cm: unknown, params: string) => void): void
  }
  export function initVimMode(editor: unknown, statusNode: HTMLElement): VimAdapter
}
