# Current Nova checkpoint

Updated: 2026-11-24
Master directive: owner-supplied; parity matrix in `engineering/continuation/`.
Repository / branch: `nova` / `selfhost/pure-nova`
HEAD: build-ir checkpoint (see `git log`); prior `74124f8`.
Working tree: self-hosting files have the uncommitted build-ir checkpoint;
unrelated `editor/` and playground changes remain untouched.

Active milestone: `CODEGEN-01` — Nova LLVM IR lowering
Active task: `CODEGEN-01.04` — finish Int/Unit/string ABI parity
Status: PARTIAL

## Last verified behavior

- `sh examples/nova-tool/tests/codegen.sh` → passed 17, failed 0.
  - `build-ir` writes IR, invokes clang with quoted paths and checks status.
  - Native binaries exit 9, 2 and 17.
  - `ptr %arg_env`, `nova_user_<index>` and the `@main` wrapper are emitted.
  - Immutable params/`let` stay in SSA.
- `sh examples/nova-tool/tests/parser.sh` → passed 50, failed 0.
- `sh examples/nova-tool/tests/sema-ast.sh` → passed 9, failed 0.
- `sh examples/nova-tool/tests/sema.sh` → passed 4, failed 0.
- Mach-O confirmed on this Apple Silicon host.

## Work completed since previous checkpoint

- `CODEGEN-01.09`: `NOVA_TOOL=build-ir` invokes host clang and produces a
  native binary; the test executes the Nova-built binary.
- Shell-safe single-quote path escaping in the tool.
- Int ABI shape and control-flow lowering from the previous checkpoint.

## Incomplete work / known failures

- ABI parity is Int-only: Unit/void, strings, lists, structs and runtime calls.
- Codegen has no `for`/`match`, strings, collections, structs, enums, overflow
  or bounds checks.
- `nova build` still uses the Rust seed; the Nova tool is not yet the CLI.
- Sema lacks modules, generics, traits, methods and full diagnostics.
- No Nova LSP or stdio transport; no differential Rust/Nova harness.

## Resume here

1. Extend `CODEGEN-01.04`: inspect the seed's Unit/void and string ABI, then
   implement Unit returns and string literals without claiming full parity.
2. Add native tests for a Unit function and a string result.
3. Run codegen/parser/sema suites; update `STATE.md`, `EVIDENCE.md`,
   `TASKS.md` and `PARITY.md`; commit.
4. Then `CODEGEN-01.06` collections/structs, `CODEGEN-01.07` checks, and
   `CLI-01.01`.

## Validation

- Command: `(cd examples/nova-tool && ../../target/debug/nova build)`
- Result: PASS
- Command: `sh examples/nova-tool/tests/codegen.sh`
- Result: PASS (17/0)

## Blockers and approvals

- No external blocker for ABI work.
- Push, publish, deployment and branch deletion require owner authorization.
- LSP needs sema, stdio and the parity slices above.
