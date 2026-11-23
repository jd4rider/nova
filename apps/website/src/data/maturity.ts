export type Maturity = 'tested' | 'implemented' | 'experimental' | 'planned' | 'unavailable'

export interface Feature {
  area: string
  status: Maturity
  note: string
  evidence: string
}

export const features: Feature[] = [
  { area: 'Lexer, parser, spans and recovery', status: 'tested', note: 'UTF-8 identifiers, interpolation, error recovery', evidence: 'crates/nova-lexer, crates/nova-parser tests' },
  { area: 'Type checker and native codegen', status: 'tested', note: 'Scalars, strings, lists, structs, Option/Result, match', evidence: 'cargo test --workspace: 72 tests' },
  { area: 'Checked arithmetic and bounds', status: 'tested', note: 'Overflow and index failures abort with source locations', evidence: 'nova-codegen-llvm native tests' },
  { area: 'CLI, formatter, project manifest', status: 'tested', note: 'new/init/run/build/check/test/fmt/doctor/lsp/tree', evidence: 'crates/nova-cli, crates/nova-fmt, crates/nova-pkg' },
  { area: 'Language server', status: 'implemented', note: 'Diagnostics, completion, hover, definition, symbols, formatting', evidence: 'Manual CLI/LSP checks; no automated transcript suite yet' },
  { area: 'VS Code extension', status: 'implemented', note: 'Nova grammar and language client for diagnostics, completion, hover, definitions, symbols and formatting', evidence: 'Typecheck, VSIX package, code --install-extension and live nova lsp transcript; no Extension Host automation' },
  { area: 'Browser playground', status: 'implemented', note: 'Real lexer, parser, type checker and formatter in WebAssembly; check/format only', evidence: 'WASM ABI tests, static route and examples; no native execution' },
  { area: 'Nova Studio workbench', status: 'implemented', note: 'Monaco, Gruvbox, explorer, tasks, terminal, Vim', evidence: 'TypeScript and production build pass; GUI interaction not automated' },
  { area: 'Visual effects and synthesized sound', status: 'experimental', note: 'Current-line lens, typing/scroll sparkles, Web Audio SFX, reduced-motion aware', evidence: 'Settings, menu and palette toggles; no automated GUI test yet' },
  { area: 'Generics, enums, traits, closures', status: 'planned', note: 'Unsupported syntax is rejected with diagnostics', evidence: 'ROADMAP.md' },
  { area: 'User modules', status: 'experimental', note: 'Standard imports only; no user module graph', evidence: 'crates/nova-sema' },
  { area: 'Async, FFI, debug adapter', status: 'planned', note: 'No scheduler, C ABI or DAP yet', evidence: 'ROADMAP.md' },
  { area: 'FreeBuff structured chat/completion', status: 'unavailable', note: 'Installed 0.2.36 CLI is interactive-terminal-only', evidence: 'docs/adr/0002-freebuff-integration.md' },
  { area: 'Public website and docs', status: 'implemented', note: 'Astro 7 + Starlight site deployed at jd4rider.github.io/nova', evidence: 'GitHub Pages build/deploy run 45244354509 succeeded' },
  { area: 'Unsigned desktop installers', status: 'implemented', note: 'macOS DMG, Windows NSIS EXE, macOS CLI archive and checksums', evidence: 'v0.1.0 release assets; DMG mounted and CLI smoke-tested' },
  { area: 'Homebrew Studio + language', status: 'tested', note: 'Studio cask depends on jd4rider/tap/nova and installs both', evidence: 'brew install verified nova 0.1.0 + /Applications/Nova Studio.app' },
  { area: 'Signed desktop releases', status: 'planned', note: 'Unsigned installers exist; no Developer ID/notarization or Authenticode', evidence: 'ROADMAP.md' }
]

export const milestones = [
  { id: 'A', name: 'Audit', status: 'done', note: 'Repository inventoried; existing compiler and IDE preserved.' },
  { id: 'B', name: 'Architecture', status: 'partial', note: 'Electron retained with rubric and measurements; Tauri and native candidates not prototyped.' },
  { id: 'C', name: 'Foundation', status: 'done', note: 'Native compiler works; Studio builds and launches. Editing interaction is not automated.' },
  { id: 'D', name: 'Website foundation', status: 'done', note: 'Astro/Starlight site builds at root and project base paths.' },
  { id: 'E', name: 'Workbench', status: 'partial', note: 'Gruvbox, Vim, terminal, LSP and settings exist; Git and DAP are planned.' },
  { id: 'F', name: 'AI', status: 'partial', note: 'Honest FreeBuff terminal fallback; structured AI is unavailable.' },
  { id: 'G', name: 'Professional tooling', status: 'planned', note: 'DAP, test explorer, recovery and cross-platform evidence remain.' },
  { id: 'H', name: 'Ecosystem', status: 'partial', note: 'Versioned docs and tested snippets; benchmarks and releases remain.' },
  { id: 'I', name: 'Publication', status: 'done', note: 'Public repository created; Pages deployed and live routes verified at https://jd4rider.github.io/nova/.' }
] as const
