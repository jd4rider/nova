---
title: Nova Wasm
description: A component framework and documentation site written in Nova, compiled to WebAssembly and rendered in the browser.
---

**Status: experimental.** Nova Wasm is a source library in `libraries/nova-wasm` and a compiler target (`NOVA_TARGET=wasm32`). The target runs; the framework is deliberately minimal. See [the live site](https://jd4rider.github.io/nova/wasm/) for the documentation and Bible reader it renders.

## The model

There is no virtual DOM, scheduler or globals. JavaScript calls the exported `main`; Nova reads its model from runtime state slots, renders HTML into the WebAssembly output buffer, and writes the next model back. The shell reads that buffer and swaps `innerHTML`. Clicks carry a `data-action` integer, which the shell stores in slot 1 before the next call. A component is three functions:

```nova
import "../../libraries/nova-wasm/src/app.nova" as app

fn view(count: Int) -> String {
    return "<button data-action=\"1\">count: " + "${count}" + "</button>"
}

fn main() {
    var count = app.get(0)
    if app.action() == 1 {
        count = count + 1
    }
    app.set(0, count)
    app.clear_action()
    app.mount(view(count))
}
```

`app.get(slot)` / `app.set(slot, value)` read and write numeric slots, `app.action()` is the pending action code, and `app.mount(html)` writes the view. The JavaScript side is [shell.js](https://github.com/jd4rider/nova/blob/main/libraries/nova-wasm/shell.js): load the module, reset the output, call `main`, decode the buffer, repeat on clicks.

## Build

```sh
cd examples/nova-wasm-site
NOVA_TARGET=wasm32 ../../bootstrap/nova build
```

`nova build` compiles Nova to wasm32 with clang, links the freestanding runtime with clang's `wasm-ld` (overridable with `NOVA_WASM_LD`) and writes `target/nova/nova-wasm-site.wasm`. Exports are `main`, `nova_state_get/set`, `nova_output_ptr/len`, `nova_reset_output` and memory.

## The site

`apps/website/public/wasm/index.html` is the whole site shell: no Astro component, no client framework. It imports `shell.js` and the module, then the Nova app renders the overview, the component guide and the Bible reader. The Bible passage is fetched by the shell from `bible-api.com` and inserted into `#passage`, because wasm exports carry numbers rather than strings; the controls and layout are Nova components.

## Limits

- **Numeric state only.** Slots hold `Int`; strings are rendered, not stored. The model cannot be a struct yet.
- **No diffing.** Each render replaces the target's `innerHTML`.
- **One exported entry.** `main` is the only Nova function called from JS.
- **No DOM access from Nova.** All DOM work is in the shell; event handling is delegated clicks on `data-action`.
- **No string input.** JS cannot hand a string to Nova, so fetched text is injected by the shell.
- **POSIX-hosted build.** The module runs anywhere; building it needs clang with wasm32 and `wasm-ld`.
- **Experimental.** The API can change, and the wasm runtime has no garbage collector.
