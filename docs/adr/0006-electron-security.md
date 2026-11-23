# ADR 0006: Electron process model and security

## Status

Accepted.

## Decision

- Main process owns filesystem, child processes, PTYs, LSP, search, and menu commands.
- Preload exposes a narrow typed API through `contextBridge`; no arbitrary `exec`, raw `fs`, or shell strings.
- `contextIsolation: true`, `sandbox: true`, `nodeIntegration: false`.
- CSP restricts scripts to self; navigation and window creation are denied.
- IPC handlers validate sender frame and payload shape; filesystem paths resolve symlinks and must remain inside the selected workspace, except OS "reveal" operations.
- Tasks use argv arrays, no shell concatenation, bounded output, cancellation, and process-group kill.
- Renderer keeps one Monaco model per URI and disposes closed models.

## Consequences

Renderer compromise does not directly grant Node execution. Workspace trust does not yet gate tasks, because trust prompts and extension execution are not implemented; this is a documented gap. Opening a folder runs no dependency installation or build scripts.
