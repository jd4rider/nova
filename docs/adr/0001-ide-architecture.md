# ADR 0001: Desktop IDE architecture

- **Status:** Accepted for the current milestone, with an explicit spike gap
- **Date:** 2026-11-23
- **Machine:** Apple M2 Pro, macOS 27.0.1 (26A434), Apple clang 21.0.0, Rust 1.96.0, Node 26.13.0, Electron 33.3.0
- **Supersedes:** the original directive's mandatory Electron choice. Electron is now a selected candidate, not a mandated one.

## Context

Nova Studio already existed as an Electron + React + strict TypeScript + Monaco workbench before this supplemental review. The supplemental directive requires evaluating Electron, Tauri 2 and fully native Rust, with runnable spikes and measured evidence, and allows a different stack if it is stronger.

The hard gates are licensing, reliable text editing (including IME/composition), a usable keyboard/accessibility path, a credible security boundary, and an achievable path to LSP, DAP, terminal, Git and AI integration.

## Candidates and evidence

| Candidate | What exists | What was actually tested |
|---|---|---|
| Electron 33 + Monaco 0.52 | 2,457 lines of TS/TSX/CSS in `editor/src`; isolated main/preload/renderer; context isolation and sandbox; LSP client; tasks; PTY fallback | `npm run typecheck`, `npm run build`, and a 10-second production launch. Renderer/GPU/network processes were present with no workspace open. Idle RSS: **431,792 KB** across four Electron processes (main 147,744; GPU 70,656; network 45,168; renderer 168,224). A Node wrapper used by `node_modules/.bin/electron` added about 42,000 KB. No typing-to-paint or cold/warm multi-sample timing was recorded. |
| Tauri 2 | No runnable spike in this repository | **Not tested.** Webview dependency, PTY, IME, menu accessibility and distribution risk remain unmeasured. |
| Fully native Rust (egui/eframe, iced, Floem, GPUI) | No editor spike or reusable editor assessment beyond documentation research | **Not tested.** Text shaping, IME, accessibility, editor maturity and licensing remain unproven. |

### Additional measurements

- Source build output (`editor/out`): **8.9 MB**.
- Electron runtime directory: **253 MB** after installing the official v33.3.0 darwin-arm64 archive.
- `editor/node_modules`: **451 MB** (development dependency tree, not an installer).
- Installer size: macOS arm64 DMG **114 MB**; zipped app **110 MB**; Windows x64 NSIS installer **94 MB**. All are unsigned.
- Cold/warm startup p50/p95, typing-to-paint, scrolling, 1 MB/10 MB file open and completion latency: **not measured**.
- Studio launch logged a missing `node-pty` native module; the app continued because PTY is optional, but the terminal is pipe-only until that module is built. FreeBuff terminal requires a PTY.

The Electron binary installer extracted only license files, so the runtime archive was downloaded directly from the official Electron v33.3.0 GitHub release for this measurement. This is a local dependency installation, not a release or deployment.

## Rubric

Scores are 0–5. Weighted score = Σ(weight × score / 5). Missing spikes are scored conservatively but cannot be treated as passing evidence.

| Criterion | Weight | Electron | Tauri 2 | Native Rust |
|---|---:|---:|---:|---:|
| Editor correctness, maturity, coverage | 25 | 5 | 5 | 2 |
| Responsiveness, resources, startup | 20 | 3 | 4 | 5 |
| Maintenance cost and implementation risk | 15 | 4 | 3 | 2 |
| LSP/DAP/terminal/AI integration | 15 | 5 | 4 | 3 |
| Cross-platform and distribution | 10 | 4 | 3 | 2 |
| Accessibility and input compatibility | 10 | 4 | 3 | 2 |
| Extensibility and ecosystem | 5 | 5 | 4 | 3 |
| **Weighted total** | **100** | **85** | **78** | **56** |

Tauri's 78 is optimism without a runnable prototype: Monaco would supply the editor, but webview differences, PTY/IME behavior and packaging are unverified. The native score reflects an immature reusable editor substrate and a much larger implementation path, not a claim that native Rust cannot work.

## Decision

**Retain the existing Electron workbench.** Do not restart the compiler, discard the editor, or migrate to another shell in this milestone.

Reasons:

1. Monaco/Chromium gives the strongest immediate path through the editor-correctness and input hard gates: Unicode editing, composition/IME, selections, clipboard, undo, multi-cursor and accessibility primitives are already implemented and maintained.
2. The existing process boundary is real and testable: renderer has no Node access, IPC is narrow, tasks use argv arrays and workspace paths are confined.
3. LSP, tasks, terminal, search and Vim integrations already exist; a rewrite would add risk without a demonstrated user benefit.
4. Tauri and native alternatives have no runnable spike and therefore cannot prove the hard gates. Choosing them on novelty would violate the directive's "evidence, not novelty" rule.

The Electron idle RSS (≈422 MiB for four processes) is a real cost and a revisit trigger, not something to hide.

## Consequences

- Ship Chromium/Electron's memory and installer footprint until measured otherwise.
- Track Electron security updates and rebuild native dependencies per platform.
- `node-pty` is optional, but without its native build the terminal is pipe-only and FreeBuff's TUI is unavailable. This is surfaced in the UI.
- The directive's required Tauri/native spikes remain unfinished work; the architecture decision is provisional until they are time-boxed and measured.

## Migration plan if the decision changes

1. Keep `bootstrap/*` independent of the desktop shell; expose only versioned CLI/LSP/DAP contracts.
2. Preserve the Monaco renderer behind a typed host bridge; Tauri can reuse it if a spike proves IME, PTY, menu and accessibility behavior.
3. Move the selected app to `apps/studio` only when migration is real; do not churn the working `editor/` tree for layout aesthetics.
4. Require a candidate to pass the same fixture matrix: Unicode/IME, undo, large files, diagnostics, completion, Vim, PTY resize/termination, diff review, packaging and measured startup/RSS.

## Revisit triggers

- Electron idle RSS exceeds 600 MB after a representative workspace is open, or cold startup exceeds 2.5 s on documented hardware.
- A Tauri 2 spike passes the full input/PTY/accessibility/editor matrix and reduces idle RSS or installer size by at least 30% with no critical feature loss.
- A native Rust editor substrate passes editor correctness and IME/accessibility tests at production quality.
- Electron blocks a required feature (for example a supported DAP/terminal integration) that another stack demonstrably provides.

## Evidence paths

- `editor/src/main/index.ts`, `editor/src/preload/index.ts`, `editor/src/renderer/src/App.tsx`
- `editor/package.json`, `editor/electron.vite.config.ts`
- `docs/adr/0006-electron-security.md`
- `apps/website/src/content/docs/studio/overview.md`
- `ROADMAP.md`

## Unresolved spike work

This ADR must be revised after runnable Tauri 2 and native Rust prototypes are built and measured. Until then, "retain Electron" is a risk-managed decision, not proof that Electron is the best possible architecture for every future workload.
