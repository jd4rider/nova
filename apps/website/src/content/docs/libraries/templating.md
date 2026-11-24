---
title: Templates and HTMX
description: Templ-style typed HTML in Nova, with escaping, component functions, HTMX attribute helpers and a server that answers fragments.
---

**Status: experimental source library.** `libraries/nova-web/src/html.nova` is a typed HTML layer, not a template-file compiler. The API can change.

Templ's model is not a separate markup language: templates are functions in the host language, checked by its compiler, that return markup. `nova-web` takes the same route. A template is a Nova function that returns a `String`; components are ordinary functions you can call, test and compose. There is no `.templ` file format, no template parser and no runtime template engine — which also means no untyped context map and no way to interpolate a value without going through `escape`.

## Escaping by default

```nova
import "../../../libraries/nova-web/src/html.nova" as html

let line = html.node(user_input)     // escaped text node
let attrs = html.attr("class", "card") // escaped attribute value
let markup = html.raw(trusted_html)  // opt out, deliberately loud
```

`escape` replaces `&`, `<`, `>`, `"` and `'`. `node` is the readable name for an escaped text node. `raw` does not escape and should only receive markup your code built.

## Building elements

| Function | Result |
|---|---|
| `el(tag, attrs, children)` | `<tag attrs>children</tag>` |
| `empty_el(tag, attrs)` | `<tag attrs>` for void elements |
| `attr(name, value)` | ` name="value"` with the value escaped |
| `flag(name)` | ` name` for boolean attributes |
| `node(value)` | Escaped text |
| `escape(value)` | Escaped `String`, when you need it directly |
| `raw(markup)` | Trusted markup, unchanged |

A component is just a function with a meaningful name:

```nova
fn card(author: String, line: String, mood: String) -> String {
    var head = html.el("span", html.attr("class", "by"), html.node("@" + author))
    head = head + html.el("span", html.attr("class", "mood"), html.node(mood))
    var body = html.el("p", "", html.node(line))
    return html.el("article", html.attr("class", "card"), head + body)
}
```

Names are resolved in a flat namespace, so component names are global across the program. Prefixing them (`card`, `wall_stats`, `haiku_form`) avoids collisions with helpers in `web.nova` — `text` and `page` are the two names that had to be renamed during development.

## HTMX is first class

HTMX is not bolted on as arbitrary attributes. The helpers name the request, target and swap explicitly:

```nova
let like = html.hx_post("/like?id=7", "#haiku-7", "outerHTML")
let feed = html.hx_get("/wall", "#wall", "innerHTML") + html.hx_trigger("every 8s")
let confirm = html.hx_confirm("really send this?")
```

`hx_get(url, target, swap)` and `hx_post(...)` produce `hx-get`/`hx-post`, `hx-target` and `hx-swap` in one string, so a typo in the swap strategy is caught by reading one call rather than three attributes. `hx_trigger(value)` and `hx_confirm(message)` cover the common extras. Any other attribute can be added with `attr`.

On the server, `web.is_htmx()` reads `HX-Request`. The Haiku Wall does not need it because every fragment route is separate (`/wall`, `/haikus`, `/like`), but a page that serves both a full document and a fragment can branch on it.

## A whole page

`html.document(title, head, body)` produces the doctype, meta tags, title, the HTMX script tag and the body:

```nova
fn page(wall: Wall) -> web.Response {
    var body = "<h1>Nova Haiku Wall</h1>"
    body = body + compose()
    body = body + "<section id=\"wall\" " + html.hx_get("/wall", "#wall", "innerHTML") + ">"
    body = body + wall_list(wall) + "</section>"
    return web.html(html.document("Nova Haiku Wall", "<style>" + styles() + "</style>", body))
}
```

The `head` argument is raw markup, which is where a stylesheet or extra script belongs. The demo in `examples/haiku` is the worked example of the whole stack: typed components, HTMX attributes, fragment routes and `web.html` responses.

## Limits

- **No template files or compiler.** Templates are Nova functions; there is no `.templ`, no markup parser and no generated code.
- **No control flow inside markup.** Use `for` and `if` in the surrounding function and concatenate the result. There are no template-level loops, conditionals or slots.
- **Strings are the only output type.** There is no attribute builder type, no element tree and no streaming renderer; a component returns a `String`.
- **Escaping is string replacement.** It covers the five HTML-significant characters; it does not parse HTML or sanitize attributes beyond quoting.
- **Flat namespace.** Two modules cannot both define `text` or `page`; the compiler rejects duplicate function names.
- **HTMX loads from a CDN.** `document` points at `unpkg.com/htmx.org@2.0.4`, so the demo needs network access for the interactive parts. The server and the HTML are fine offline; the HTMX behaviours are not.
- **No client-side build step.** There is no bundler, minifier or asset pipeline — only strings.

## Verified behavior

The demo's page was fetched from the running server and checked for the HTMX script tag, the compose form's `hx-post="/haikus"`, and the wall's polling attributes. A posted line containing `<script>alert(1)</script>` came back as `&lt;script&gt;alert(1)&lt;/script&gt;` inside the card, and the like button returned a replacement card with the incremented heart count.
