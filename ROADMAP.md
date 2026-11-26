# Nova roadmap and verified feature matrix

Status means: **tested** = covered by automated tests and passing; **implemented** = works in manual verification but lacks focused tests; **experimental** = partial/limited; **planned** = not implemented. Nothing here is marked complete without evidence.

## Language and compiler

| Area | Status | Notes |
|---|---|---|
| UTF-8 lexer, Unicode identifiers, spans, comments, raw strings | tested | `examples/nova-tool/tests/parser.sh` (51/51) |
| Recursive-descent + Pratt parser, interpolation | tested | `examples/nova-tool/tests/parser.sh` (51/51); recovery parity is incomplete |
| Name resolution, local inference, signatures, match exhaustiveness | partial | 39 sema-ast assertions and 27 frozen diagnostics; not full type inference |
| Scalars, `String`, `List`, structs, Option, Result, `?` | tested | Native codegen tests |
| `let`/`var`, if/while/for, ranges, assignment, `+=` family | tested | Checked overflow, bounds checks |
| Integer overflow traps, division by zero | tested | Runtime abort with message |
| Pattern matching | tested | Option/Result, literals, bindings, tuple patterns and user enum variants with multiple payload fields |
| String predicates and trimming | tested | `contains`, `starts_with`, `ends_with`, `trim` lower to runtime helpers; native codegen test executes them |
| Modules/imports | tested | `import "util.nova"` flattens relative source files with cycle checks; `import "util"` and `import "@publisher/box"` resolve declared dependencies through `Box.toml` and `Box.lock`. The underlying namespace is still flat, so duplicate unqualified names remain errors; only direct dependencies are mapped |
| Generics | planned | Not implemented by the Nova compiler; tracked as COMP-01.04 |
| User enums | tested | Unit or multi-payload variants; nominal tagged heap payload, exhaustive `match`; native codegen test |
| `impl` methods | planned | Not implemented by the Nova compiler; tracked with COMP-01.04 |
| Tuples, fixed arrays, slices | partial | Tuple construction/access/patterns and mutation, `[T; N]` bounds-checked indexing, borrowed `Slice<T>` views; no tuple `let` destructuring |
| Traits | planned | Not implemented by the Nova compiler; tracked with COMP-01.04 |
| Closures/first-class functions | planned | Not implemented by the Nova compiler; tracked with COMP-01.04 |
| Async/await, tasks, channels | experimental | `async fn`/`await` run synchronously; no scheduler, tasks or channels |
| FFI / `extern "C"` | tested | Scalar-only C ABI (Bool/Int/Float/Char/Unit) validated at compile time, declarations emitted `nounwind`, values passed by copy; no pointer ownership transfer or unwinding across the boundary |
| Debug info and DAP | planned | No DWARF emission yet |
| Incremental compilation | planned | Artifacts cached only per invocation |
| `Map`, `Set`, fixed-width ints | partial | Annotated `map()`/`set()` with `len`, `get`, `contains` and `insert` for Int/Bool/String keys; no literals or iteration; fixed-width ints planned |
| Defer, named/default arguments, macros | partial | Expression and statement macros expand by AST substitution; `defer` and named/default arguments remain planned |

## Tooling

| Area | Status | Notes |
|---|---|---|
| `nova new/init/run/build/check/fmt/clean/doctor/tree` and `nova box` | tested | `cli.sh` 22/22 and `toolchain.sh` 126/126; `test` and `repl` are not ported |
| Project manifest, lockfile, path and registry dependencies | tested | `Box.toml`, `Box.lock`, SemVer and fetched registry boxes via `nt_registry_module`; no workspaces |
| Formatter | implemented | Conservative indentation/whitespace; not a full pretty-printer |
| LSP stdio JSON-RPC | tested manually | Full sync, diagnostics, completion, hover, definition, symbols, formatting |
| VS Code extension | implemented | Shared TextMate grammar and language client; VSIX packaged and installed, live `nova lsp` transcript verified; no Extension Host automation |
| Browser playground | frozen artifact | Checked-in WASM passes static and source checks; rebuilding needs a Nova wasm backend |
| LSP incremental sync, rename, references, semantic tokens, code actions, cancellation | planned | Not advertised |
| Nova Boxes: manifests, SemVer, path/registry resolution, deterministic archives | implemented | `examples/nova-tool/tests/boxes.sh` (64 assertions), including SHA-256 verified fetch and registry reads |
| Registry publishing, signing and provenance | planned | Reviewed-PR protocol is designed in `docs/boxes/REGISTRY_PROTOCOL.md`; no workflow publishes |
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
| LSP integration | implemented | Providers bind to the Nova server; `lsp.sh` verifies diagnostics, completion, hover, definition, symbols and formatting |
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
2. Run time-boxed native-shell evaluations without adding a Rust dependency; update ADR 0001 with real measurements.
3. Keep the live Pages deployment verified; add a custom domain and signed release artifacts only with explicit owner authorization.
4. Implement LSP incremental document sync and cancellation.
5. Add package dependencies and registry resolution on top of aliased modules.
6. Add DWARF debug info and a DAP adapter only after verifying emitted information.
7. Add benchmark harness with recorded hardware and honest comparisons.
8. Implement user modules and file linking, then scaffold templates beyond the single binary project.
9. Add real async scheduling with tasks and channels.
10. Add `std.http` with HTTPS, package resolution and a documented security boundary.
11. Build real Nova libraries and TUI/GUI/WASM framework packages only after modules, packaging and native/runtime abstractions exist.
