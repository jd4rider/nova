# Nova Studio guide

## Start

```sh
cd editor
NOVA_BIN=/absolute/path/to/nova npm run dev
```

Or build: `npm run build` (outputs under `out/`). `npm run typecheck` is part of the build gate. The Electron binary is installed locally by `node_modules/electron/install.js` if npm blocked postinstall scripts.

Open a folder containing `nova.toml`. Without a manifest, files open and syntax highlight but LSP startup and tasks are reported as unavailable.

## Shortcuts

| Action | Shortcut |
|---|---|
| Quick open | Cmd/Ctrl+P |
| Command palette | Cmd/Ctrl+Shift+P |
| Save | Cmd/Ctrl+S |
| Toggle bottom panel | Cmd/Ctrl+` |
| Quick open / palette navigation | Up/Down, Enter, Escape |

Menu commands cover open folder, save, build, run, check, test, format, new file/shell, FreeBuff terminal, sidebar/terminal/docs toggles, Vim and visual-effect toggles, and settings. Commands are also available in the palette.

## Editor

Monaco provides multi-cursor, rectangular selection, undo/redo, folding, bracket colorization, indentation guides, minimap, find/replace, and snippets through its standard keybindings. Nova LSP supplies diagnostics, completion, hover, definition and document formatting. Format-on-save is not enabled by default. External file changes are detected at save time: Nova asks before overwriting.

One model is kept per `file://` URI; closing a tab disposes it. Supported highlighting currently centers on `.nova`; other files open as plain text. Markdown preview, split editor groups, drag-reordered tabs, breadcrumbs, inlay hints, and a test explorer are not implemented.

## Vim mode

Settings → Vim mode. Uses `monaco-vim`, which is not Neovim.

Working: NORMAL/INSERT/VISUAL modes, motions, counts, operators, registers, search, marks, repeat, leader mappings supported by monaco-vim, and ex commands `:w`, `:q`, `:wq`, `:e`, `:set number` (toggles line numbers). Escape dismisses Vim state; dirty close/save prompts use normal confirmations.

Limitations: no real Neovim, no guaranteed LSP-completion key behavior in every mode, incomplete text objects/macros/plugins, and no tested leader menu for files/search/build/tests/AI. Keybindings can conflict with Monaco. Cursor shape is terminal-controlled by monaco-vim and not independently configured. Report additions only after testing.

## Documentation tab

The bottom panel includes a **Docs** tab with an offline quick reference for the language, Studio, CLI and build process, plus the public documentation URL. Open it from **View → Documentation** or the command palette. The canonical, searchable docs live at <https://jd4rider.github.io/nova/docs/>; the embedded panel is intentionally a compact reference, not a browser or a scraped copy of the site.

## Visual effects and sound

Settings → **Visual effects & sound** contains independent toggles for animated effects and synthesized sound, plus a volume slider. Preferences persist in `nova.settings` in local storage. The same toggles are available under View and in the command palette, and the status bar shows `FX` and `SFX` state.

- **Current-line lens:** a dock-style lifted plate and enlarged line number follow the cursor, including cursor movement from Vim motions. Sparkles appear at the caret while typing and drift across the viewport while scrolling.
- **Sound:** short tones are synthesized locally with Web Audio. No audio files, network requests or telemetry are used. Sound is off by default; effects are on by default.
- **Accessibility:** OS `prefers-reduced-motion` suppresses animated effects even when the setting is enabled. The setting remains stored so it can resume when that preference is removed.

Monaco renders text as DOM, and the lens intentionally magnifies the line plate and gutter without reflowing glyphs. That keeps caret, selection and hit-test geometry stable; it is a visual lens, not a per-line font-size change. The visual layer is pointer-transparent and never captures editor input.

## Terminal and tasks

Build/Run/Check/Test execute `nova` with argv arrays in the workspace. Output is bounded and shown verbatim; failures show the actual exit code. Cancel kills the process group. Node-pty enables a real PTY; without a compiled `node-pty`, the terminal reports **pipe mode** and lacks a TTY. That limitation is shown in the UI and status bar rather than hidden.

## Security

Context isolation and sandbox are enabled, Node integration is disabled, CSP is applied, navigation/new windows are blocked, and filesystem paths are confined to the workspace after symlink resolution. Renderer cannot request arbitrary process execution. Workspace trust prompts and extension execution are not implemented; do not open untrusted workspaces and expect a sandbox boundary around build tasks.
