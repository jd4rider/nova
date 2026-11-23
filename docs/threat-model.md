# Threat model (current)

## Assets

- User source code and workspace files
- OS credentials and FreeBuff authentication state
- Local machine availability and integrity

## Trust boundaries

- **Renderer** is treated as less trusted. It has no Node access; preload exposes a narrow validated IPC surface. Context isolation and sandbox are on, CSP is set, navigation/new windows are denied.
- **Main process** mediates filesystem, LSP, tasks, search, PTYs, and menus. Paths are resolved and confined to the selected workspace. Tasks use argv arrays, not shells.
- **Compiler subprocess / Clang / Nova programs** run with user privileges. Opening a folder does not install dependencies or execute build scripts.
- **FreeBuff** is an interactive local process. No structured integration, output scraping, or automatic credential handling exists.

## Threats considered

- Renderer XSS → IPC abuse: mitigated by CSP, no Node integration, sender/payload validation.
- Symlink escape outside workspace: resolved before boundary checks. TOCTOU races remain possible.
- Malicious source or build input: the compiler must not hang on malformed input; parser recovery and CLI panic boundaries exist. Not fuzzed yet.
- Terminal escape sequences: xterm renders them; links should be treated as untrusted. Safe link handling is not fully implemented.
- Secret exfiltration: Nova Studio sends no repository content anywhere. A user-run terminal program may do anything the user can.
- Dependency compromise: lockfiles are committed, but no audit pipeline or signing exists.

## Gaps

Workspace trust gating for tasks, extension permissions, network isolation, signed updates, encrypted local conversation storage, and a full audit are not implemented. Do not treat this build as hardened for hostile workspaces.
