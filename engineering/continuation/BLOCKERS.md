# Blockers

## No external blockers for the active codegen slice

The host has the Rust seed, a working Cargo build, and Apple clang. The next
slice does not require credentials, paid services or owner decisions.

## Owner-authorization boundaries

- Do not push the branch or open/merge a pull request.
- Do not publish packages, releases or registry data.
- Do not change repository visibility.
- Do not delete `bootstrap/` or rewrite history.

## Internal dependencies (not external blockers)

- `LSP-01` needs full sema and a Nova stdio streaming primitive.
- `BOOT-02` needs a Nova CLI/codegen path that can build a nontrivial program.
- Module resolution needs a decision on flattening versus qualifying imported
  names; track it in `DECISIONS.md` when implemented.

## Known technical risks

- LLVM IR must be validated by clang, not assumed correct from string output.
- The temporary codegen ABI differs from the Rust runtime ABI; this is recorded
  in `DECISIONS.md` and must be removed before CLI integration.
- Sema's `?` type can hide unresolved errors; it is not full parity.
