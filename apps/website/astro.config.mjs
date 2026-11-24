import { defineConfig } from 'astro/config'
import starlight from '@astrojs/starlight'
import sitemap from '@astrojs/sitemap'
import { readFileSync } from 'node:fs'

const rawBase = process.env.BASE_PATH ?? '/'
const base = rawBase === '/' ? '/' : `/${rawBase.replace(/^\/+|\/+$/g, '')}`
const site = process.env.SITE_URL ?? 'http://localhost:4321'
const github = process.env.PUBLIC_GITHUB_URL ?? 'https://github.com/jd4rider/nova'
const novaGrammar = JSON.parse(
  readFileSync(new URL('../../grammar/nova.tmLanguage.json', import.meta.url), 'utf8')
)

export default defineConfig({
  site,
  base,
  trailingSlash: 'always',
  markdown: {
    shikiConfig: {
      themes: { light: 'gruvbox-light-medium', dark: 'gruvbox-dark-hard' },
      langs: [{ ...novaGrammar, name: 'nova' }],
      wrap: true
    }
  },
  integrations: [
    starlight({
      title: 'Nova',
      description:
        'A native-compiled language with readable syntax, explicit semantics and a professional desktop IDE.',
      favicon: '/favicon.svg',
      disable404Route: true,
      customCss: ['./src/styles/starlight.css'],
      social: [{ icon: 'github', label: 'GitHub', href: github }],
      expressiveCode: {
        shiki: { langs: [{ ...novaGrammar, name: 'nova' }] }
      },
      sidebar: [
        { label: 'Getting started', items: [{ slug: 'getting-started/installation' }, { slug: 'getting-started/hello-world' }, { slug: 'getting-started/first-project' }, { slug: 'getting-started/troubleshooting' }] },
        { label: 'Language', items: [{ slug: 'language/tour' }, { slug: 'language/reference' }, { slug: 'language/memory' }] },
        { label: 'Standard library', items: [{ slug: 'stdlib' }] },
        { label: 'Libraries', items: [{ slug: 'libraries/tui' }] },
        { label: 'Toolchain', items: [{ slug: 'toolchain/cli' }, { slug: 'toolchain/projects' }, { slug: 'toolchain/lsp' }, { slug: 'toolchain/vscode' }, { slug: 'toolchain/playground' }] },
        { label: 'Nova Studio', items: [{ slug: 'studio/overview' }, { slug: 'studio/shortcuts' }, { slug: 'studio/vim' }, { slug: 'studio/freebuff' }] },
        { label: 'Internals', items: [{ slug: 'internals/compiler' }, { slug: 'internals/runtime' }] },
        { label: 'Project', items: [{ slug: 'status' }, { slug: 'contributing' }, { slug: 'cookbook' }, { slug: 'website' }] }
      ]
    }),
    sitemap()
  ]
})
