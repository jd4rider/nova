---
title: Nova Web
description: An Express/Fiber-style HTTP server framework written in Nova, with a blocking accept loop, typed request and response values, a router, and the Haiku Wall demo.
---

**Status: experimental source library.** `nova-web` lives in `libraries/nova-web` and is imported by path, not installed from a registry. The API can change. The server paths were exercised with `curl` against the demo on macOS; Windows has no socket implementation.

Nova has no package runtime and no hidden event loop. `nova-web` is the opposite of a framework you configure: it hands you the request loop, so the server is ordinary Nova code that you can read, test and change. Handlers are functions, responses are values, and application state is threaded through the loop the same way an update function threads a model in the [TUI runtime](../tui/).

## Run the demo

From the repository root:

```sh
cd examples/haiku
../../bootstrap/nova build
./target/nova/nova-haiku
```

Open <http://127.0.0.1:8080>. The Haiku Wall is a small HTMX app: post a three-line poem, heart someone else's, and watch the wall poll for new cards. The process prints the URL, keeps the wall in memory and exits on `Ctrl+C`.

```sh
curl -s http://127.0.0.1:8080/healthz          # ok
curl -s http://127.0.0.1:8080/wall             # the fragment HTMX swaps in
curl -s -X POST --data 'id=1' \
  http://127.0.0.1:8080/like                   # the card, with one more heart
curl -s -X POST --data-urlencode 'one=a line' \
  --data-urlencode 'two=another line' \
  --data-urlencode 'three=a last line' \
  http://127.0.0.1:8080/haikus                 # 201 and the new card
```

## The API

`libraries/nova-web/src/web.nova` defines two values and a loop:

```nova
struct Request {
    method: String
    path: String
    query: String
    body: String
}

struct Response {
    status: Int
    content_type: String
    body: String
}
```

| Function | Behavior |
|---|---|
| `start(port)` | Bind `127.0.0.1:port`. `true` on success, `false` when the port is taken or the platform has no sockets. |
| `next()` | Block for one request. An empty `method` means the connection ended. |
| `send(response)` | Write the status line, `Content-Type`, `Content-Length` and body, then close the connection. |
| `stop()` | Close the listener. |
| `run(state, step)` | Accept, call `step(request, state)`, send the response, keep the returned state. Returns the number of requests served. |
| `serve(port, state, step)` | `start` followed by `run`; returns `-1` when the port cannot be bound. |

Response helpers: `html(body)` → `200 text/html`, `text(body)` → `text/plain`, `json(body)` → `application/json`, `created(body)` → `201`, `no_content()` → `204`, `not_found()` → a small `404` page, `server_error(message)` → `500`, and `response(status, content_type, body)` for anything else.

Request helpers: `form(name)` (body field, or query parameter when the body is empty, percent-decoded), `header(name)`, and `is_htmx()` (the `HX-Request` header).

## A complete server

```nova
import std.io { println }
import "../../../libraries/nova-web/src/web.nova" as web

struct Visit {
    count: Int
}

fn handle(req: web.Request, visit: Visit) -> (web.Response, Visit) {
    if req.method == "GET" && req.path == "/" {
        let body = "<h1>visits: " + "${visit.count}" + "</h1>"
        return (web.html(body), Visit { count: visit.count + 1 })
    }
    return (web.not_found(), visit)
}

fn main() {
    if web.start(8080) == false {
        println("could not bind 127.0.0.1:8080")
        return
    }
    println("http://127.0.0.1:8080")
    let served = web.run(Visit { count: 0 }, |req: web.Request, state: Visit| -> (web.Response, Visit) {
        return handle(req, state)
    })
    println("served ${served} requests")
}
```

The closure captures nothing mutable. The state comes in, the next state goes out, and no global is involved — which is also why the demo is easy to reason about.

## Routing

`libraries/nova-web/src/router.nova` is an exact-path router. It stores `fn(Request) -> Response` handlers and matches on method and path in registration order:

```nova
import "../../../libraries/nova-web/src/router.nova" as router

fn health(req: web.Request) -> web.Response {
    return web.text("ok")
}

fn main() {
    var routes = router.router()
    routes = router.get(routes, "/healthz", health)
    routes = router.post(routes, "/echo", echo)
    // ...
}
```

