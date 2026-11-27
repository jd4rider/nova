# Nova Terminal blockers

## No external blocker for the active parser slice

The host has macOS ARM64, Apple clang, the Nova bootstrap seed and a current
Nova tool build. The next slice needs no credentials or owner decision.

## Platform and tooling limits

- **Single-window GUI host.** `runtime/gui.m` opens one window per process, so
  TERM-08.003 multiple windows is blocked until the host grows a window model.
- **No mouse reporting yet.** The host reports mouse events, but the terminal
  does not emit X10/SGR mouse sequences; this is implementation work, not a
  platform blocker.
- **Compiler surface.** `graphemes()` is not implemented in codegen, aggregate
  `push` cannot write a grown header back through a parameter, and string
  slicing is absent. The terminal uses `chars()`, fixed rings and a join helper
  instead. These are recorded limitations, not hidden fallbacks.
- **Bootstrap seed lag.** `bootstrap/nova` cannot build imported-struct fields.
  Use the current `examples/nova-tool/target/nova/nova-tool`.
- **No signed-release credentials.** Developer ID signing and notarization
  cannot be performed here; the local bundle is ad-hoc signed only.
- **GUI interaction automation is scripted, not native.** The harness drives
  the app through `NOVA_TERM_SCRIPT`; it does not inject OS-level key or mouse
  events.

## Owner-authorization boundaries

- Do not push, publish, deploy or delete branches without explicit approval.
- Do not claim terminal compatibility that has not been exercised here.
