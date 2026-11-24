# Nova continuation entry point

This directory is the durable project memory for the Nova self-hosting work.
It is version-controlled and contains no secrets.

## Master directive

The owner supplied the Self-Hosted and Self-Sustaining Ecosystem Master
Directive and the Session Continuation Protocol in FreeBuff sessions on
2026-11-24. The directive is not stored verbatim here. Its technical
requirements are tracked in `ROADMAP.md` and `TASKS.md`; the existing evidence
inventory is `docs/rust-migration.md` and `docs/engineering/ecosystem-audit.md`.

## Repository map

- Repository: `nova`
- Self-hosting branch: `selfhost/pure-nova`
- Main compiler seed: `bootstrap/` (Rust, stage 0)
- Nova libraries: `libraries/`
- Nova tool executable: `examples/nova-tool`
- Migration plan: `docs/rust-migration.md`
- Compiler/LSP audit: `docs/engineering/ecosystem-audit.md`

Do not push, publish, deploy, change visibility or delete the Rust seed without
owner authorization.

## How to resume

1. Read `STATE.md`, then `ROADMAP.md`, `TASKS.md`, `DECISIONS.md`,
   `BLOCKERS.md` and `EVIDENCE.md`.
2. Run `git status`, `git branch --show-current` and `git log --oneline -3`.
   Preserve unrelated working-tree changes.
3. Verify the checkpoint with the smallest relevant test before editing.
4. Continue the active task, or select the highest-priority READY task whose
   prerequisites are satisfied.
5. Update `STATE.md`, `TASKS.md` and `EVIDENCE.md` before the session ends.

Persistence is not automatic. On a new session, attach or reference
`AGENTS.md` (or this file) so the agent reads it.

## Current owner command

When the owner says `keep going`, resume from `STATE.md`; do not restart the
project or ask for context that is already recorded here.
