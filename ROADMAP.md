# Nova roadmap and verified feature matrix

Status means: **tested** = covered by automated tests and passing; **implemented** = works in manual verification but lacks focused tests; **experimental** = partial/limited; **planned** = not implemented. Nothing here is marked complete without evidence.

## Language and compiler

| Area | Status | Notes |
|---|---|---|
| UTF-8 lexer, Uniocde identifiers, spans, comments, raw strings | tested | `cargo test -p nova-lexer` |
| Recursive-descent + Pratt parser, recovery, interpolation | tested | `cargo test -p nova-parser` |
| Name resolution, local inference, signatures, match exhaustiveness | tested | 22 sema tests; no generics/traits/closures |
| Scalars, `String`, `List`, structs, Option, Result, `?` | tested | Native codegen tests |
| `let`/`var`, if/while/for, ranges, assignment, `+=` family | tested | Checked overflow, bounds checks |
| Integer overflow traps, division by zero | tested | Runtime abort with message |
| Pattern matching | tested | Option/Result, literals, bindings and user enum variants; one payload per variant |
| String predicates and trimming | tested | `contains`, `starts_with`, `ends_with`, `trim` lower to runtime helpers; native codegen test executes them |
| Modules/imports | experimental | Parsed and resolved for std imports only; no user module graph or path imports |
| Generics | planned | `first<T>` syntax not accepted by sema |
| User enums | tested | Unit variants or one payload; nominal type, tagged heap payload, exhaustive `match`; native codegen test |
| Traits, impls, methods | planned | |
| Closures/first-class functions | planned | |
| Async/await, tasks, channels | planned | No scheduler |
| FFI / `extern "C"` | planned | |
| Debug info and DAP | planned | No DWARF emission yet |
| Incremental compilation | planned | Artifacts cached only per invocation |
| `Map`, `Set`, tuples, fixed-width ints, arrays/slices | planned | List and scalar i64/f64/bool/char exist |
| Defer, named/default arguments, macros | planned | |

## Tooling

| Area | Status | Notes |
|---|---|---|
| `nova new/init/run/build/check/test/fmt/clean/doctor/repl/tree` | tested/implemented | CLI smoke-tested; `repl` uses interpreter-labeled evaluation |
| Project manifest, lockfile, local path dependencies | tested | No registry, workspaces, or semver registry resolution |
| Formatter | implemented | Conservative indentation/whitespace; not a full pretty-printer |
| LSP stdio JSON-RPC | tested manually | Full sync, diagnostics, completion, hover, definition, symbols, formatting |
| VS Code extension | implemented | Shared TextMate grammar and language client; VSIX packaged and installed, live `nova lsp` transcript verified; no Extension Host automation |
| Browser playground | implemented | Real lexer, parser, sema and formatter compiled to WASM; static route and ABI tests pass; check/format only |
| LSP incremental sync, rename, references, semantic tokens, code actions, cancellation | planned | Not advertised |
| Package registry, publishing, checksums | planned | |
| Benchmarks and performance claims | planned | No benchmark evidence; directive performance targets are aspirations only |
| Astro 7 + Starlight public website and docs | tested | `npm run check`, production build, root/project-base link checks, grammar scopes, checked snippets |
| GitHub Pages workflow | implemented | Public repository and verified deployment at https://jd4rider.github.io/nova/ |
| Unsigned desktop installers | implemented | v0.2.0 macOS arm64 DMG, Windows x64 NSIS EXE, macOS CLI archive and SHA-256 manifest |
| Homebrew formula and Studio cask | tested | `brew install --cask jd4rider/tap/nova-studio` installed CLI 0.1.0 and the app; the tap is not yet bumped to 0.2.0 |
| VS Code Marketplace / Open VSX | planned | The 0.1.0 extension is a local VSIX only |
| Signed desktop releases | planned | Unsigned installers exist; no Developer ID/notarization or Authenticode |

## Nova Studio

| Area | Status | Notes |
|---|---|---|
| Electron + React + TS strict + Vite build | tested | `npm run typecheck`, `npm run build` pass; production app launched manually |
| Explorer, tabs, dirty state, atomic save conflict check | implemented | Runtime launch verified; interaction not automated |
| Restore last workspace on launch | implemented | Path stored in Electron userData and reopened when the directory still exists |
| Live workspace watching | implemented | Recursive `fs.watch` coalesced in the main process; clean tabs reload, dirty buffers are untouched, explorer refreshes |
| Tab actions, Save All, Go to Line, VS Code shortcuts | implemented | Tab context menu, ⌘W/⌘B/⌘J/⌃G/⌥⌘S; no automated GUI interaction test |
| IDE architecture decision | implemented | Electron retained; 431,792 KB idle RSS measured; Tauri/native spikes still unfinished |
| Monaco + Nova grammar + Dark+/Gruvbox themes | implemented | Monarch tokenizer; VS Code Dark+ is the default look; no TextMate/Tree-sitter yet |
| LSP integration | implemented | Providers bind to the Rust server; runtime round trip not yet automated |
| Build/run/check/test tasks | implemented | Real child processes, bounded output, workspace cwd |
| Terminal | experimental | xterm + optional node-pty; pipe fallback when native build unavailable |
| Vim mode | experimental | monaco-vim; representative ex commands; not full Neovim |
| Visual effects + synthesized sound | experimental | Persisted toggles, current-line lens, typing/scroll sparkles, Web Audio SFX, reduced-motion aware; no automated GUI test |
| Search / quick open / command palette / settings | implemented | Ripgrep-backed search |
| Git panel, debugger, test explorer UI, split editor groups, multi-window | planned | |
| AI panel: embedded FreeBuff TUI | implemented | Real CLI in a PTY; no output scraping, requires `node-pty` |
| AI chat: Claude and OpenAI-compatible endpoints | implemented | Streaming adapters tested against a local SSE server; keys encrypted in the main process |
| FreeBuff headless/structured mode | not exposed by the CLI | 0.2.36 rejects `--print`/`-p`; Studio probes `--help` instead of assuming |
| Inline AI completion, agent edit review, Copilot OAuth | planned | Not implemented; GitHub Models works with a personal access token |
| IDE unit/e2e tests | planned | No Vitest/Playwright suite yet |

## Next concrete tasks

1. Add Playwright Electron smoke test: launch, open fixture workspace, assert diagnostics and build output.
2. Run time-boxed Tauri 2 and native Rust spikes; update ADR 0001 with real measurements.
3. Keep the live Pages deployment verified; add a custom domain and signed release artifacts only with explicit owner authorization.
4. Implement LSP incremental document sync and cancellation.
5. Implement user modules and path-based imports.
6. Add generics and user enums, then traits.
7. Add DWARF debug info and a DAP adapter only after verifying emitted information.
8. Add benchmark harness with recorded hardware and honest comparisons.
9. Implement user modules and file linking, then scaffold templates beyond the single binary project.
10. Implement user enums, classes/traits, generics and methods with typed tests and codegen.
11. Add `std.http` with HTTPS, package resolution and a documented security boundary.
12. Build real Nova libraries and TUI/GUI/WASM framework packages only after modules, packaging and native/runtime abstractions exist.
