# Current Nova checkpoint

Updated: 2026-11-24
Master directive: owner-supplied; parity matrix in `engineering/continuation/`.
Repository / branch: `nova` / `selfhost/pure-nova`
HEAD: `8092f97` (Int ABI); prior `98c1ded`.
Working tree: self-hosting files have the uncommitted ABI checkpoint; unrelated
`editor/` and playground changes remain untouched.

Active milestone: `CODEGEN-01` — Nova LLVM IR lowering
Active task: `CODEGEN-01.09` — clang invocation/linking from the Nova tool
Status: READY

## Last verified behavior

- `sh examples/nova-tool/tests/codegen.sh` → passed 15, failed 0.
  - Signatures are `define i64 @nova_user_<index>(ptr %arg_env, i64 ...)`.
  - Direct calls use `ptr null` for the hidden environment pointer.
  - `define i32 @main()` calls the entry symbol with `ptr null`.
  - Native binaries exit 9, 2 and 17.
- `sh examples/nova-tool/tests/parser.sh` → passed 50, failed 0.
- `sh examples/nova-tool/tests/sema-ast.sh` → passed 9, failed 0.
- `sh examples/nova-tool/tests/sema.sh` → passed 4, failed 0.
- Mach-O confirmed on this Apple Silicon host.

## Work completed since previous checkpoint

- Control flow: `if`/`else`, `while`, comparisons and branch labels.
- SSA for immutable params/`let`; mutable bindings use slots.
- Int-subset ABI: hidden `ptr %arg_env`, `nova_user_<index>` symbols and
  `@main` entry wrapper.
- `PARITY.md` with stable IDs for every audited compiler/LSP slice.

## Incomplete work / known failures

- ABI parity is Int-only: Unit/void, strings, lists, structs and runtime calls
  still need the seed ABI.
- Codegen has no `for`/`match`, strings, collections, structs, enums, overflow
  or bounds checks.
- The test invokes clang; the Nova tool does not invoke a linker yet.
- Sema lacks modules, generics, traits, methods and full diagnostics.
- `nova build` still uses the Rust seed; no Rust crate deleted.
- No Nova LSP or stdio transport; no differential Rust/Nova harness.

## Resume here

1. Inspect the Nova host/process surface (`libraries/nova-gui/src/gui.nova`) and
   the seed CLI linking path.
2. Add a Nova tool mode that writes the `.ll` and invokes clang, with explicit
   failure diagnostics and no shell injection.
3. Add a native build/run test; keep `emit-ir` and the Rust seed intact.
4. Run codegen/parser/sema suites; update `STATE.md`, `EVIDENCE.md`,
   `TASKS.md` and `PARITY.md`; commit.

## Validation

- Command: `(cd examples/nova-tool && ../../target/debug/nova build)`
- Result: PASS
- Command: `sh examples/nova-tool/tests/codegen.sh`
- Result: PASS (15/0)

## Blockers and approvals

- No external blocker for clang invocation.
- Push, publish, deployment and branch deletion require owner authorization.
- LSP needs sema, stdio and the parity slices above.
