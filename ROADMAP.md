# Nova roadmap and verified feature matrix

Status means: **tested** = covered by automated tests and passing; **implemented** = works in manual verification but lacks focused tests; **experimental** = partial/limited; **planned** = not implemented. Nothing here is marked complete without evidence.

## Language and compiler

| Area | Status | Notes |
|---|---|---|
| UTF-8 lexer, Uniocde identifiers, spans, comments, raw strings | tested | `cargo test -p nova-lexer` |
| Recursive-descent + Pratt parser, recovery, interpolation | tested | `cargo test -p nova-parser` |
| Name resolution, local inference, signatures, match exhaustiveness | tested | 22 sema tests plus native tests for generics, traits and closures |
| Scalars, `String`, `List`, structs, Option, Result, `?` | tested | Native codegen tests |
| `let`/`var`, if/while/for, ranges, assignment, `+=` family | tested | Checked overflow, bounds checks |
| Integer overflow traps, division by zero | tested | Runtime abort with message |
| Pattern matching | tested | Option/Result, literals, bindings, tuple patterns and user enum variants with multiple payload fields |
| String predicates and trimming | tested | `contains`, `starts_with`, `ends_with`, `trim` lower to runtime helpers; native codegen test executes them |
| Modules/imports | tested | `import "util.nova"` flattens relative source files with cycle checks; `import "util.nova" as util` adds qualified calls and type names. The underlying namespace is still flat, so duplicate unqualified names remain errors; packages are not implemented |
| Generics | tested | Generic functions, structs, enums and inherent methods are monomorphised on demand from concrete annotations; trait impls on generic types and type-parameter-annotated locals remain unsupported |
| User enums | tested | Unit or multi-payload variants; nominal tagged heap payload, exhaustive `match`; native codegen test |
| `impl` methods | tested | Inherent methods on structs/enums and on generic types (`impl<T> Pair<T>`) with an explicit `self` receiver; no static methods or trait impls on generic types |
| Tuples, fixed arrays, slices | tested | Tuple construction/access/patterns/mutation, `[T; N]` bounds-checked indexing, borrowed `Slice<T>` from `.as_slice()`; no tuple `let` destructuring |
| Traits | tested | `trait`, static `impl Trait for Type`, and `dyn Trait` objects that box a copy of the value and dispatch through a per-type vtable; trait impls on generic types remain unsupported |
| Closures/first-class functions | tested | Capturing closures by value, nested captures, closures returned from functions and `fn(...) -> T` function values; mutable captures and escaping-lifetime analysis are not supported |
| Async/await, tasks, channels | experimental | `async fn`/`await` run synchronously; no scheduler, tasks or channels |
| FFI / `extern "C"` | tested | Scalar-only C ABI (Bool/Int/Float/Char/Unit) validated at compile time, declarations emitted `nounwind`, values passed by copy; no pointer ownership transfer or unwinding across the boundary |
| Debug info and DAP | planned | No DWARF emission yet |
| Incremental compilation | planned | Artifacts cached only per invocation |
| `Map`, `Set`, fixed-width ints | tested/partial | Hashed `Map`/`Set` (open addressing, FNV-1a) with Int/Bool/Char/String keys; fixed-width ints remain planned |
| Defer, named/default arguments, macros | partial | Expression and statement macros expand by AST substitution; `defer` and named/default arguments remain planned |

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
5. Add package dependencies and registry resolution on top of aliased modules.
6. Add DWARF debug info and a DAP adapter only after verifying emitted information.
7. Add benchmark harness with recorded hardware and honest comparisons.
8. Implement user modules and file linking, then scaffold templates beyond the single binary project.
9. Add real async scheduling with tasks and channels.
10. Add `std.http` with HTTPS, package resolution and a documented security boundary.
11. Build real Nova libraries and TUI/GUI/WASM framework packages only after modules, packaging and native/runtime abstractions exist.
