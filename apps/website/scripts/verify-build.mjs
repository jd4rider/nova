import { existsSync, readFileSync, readdirSync, statSync } from 'node:fs'
import { extname, join, relative, resolve } from 'node:path'

const here = import.meta.dirname
const dist = resolve(here, '../dist')
const site = process.env.SITE_URL ?? 'http://localhost:4321'
const rawBase = process.env.BASE_PATH ?? '/'
const base = rawBase === '/' ? '/' : `/${rawBase.replace(/^\/+|\/+$/g, '')}/`
const errors = []

function walk(dir) {
  return readdirSync(dir).flatMap((name) => {
    const full = join(dir, name)
    return statSync(full).isDirectory() ? walk(full) : [full]
  })
}

function existsFor(rel) {
  const clean = decodeURIComponent(rel.replace(/^\/+/, ''))
  const candidates =
    clean === '' || clean.endsWith('/')
      ? [`${clean}index.html`]
      : extname(clean)
        ? [clean]
        : [clean, `${clean}.html`, `${clean}/index.html`]
  return candidates.some((candidate) => existsSync(join(dist, candidate)))
}

if (!existsSync(dist)) {
  console.error('dist/ does not exist; run npm run build first.')
  process.exit(1)
}

const required = [
  'index.html',
  '404.html',
  'install/index.html',
  'language/index.html',
  'studio/index.html',
  'docs/index.html',
  'blog/index.html',
  'blog/building-nova-in-rust/index.html',
  'website/index.html',
  'roadmap/index.html',
  'benchmarks/index.html',
  'releases/index.html',
  'contribute/index.html',
  'community/index.html',
  'license/index.html',
  'security/index.html',
  'getting-started/installation/index.html',
  'getting-started/hello-world/index.html',
  'language/tour/index.html',
  'studio/overview/index.html',
  'toolchain/vscode/index.html',
  'status/index.html',
  'pagefind/pagefind.js',
  'sitemap-index.xml',
  'robots.txt'
]
for (const file of required) {
  if (!existsSync(join(dist, file))) errors.push(`missing required output: ${file}`)
}

const htmlFiles = walk(dist).filter((file) => file.endsWith('.html'))
let checkedLinks = 0

for (const file of htmlFiles) {
  const source = readFileSync(file, 'utf8')
  const relFile = relative(dist, file).replaceAll('\\', '/')
  const pageRel = relFile.endsWith('index.html') ? relFile.slice(0, -'index.html'.length) : relFile
  const pageUrl = new URL(`${base}${pageRel}`, 'https://site.test')

  for (const match of source.matchAll(/\b(?:href|src)\s*=\s*["']([^"']+)["']/gi)) {
    let raw = match[1].replaceAll('&amp;', '&').trim()
    if (!raw || raw.startsWith('#') || raw.startsWith('//')) continue
    if (/^(?:https?:|mailto:|tel:|data:|javascript:)/i.test(raw)) continue
    checkedLinks += 1

    let resolved
    try {
      resolved = new URL(raw, pageUrl)
    } catch {
      errors.push(`${relFile}: invalid URL ${raw}`)
      continue
    }
    if (resolved.origin !== pageUrl.origin) continue

    const pathname = decodeURIComponent(resolved.pathname)
    if (base !== '/' && !pathname.startsWith(base)) {
      errors.push(`${relFile}: root-relative URL escapes base ${base}: ${raw}`)
      continue
    }
    const rel = base === '/' ? pathname.slice(1) : pathname.slice(base.length)
    if (!existsFor(rel)) errors.push(`${relFile}: broken internal target ${raw}`)
  }

  const canonical = source.match(/<link[^>]+rel=["']canonical["'][^>]+href=["']([^"']+)/i)?.[1]
  if (canonical) {
    const canonicalUrl = new URL(canonical.replaceAll('&amp;', '&'))
    if (!canonicalUrl.pathname.startsWith(base)) {
      errors.push(`${relFile}: canonical URL is outside base ${base}: ${canonical}`)
    }
    if (canonicalUrl.origin !== new URL(site).origin && canonicalUrl.origin !== 'http://localhost:4321') {
      errors.push(`${relFile}: canonical origin does not match SITE_URL: ${canonical}`)
    }
  }
}

const robots = readFileSync(join(dist, 'robots.txt'), 'utf8')
if (!robots.includes(`${base.replace(/\/$/, '')}/sitemap-index.xml`)) {
  errors.push(`robots.txt does not point at ${base}sitemap-index.xml`)
}

const sitemap = readFileSync(join(dist, 'sitemap-0.xml'), 'utf8')
if (!sitemap.includes(`${base}getting-started/hello-world/`)) {
  errors.push(`sitemap does not include the hello-world route under ${base}`)
}

if (errors.length) {
  console.error(`Website verification failed (${errors.length} problem(s)):`)
  for (const error of errors.slice(0, 50)) console.error(`- ${error}`)
  process.exit(1)
}

console.log(`Website verification passed: ${htmlFiles.length} pages, ${checkedLinks} internal references, base ${base}.`)
