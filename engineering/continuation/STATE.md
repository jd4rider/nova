# Current Nova checkpoint

Updated: 2026-11-24
Master directive: owner-supplied; requirements and parity matrix tracked in
`engineering/continuation/`.
Repository / branch: `nova` / `selfhost/pure-nova`
HEAD: control-flow checkpoint (see `git log`); previous checkpoint `0fa226f`.
Working tree: self-hosting files have uncommitted control-flow/parity changes;
pre-existing unrelated `editor/` and playground changes remain untouched.

Active milestone: `CODEGEN-01` — Nova LLVM IR lowering
Active task: `CODEGEN-01.04` — runtime ABI alignment
Status: READY

## Last verified behavior

- `sh examples/nova-tool/tests/codegen.sh` → passed 15, failed 0.
  - `if`/`else`, `while`, comparisons and branches lower to LLVM IR.
  - The loop/branch native binary exits 17.
  - Immutable params/`let` stay in SSA; mutable bindings use slots.
- `sh examples/nova-tool/tests/parser.sh` → passed 50, failed 0.
- `sh examples/nova-tool/tests/sema-ast.sh` → passed 9, failed 0.
- `sh examples/nova-tool/tests/sema.sh` → passed 4, failed 0.
- Artifact is Mach-O on this Apple Silicon host.

## Work completed since previous checkpoint

- `libraries/nova-codegen/src/llvm.nova`: `if`/`else`, `while`, comparisons,
  branch labels, SSA immutable bindings.
- `tests/codegen-flow.nova` and four additional codegen assertions.
- `engineering/continuation/PARITY.md`: compiler and LSP parity matrix with
  stable task IDs.
- Audit confirmed Rust LSP scope from `bootstrap/nova-lsp/src/lib.rs`.

## Incomplete work / known failures

- Codegen has no `for`/`match`, strings, bools, structs, enums, collections,
  overflow/bounds checks or runtime ABI parity.
- The test invokes clang; the Nova tool does not.
- Sema does not resolve modules, generics, traits, methods or full diagnostics.
- `nova build` still uses the Rust seed; no Rust crate deleted.
- No Nova LSP exists; stdio transport is not implemented.
- No differential Rust/Nova harness yet.

## Resume here

1. Start `CODEGEN-01.04`: inspect the Rust entry/function ABI and match the
   hidden environment pointer plus entry wrapper for the supported Int subset.
2. Add ABI assertions or native tests; do not remove the temporary ABI until
   the new path passes.
3. Run codegen/parser/sema suites; update `STATE.md`, `EVIDENCE.md` and
   `TASKS.md`; commit.
4. After ABI, implement `CODEGEN-01.09` (clang invocation) and `CLI-01.01`,
   then module resolution and the LSP transport.

## Validation

- Command: `(cd examples/nova-tool && ../../target/debug/nova build)`
- Result: PASS
- Command: `sh examples/nova-tool/tests/codegen.sh`
- Result: PASS (15/0)

## Blockers and approvals

- No external blocker for ABI/codegen work.
- Push, publish, deployment and branch deletion require owner authorization.
- LSP work awaits sema, a stdio primitive and the parity slices above.
