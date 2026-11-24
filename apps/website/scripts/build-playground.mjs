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
const fmtSource = resolve(
  repoRoot,
  'examples/nova-fmt-wasm/target/nova/nova-fmt-wasm.wasm'
)
const fmtTarget = resolve(targetDir, 'nova-fmt.wasm')

execFileSync(
  'cargo',
  ['build', '-p', 'nova-wasm', '--target', 'wasm32-unknown-unknown', '--release'],
  { cwd: repoRoot, stdio: 'inherit' }
)
execFileSync('cargo', ['build', '-p', 'nova-cli'], { cwd: repoRoot, stdio: 'inherit' })
execFileSync(resolve(repoRoot, 'target/debug/nova'), ['build'], {
  cwd: resolve(repoRoot, 'examples/nova-fmt-wasm'),
  env: { ...process.env, NOVA_TARGET: 'wasm32-unknown-unknown' },
  stdio: 'inherit'
})
mkdirSync(targetDir, { recursive: true })
copyFileSync(source, target)
copyFileSync(fmtSource, fmtTarget)
console.log(`playground wasm: ${source} -> ${target} (${statSync(target).size} bytes)`)
console.log(`playground formatter: ${fmtSource} -> ${fmtTarget} (${statSync(fmtTarget).size} bytes)`)
