import { existsSync } from 'node:fs'
import path from 'node:path'

export function novaExecutable(): string {
  const configured = process.env.NOVA_BIN
  if (configured && configured.trim().length > 0) return configured
  const candidates = [
    path.resolve(process.cwd(), '../target/debug/nova'),
    path.resolve(process.cwd(), 'target/debug/nova'),
    path.resolve(process.cwd(), '../target/release/nova'),
    path.resolve(process.cwd(), 'target/release/nova')
  ]
  for (const candidate of candidates) {
    if (existsSync(candidate)) return candidate
  }
  return 'nova'
}

export function isNovaProject(root: string): boolean {
  return existsSync(path.join(root, 'nova.toml'))
}
