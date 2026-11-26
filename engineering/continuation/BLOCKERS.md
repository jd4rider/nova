# Blockers

## No external blockers for the active parity slice

The host has the checked-in Nova bootstrap and Apple clang. The next slice does
not require credentials, paid services or owner decisions.

## Owner-authorization boundaries

- Do not push the branch or open/merge a pull request without explicit approval.
- Do not publish packages, releases or registry data.
- Do not change repository visibility.
- Do not rewrite history.

## Internal dependencies (not external blockers)

- `COMP-01.04` needs generics, traits, methods and closures in the Nova checker
  and codegen.
- `LSP-01` needs the fuller semantic API for rename, references and code
  actions; the current Nova server covers diagnostics, completion, hover,
  definition, symbols and formatting.
- `REL-01` needs bootstrap binaries for Linux and Windows; the checked-in
  `bootstrap/nova` is macOS arm64 only.
- The playground needs a Nova wasm backend before its checked-in `.wasm`
  artifacts can be rebuilt.
