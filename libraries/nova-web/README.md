# nova-web

An Express/Fiber-style HTTP server framework and a Templ-style typed HTML/HTMX layer, written in Nova. Both are source libraries imported by path; there is no package to install.

```
libraries/nova-web/src/
  web.nova      Request, Response, helpers, the accept loop
  router.nova   exact-path router with stored handlers
  html.nova     escaping, elements, HTMX attributes, documents
```

## Quick start

```nova
import std.io { println }
import "../../../libraries/nova-web/src/web.nova" as web
import "../../../libraries/nova-web/src/html.nova" as html

fn handle(req: web.Request) -> web.Response {
    return web.html(html.document("Hello", "", "<h1>Hello from Nova</h1>"))
}

fn main() {
    let served = web.serve(8080, 0, |req: web.Request, state: Int| -> (web.Response, Int) {
        return (handle(req), state)
    })
    println("served ${served} requests")
}
```

## Demo

`examples/haiku` is a runnable HTMX app with in-memory state, an append-only form, hearts and a polling wall:

```sh
cd examples/haiku
../../bootstrap/nova build
./target/nova/nova-haiku
```

Open <http://127.0.0.1:8080>.

## Limits

Single-threaded and blocking, bound to loopback, HTTP/1.1 without TLS, form-encoded bodies only, exact routes only, no custom response headers, no static files, POSIX only. See the documentation pages for the full list:

- `apps/website/src/content/docs/libraries/web.md`
- `apps/website/src/content/docs/libraries/templating.md`

The server builtins (`http_listen`, `http_accept`, `http_method`, `http_path`, `http_query`, `http_body`, `http_header`, `http_form`, `http_respond`, `http_close`) live in `runtime/runtime.c` and are documented under the standard library.
