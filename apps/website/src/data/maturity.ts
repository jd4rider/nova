export type Maturity = 'tested' | 'implemented' | 'experimental' | 'planned' | 'unavailable'

export interface Feature {
  area: string
  status: Maturity
  note: string
  evidence: string
}

export const features: Feature[] = [
  { area: 'Lexer, parser, spans and recovery', status: 'tested', note: 'UTF-8 identifiers, interpolation, error recovery', evidence: 'parser.sh 54/54' },
  { area: 'Type checker and native codegen', status: 'tested', note: 'Scalars, strings, lists, structs, Option/Result and match for the supported subset', evidence: 'codegen.sh 248/248; diff.sh 18/18' },
  { area: 'Checked arithmetic and bounds', status: 'tested', note: 'Overflow and index failures abort with source locations', evidence: 'codegen.sh overflow, bounds and native cases' },
  { area: 'CLI, formatter, project manifest', status: 'tested', note: 'new/init/run/build/check/fmt/doctor/lsp/tree, nova test/repl and nova box', evidence: 'cli.sh 33/33; toolchain.sh 126/126' },
  { area: 'Language server', status: 'implemented', note: 'Diagnostics, completion, hover, definition, symbols, formatting', evidence: 'lsp.sh diagnostics, completion, hover, definition, symbols and formatting' },
  { area: 'VS Code extension', status: 'implemented', note: 'Nova grammar and language client for diagnostics, completion, hover, definitions, symbols and formatting', evidence: 'Typecheck, VSIX package, code --install-extension and live nova lsp transcript; no Extension Host automation' },
  { area: 'Browser playground', status: 'experimental', note: 'Frozen WebAssembly front end from the former compiler; check/format only', evidence: 'checked-in wasm artifacts; static route and examples pass; Nova wasm backend missing' },
  { area: 'Nova Studio workbench', status: 'implemented', note: 'Monaco, Dark+ default, explorer, tasks, terminal, Vim, tab actions and VS Code shortcuts', evidence: 'TypeScript and production build pass; GUI interaction not automated' },
  { area: 'Nova Engine Studio shell', status: 'tested', note: 'Native Unity-first shell over one six-entity World: hierarchy, Scene/Game views, read-only inspector, play/step and console; CPU viewport, not embedded Metal yet', evidence: 'scene-gui shell.sh: both tab captures, traffic-light trace, 12 pixel checks; installed app signature verified' },
  { area: 'Live workspace file watching', status: 'implemented', note: 'Recursive watcher refreshes the explorer and reloads clean tabs; dirty buffers are never overwritten', evidence: 'Recursive fs.watch probe verified events; no automated GUI test' },
  { area: 'String predicates, trimming and layout', status: 'tested', note: 'contains, starts_with, ends_with, trim, ANSI-aware display_width, clip, word wrap and graphemes', evidence: 'Native codegen tests for predicates and Unicode/ANSI/grapheme layout' },
  { area: 'Terminal input and events', status: 'tested', note: 'TTY/raw mode, key polling, terminal size, UTF-8 input, common CSI keys, SGR/X10 mouse, bracketed paste and polled resize events', evidence: 'Native terminal-size test and a PTY transcript sending resize, wheel, click, paste and quit' },
  { area: 'User enums with payloads', status: 'tested', note: 'Unit variants or one payload, exhaustive match and nominal types', evidence: 'Native codegen test constructs and matches three variants' },
  { area: 'Visual effects and synthesized sound', status: 'experimental', note: 'Current-line lens, typing/scroll sparkles, Web Audio SFX, reduced-motion aware', evidence: 'Settings, menu and palette toggles; no automated GUI test yet' },
  { area: 'Generics, traits, closures', status: 'experimental', note: 'Explicit one-parameter generic functions, static trait conformance with concrete method dispatch, value-capturing closures and function values; generic structs/methods and dyn Trait are not implemented', evidence: 'cli.sh generic, closure and trait build/run; missing trait method E0201' },
  { area: 'Nova TUI library', status: 'experimental', note: 'Panels, tables, grapheme-aware wrapping, raw keys, mouse, paste and live resize plus a Bubble Tea-style model/update/view runtime; POSIX verified, Windows untested', evidence: 'examples/tui and examples/bible PTY runs; libraries/nova-tui' },
  { area: 'HTTP and JSON builtins', status: 'experimental', note: 'http_get shells out to curl without a shell; json_string decodes one string field and is not a parser; char_string converts key codes; POSIX only', evidence: 'examples/bible fetched John 3:16 from bible-api.com in a PTY' },
  { area: 'Web server framework', status: 'experimental', note: 'nova-web: blocking HTTP/1.1 on 127.0.0.1, typed Request/Response, state threaded through the loop, exact-path router; no TLS, custom headers, static files or concurrency', evidence: 'examples/haiku served 200/201/404 responses and persisted state under curl' },
  { area: 'Typed HTML and HTMX templates', status: 'experimental', note: 'Templ-style templates as Nova functions with escaping, element and attribute helpers, and first-class hx_get/hx_post/trigger/confirm; no template files or markup compiler', evidence: 'examples/haiku page and fragments; user input came back escaped' },
  { area: 'User modules and dependency imports', status: 'experimental', note: 'Path imports with cycle checks; `import "name"` resolves declared path or fetched registry dependencies. `as` aliases qualify calls and types; the namespace stays flat and only direct dependencies are mapped', evidence: 'sema path-import tests; boxes.sh builds a path-dependency and a fetched registry-box program' },
  { area: 'Nova Boxes package manager', status: 'implemented', note: 'Box.toml, SemVer, path and registry resolution, deterministic .box archives, SHA-256 verified fetch, search/info, offline records', evidence: 'boxes.sh: 64 assertions including HTTP mirror reads and checksum rejection; publishing not implemented' },
  { area: 'Nova self-hosted tooling', status: 'experimental', note: 'Compiler, formatter, lexer, parser, checks, diagnostics, Boxes, CLI and language server are all Nova; `bootstrap/nova` rebuilds the toolchain', evidence: 'toolchain.sh 126/126; codegen.sh 248/248; lsp.sh; docs/rust-migration.md' },
  { area: 'Scalar C FFI', status: 'tested', note: '`extern "C"` declarations with Bool/Int/Float/Char/Unit values passed by copy; no pointer ownership transfer or unwinding across the boundary', evidence: 'Native codegen FFI test and the TUI timer/flush calls' },
  { area: 'Async scheduling and debug adapter', status: 'planned', note: 'No async syntax, tasks, channels or DAP', evidence: 'ROADMAP.md' },
  { area: 'AI panel: FreeBuff TUI and provider chat', status: 'implemented', note: 'FreeBuff CLI embedded in a PTY; streaming chat via Claude or any OpenAI-compatible endpoint with keys encrypted in the main process', evidence: 'Typecheck, production build and local SSE adapter tests; no automated GUI test yet' },
  { area: 'FreeBuff headless/structured mode', status: 'unavailable', note: 'CLI 0.2.36 rejects --print/-p and starts its TUI on stdin; Studio probes --help instead of scraping', evidence: 'docs/adr/0002-freebuff-integration.md' },
  { area: 'Public website and docs', status: 'implemented', note: 'Astro 7 + Starlight site deployed at jd4rider.github.io/nova', evidence: 'GitHub Pages build/deploy succeeded; live routes verified' },
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
