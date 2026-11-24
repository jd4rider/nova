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
  { area: 'Nova Studio workbench', status: 'implemented', note: 'Monaco, Dark+ default, explorer, tasks, terminal, Vim, tab actions and VS Code shortcuts', evidence: 'TypeScript and production build pass; GUI interaction not automated' },
  { area: 'Live workspace file watching', status: 'implemented', note: 'Recursive watcher refreshes the explorer and reloads clean tabs; dirty buffers are never overwritten', evidence: 'Recursive fs.watch probe verified events; no automated GUI test' },
  { area: 'String predicates and trimming', status: 'tested', note: 'contains, starts_with, ends_with and trim on String', evidence: 'Native codegen test compiles and runs a program using all four' },
  { area: 'User enums with payloads', status: 'tested', note: 'Unit variants or one payload, exhaustive match and nominal types', evidence: 'Native codegen test constructs and matches three variants' },
  { area: 'Visual effects and synthesized sound', status: 'experimental', note: 'Current-line lens, typing/scroll sparkles, Web Audio SFX, reduced-motion aware', evidence: 'Settings, menu and palette toggles; no automated GUI test yet' },
  { area: 'Generics, traits, closures', status: 'experimental', note: 'Generic functions, structs, enums and inherent methods are monomorphised; traits check static conformance and `dyn Trait` dispatches through boxed vtables; closures capture by value', evidence: 'ROADMAP.md and native codegen tests' },
  { area: 'Nova TUI library', status: 'experimental', note: 'ANSI panels, lists, tables, progress, spinners and screen helpers; path-imported, no raw input or Unicode width calculation', evidence: 'examples/tui builds and runs with the current nova binary; libraries/nova-tui' },
  { area: 'User modules', status: 'experimental', note: 'Path imports with cycle checks; an optional `as` alias qualifies calls and type names; the namespace remains flat and packages are not implemented', evidence: 'sema path-import tests and the TUI demo import' },
  { area: 'Scalar C FFI', status: 'tested', note: '`extern "C"` declarations with Bool/Int/Float/Char/Unit values passed by copy; no pointer ownership transfer or unwinding across the boundary', evidence: 'Native codegen FFI test and the TUI timer/flush calls' },
  { area: 'Async scheduling and debug adapter', status: 'planned', note: '`async`/`await` executes synchronously; no tasks, channels or DAP', evidence: 'ROADMAP.md' },
  { area: 'AI panel: FreeBuff TUI and provider chat', status: 'implemented', note: 'FreeBuff CLI embedded in a PTY; streaming chat via Claude or any OpenAI-compatible endpoint with keys encrypted in the main process', evidence: 'Typecheck, production build and local SSE adapter tests; no automated GUI test yet' },
  { area: 'FreeBuff headless/structured mode', status: 'unavailable', note: 'CLI 0.2.36 rejects --print/-p and starts its TUI on stdin; Studio probes --help instead of scraping', evidence: 'docs/adr/0002-freebuff-integration.md' },
  { area: 'Public website and docs', status: 'implemented', note: 'Astro 7 + Starlight site deployed at jd4rider.github.io/nova', evidence: 'GitHub Pages build/deploy run 45244354509 succeeded' },
  { area: 'Unsigned desktop installers', status: 'implemented', note: 'macOS DMG, Windows NSIS EXE, macOS CLI archive and checksums', evidence: 'v0.2.0 release assets; DMG mounted and CLI smoke-tested' },
  { area: 'Homebrew Studio + language', status: 'tested', note: 'Studio cask depends on jd4rider/tap/nova and installs both', evidence: 'brew install verified nova 0.1.0 + app; the tap has not been bumped to 0.2.0 yet' },
  { area: 'Signed desktop releases', status: 'planned', note: 'Unsigned installers exist; no Developer ID/notarization or Authenticode', evidence: 'ROADMAP.md' }
]

export const milestones = [
  { id: 'A', name: 'Audit', status: 'done', note: 'Repository inventoried; existing compiler and IDE preserved.' },
  { id: 'B', name: 'Architecture', status: 'partial', note: 'Electron retained with rubric and measurements; Tauri and native candidates not prototyped.' },
  { id: 'C', name: 'Foundation', status: 'done', note: 'Native compiler works; Studio builds and launches. Editing interaction is not automated.' },
  { id: 'D', name: 'Website foundation', status: 'done', note: 'Astro/Starlight site builds at root and project base paths.' },
  { id: 'E', name: 'Workbench', status: 'partial', note: 'Gruvbox, Vim, terminal, LSP and settings exist; Git and DAP are planned.' },
  { id: 'F', name: 'AI', status: 'partial', note: 'FreeBuff TUI plus Claude/OpenAI-compatible streaming chat; no inline completion, agent edits or Copilot OAuth.' },
  { id: 'G', name: 'Professional tooling', status: 'planned', note: 'DAP, test explorer, recovery and cross-platform evidence remain.' },
  { id: 'H', name: 'Ecosystem', status: 'partial', note: 'Versioned docs and tested snippets; benchmarks and releases remain.' },
  { id: 'I', name: 'Publication', status: 'done', note: 'Public repository created; Pages deployed and live routes verified at https://jd4rider.github.io/nova/.' }
] as const
