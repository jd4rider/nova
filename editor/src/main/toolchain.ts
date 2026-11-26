import { existsSync } from 'node:fs'
import path from 'node:path'

export function novaExecutable(): string {
  const configured = process.env.NOVA_BIN
  if (configured && configured.trim().length > 0) return configured
  const candidates = [
    path.resolve(process.cwd(), '../bootstrap/nova'),
    path.resolve(process.cwd(), 'bootstrap/nova'),
    path.resolve(process.cwd(), '../examples/nova-tool/target/nova/nova-tool'),
    path.resolve(process.cwd(), 'examples/nova-tool/target/nova/nova-tool')
  ]
  for (const candidate of candidates) {
    if (existsSync(candidate)) return candidate
  }
  return 'nova'
}

/**
 * The Nova-implemented toolchain binary. `nova-tool lsp` is the language
 * server; Studio prefers it over `nova lsp` so no Rust binary is in the
 * request path.
 */
export function novaToolExecutable(): string | null {
  const configured = process.env.NOVA_TOOL_BIN
  if (configured && configured.trim().length > 0 && existsSync(configured)) return configured
  const executable = novaExecutable()
  const candidates = [
    path.join(path.dirname(executable), 'nova-tool'),
    path.resolve(process.cwd(), '../bootstrap/nova'),
    path.resolve(process.cwd(), 'bootstrap/nova'),
    path.resolve(process.cwd(), '../examples/nova-tool/target/nova/nova-tool'),
    path.resolve(process.cwd(), 'examples/nova-tool/target/nova/nova-tool')
  ]
  for (const candidate of candidates) {
    if (existsSync(candidate)) return candidate
  }
  return null
}

export function isNovaProject(root: string): boolean {
  return existsSync(path.join(root, 'nova.toml'))
}
