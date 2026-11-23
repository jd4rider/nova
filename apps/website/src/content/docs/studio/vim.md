---
title: Vim mode
description: What monaco-vim supports in Studio, and what it does not.
---

Vim mode is an optional toggle in Settings or the View menu. Studio uses `monaco-vim` 0.4.2. It is **partial Vim emulation**, not Neovim and not a compatibility claim.

## Supported

- NORMAL, INSERT and VISUAL modes as provided by monaco-vim
- Motions, counts, operators, registers, search, marks and repeat supported by the library
- Leader mappings supported by monaco-vim
- `:w`, `:q`, `:wq`, `:e`
- `:set number` toggles absolute line numbers
- Save/close commands use the same dirty-buffer confirmation as the rest of Studio
- Escape leaves Vim mode; completion UI and OS shortcuts may take focus first

## Not verified or not supported

- Full VISUAL LINE/BLOCK behavior across every motion
- Text objects and macros beyond the library's implementation
- A tested leader menu for files, search, build, tests, diagnostics or AI
- Guaranteed LSP completion-key behavior in every mode
- Configurable cursor shape independent of the terminal
- Full Neovim plugin compatibility

Do not rely on Vim mode for destructive operations without checking the buffer state. A real Neovim integration is a future option, not a current feature.
