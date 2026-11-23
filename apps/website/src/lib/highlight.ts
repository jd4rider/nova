import { createHighlighter, type Highlighter } from 'shiki'
import grammar from '../../../../grammar/nova.tmLanguage.json'


let pending: Promise<Highlighter> | undefined

function highlighter(): Promise<Highlighter> {
  pending ??= createHighlighter({
    themes: ['gruvbox-dark-hard', 'gruvbox-light-medium'],
    langs: [{ ...grammar, name: 'nova' }]
  })
  return pending
}

export async function highlightNova(code: string, light = false): Promise<string> {
  const shiki = await highlighter()
  return shiki.codeToHtml(code, {
    lang: 'nova',
    theme: light ? 'gruvbox-light-medium' : 'gruvbox-dark-hard'
  })
}