`matches(routes, request)` tells you whether a stateless route should answer, and `dispatch(routes, request)` returns its `Response` or a `404`. In the demo, `/healthz` and `/favicon.ico` go through the router, while `/`, `/wall`, `/haikus` and `/like` are stateful and handled in the loop because they need the wall.

Routing is deliberately small: exact paths only, no parameters, wildcards, middleware chain or per-method fallbacks. Query strings are available through `Request.query` and `form`.

## How the demo uses HTMX

| Route | Method | Response | HTMX role |
|---|---|---|---|
| `/` | GET | Full document | Initial render |
| `/wall` | GET | List fragment | `hx-get="/wall"` + `hx-trigger="every 8s"` replaces the wall |
| `/haikus` | POST | One card, `201` | Form uses `hx-post`, `hx-target="#wall"`, `hx-swap="afterbegin"` |
| `/like` | POST | One card | Heart button uses `hx-post="/like?id=N"`, `hx-swap="outerHTML"` |
| `/healthz` | GET | `ok` | Plain check, routed |
| `/favicon.ico` | GET | `204` | Routed |

The form also uses `hx-on::after-request` to reset itself after a successful post. No JavaScript is written by hand; HTMX is loaded from a CDN by `html.document`, and the server answers fragments for HTMX requests without checking `is_htmx()` because each route is already a fragment route.

## Builtins underneath

The framework is a thin layer over ten native builtins. They are available to any Nova program, not just `nova-web`:

| Builtin | Result |
|---|---|
| `http_listen(port)` | `0` on success, `-1` on failure. Binds `127.0.0.1`. |
| `http_accept()` | `1` when a request was parsed, `-1` on error or when no listener is bound. Blocks. |
| `http_method()` | Request method, e.g. `"GET"`. |
| `http_path()` | Path without the query string. |
| `http_query()` | Raw query string, e.g. `"id=3&mood=calm"`. |
| `http_body()` | Request body, up to 4 MiB. |
| `http_header(name)` | Header value, case-insensitive name, or `""`. |
| `http_form(name)` | Body or query field, percent-decoded (`+` becomes a space). |
| `http_respond(status, content_type, body)` | Write a response and close the connection. |
| `http_close()` | Close the listener. |

## Limits

- **One request at a time.** The accept loop is blocking and single-threaded. There is no worker pool, event loop or keep-alive; a slow handler stalls everyone.
- **Loopback only.** `http_listen` binds `127.0.0.1`, so the server is not reachable from another machine. Change the bind address in `runtime.c` if you need that.
- **No TLS.** Plain HTTP/1.1. Terminate TLS in a proxy if you need it.
- **No custom response headers.** `http_respond` sets `Content-Type`, `Content-Length` and `Connection: close`; there is no API for `Location`, `Set-Cookie`, caching or CORS headers. Redirects and cookies need a runtime addition.
- **No request bodies beyond forms.** `http_form` understands `application/x-www-form-urlencoded` only; JSON and multipart bodies arrive as the raw `body` string. There is no streaming upload.
- **No static files.** Everything is generated in Nova; there is no filesystem reader to serve a directory from.
- **Exact paths only.** No `/haikus/:id` parameters, wildcards or regex routes.
- **State is memory only.** The wall disappears when the process exits. There is no database layer, and the runtime has no filesystem API yet.
- **POSIX only.** The Windows branch returns failure codes, so `start` is `false` and `accept` is `-1`.
- **No access log or metrics.** Add them in the loop; the demo does not.

## Verified behavior

The demo was built with the repository's `nova` binary and exercised with `curl` on macOS:

- `GET /` → `200`, full page with the HTMX script and compose form
- `GET /healthz` → `200 ok` through the router
- `POST /haikus` with an HTMX header → `201` and one card; a `<script>` line came back escaped as `&lt;script&gt;`
- `POST /like?id=1` → the card with the heart count incremented, and the new count persisted on the next request
- `GET /wall` → the fragment including the newly posted card
- `GET /nope` → `404`

The compiler side is covered by a native codegen test that compiles and runs all ten builtins outside a request; the end-to-end HTTP evidence is the demo run above.
