---
title: Plugins
description: Add commands to Nova Studio from a manifest in the workspace, and how the plugin hook is built.
---

Nova Studio reads plugins from `<workspace>/plugins`. Drop a `.plugin` manifest into that folder, reopen the workspace with **File: Open Folder…** (`⇧⌘O`), and the command appears in the palette (`⇧⌘P`) as **Plugin: …**.

## Write one

`plugins/tree.plugin`:

```
id=tree
label=List workspace files
command=find . -maxdepth 2 -type f -name '*.nova' | head -40
```

| Field | Required | Meaning |
|---|---|---|
| `id` | yes | Unique id; the palette key is `plugin:<id>`. |
| `label` | no | What the palette shows. Defaults to `id`. |
| `command` | yes | Shell command, run with the workspace as the working directory. |

Select it in the palette and it runs like Build does: the command, its output and its exit code land in the Output panel, and the status bar reports the exit code. Manifests are plain `key=value` lines; blank lines, unknown keys and `#` comments are ignored because only the three known keys are read.

Two working examples ship in the repository's own `plugins/` folder: `format.plugin` runs `nova fmt`, `tree.plugin` lists Nova sources.

## How the hook works

The whole implementation is in `examples/nova-studio/src/main.nova`:

- `load_plugins(root)` lists `<root>/plugins`, reads every `*.plugin` file and parses it with `manifest_value`, returning `List<Plugin>`.
- `rows(app)` appends one palette row per plugin, keyed `plugin:<id>`.
- `run_command` matches that prefix, looks the plugin up, and runs `gui.run(command, app.root)`, reporting `gui.exit_status()`.
- The list reloads whenever a workspace is opened, so adding a manifest does not require restarting the app.

Because it is the same `gui.run` used by Build, a plugin can drive any CLI tool: a formatter, a linter, a generator, a test runner.

## Limits, stated plainly

- **Commands only.** A plugin cannot add panels, gutter decorations or editor behaviour yet; those need the workbench source.
- **No sandbox.** The command runs with your user account and the workspace as cwd. Only install manifests you have read.
- **Workspace scoped.** Plugins are loaded from `<workspace>/plugins`, not from a global folder.
- **Reload on open.** Editing a manifest takes effect the next time its workspace is opened.

## Extending the workbench itself

For new UI, the workbench is a normal Nova program: add a file under `examples/nova-studio/src`, import it, and draw with [nova-gui](../../libraries/gui/). The plugin manifest hook is deliberately the small, stable surface for command contributions.
