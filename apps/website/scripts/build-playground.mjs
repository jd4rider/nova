import { execFileSync } from 'node:child_process'
import { copyFileSync, mkdirSync, statSync } from 'node:fs'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const here = dirname(fileURLToPath(import.meta.url))
const siteRoot = resolve(here, '..')
const repoRoot = resolve(siteRoot, '../..')
const source = resolve(
  repoRoot,
  'target/wasm32-unknown-unknown/release/nova_wasm.wasm'
)
const targetDir = resolve(siteRoot, 'public/playground')
const target = resolve(targetDir, 'nova.wasm')

execFileSync(
  'cargo',
  ['build', '-p', 'nova-wasm', '--target', 'wasm32-unknown-unknown', '--release'],
  { cwd: repoRoot, stdio: 'inherit' }
)
mkdirSync(targetDir, { recursive: true })
copyFileSync(source, target)
console.log(`playground wasm: ${source} -> ${target} (${statSync(target).size} bytes)`)
