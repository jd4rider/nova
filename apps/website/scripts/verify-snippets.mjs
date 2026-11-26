import { existsSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, rmSync, statSync, writeFileSync } from 'node:fs'
import { join, resolve } from 'node:path'
import { spawnSync } from 'node:child_process'

const here = import.meta.dirname
const siteRoot = resolve(here, '..')
const repoRoot = resolve(siteRoot, '../..')
const docsRoot = join(siteRoot, 'src/content/docs')
const compiler = process.env.NOVA_BIN ?? join(repoRoot, 'bootstrap/nova')

if (!existsSync(compiler)) {
  console.error(`Nova compiler not found at ${compiler}. The checked-in bootstrap is bootstrap/nova.`)
  process.exit(1)
}

function walk(dir) {
  return readdirSync(dir).flatMap((name) => {
    const full = join(dir, name)
    return statSync(full).isDirectory() ? walk(full) : [full]
  })
}

const snippets = []
for (const file of walk(docsRoot).filter((file) => /\.mdx?$/.test(file))) {
  const source = readFileSync(file, 'utf8')
  for (const match of source.matchAll(/```nova[ \t]+check[^\n]*\n([\s\S]*?)```/g)) {
    snippets.push({ label: `${file.slice(docsRoot.length + 1)}`, source: match[1] })
  }
}
const example = readFileSync(join(repoRoot, 'examples/hello/src/main.nova'), 'utf8')
snippets.push({ label: 'examples/hello/src/main.nova', source: example })

if (snippets.length < 2) {
  console.error('No checked Nova snippets found; expected fences marked ```nova check.')
  process.exit(1)
}

let failed = 0
for (const snippet of snippets) {
  const dir = mkdtempSync(join(process.env.TMPDIR ?? '/tmp', 'nova-snippet-'))
  try {
    mkdirSync(join(dir, 'src'))
    writeFileSync(join(dir, 'nova.toml'), '[package]\nname = "snippet"\nversion = "0.1.0"\n\n[bin]\npath = "src/main.nova"\n')
    writeFileSync(join(dir, 'src/main.nova'), snippet.source)
    const result = spawnSync(compiler, ['check'], { cwd: dir, encoding: 'utf8', timeout: 120_000 })
    if (result.error || result.status !== 0) {
      failed += 1
      console.error(`FAILED ${snippet.label}`)
      console.error(result.error?.message ?? result.stderr ?? result.stdout)
      console.error('--- snippet ---')
      console.error(snippet.source)
    } else {
      console.log(`ok ${snippet.label}`)
    }
  } finally {
    rmSync(dir, { recursive: true, force: true })
  }
}

if (failed) {
  console.error(`${failed} checked Nova snippet(s) failed.`)
  process.exit(1)
}
console.log(`Checked ${snippets.length} Nova programs successfully with ${compiler}.`)
