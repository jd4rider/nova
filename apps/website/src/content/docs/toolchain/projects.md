---
title: Projects and dependencies
description: Box.toml, Box.lock and how path and registry dependencies reach a build.
---

## Layout

```text
my-project/
  Box.toml
  Box.lock
  src/main.nova
  tests/
  target/
```

`Box.toml` is the Nova Boxes manifest. `nova.toml` is still accepted for older
projects; `Box.toml` wins when both exist.

```toml
[package]
name = "my-project"
version = "0.1.0"

[bin]
path = "src/main.nova"

[dependencies]
util = { path = "../util" }
"@nova/reglib" = "^1.0.0"
```

A dependency is imported by its declared name, using the dependency's own
`[bin] path` as the entry file:

```nova
import "util"
import "@nova/reglib"
```

The manifest parser is `bootstrap/nova-pkg`; package behavior itself lives in
Nova in `libraries/nova-boxes`.

## Commands

```sh
nova box add '@nova/reglib@^1.0.0'   # edit Box.toml
nova box remove '@nova/reglib'
nova box update                       # resolve path and registry deps into Box.lock
nova box fetch '@nova/reglib'         # verify and unpack into the cache
nova box search math
nova box info '@nova/reglib'
nova box tree / check / lock / metadata
nova box pack / verify / unpack
```

## Lockfile and imports

`nova box update` writes `Box.lock` with the chosen version, source and SHA-256
for every registry box. `nova build` reads that pin and the unpacked copy under
`NOVA_BOXES_CACHE` (default `~/.nova/boxes`), so compilation never performs a
network request. `nova build --locked` requires the lockfile to exist.

Only direct dependencies are added to the compiler's import map. Transitive
boxes are recorded in the lock but must be imported through a direct
dependency's own source.

## Registry

The registry is static data: `config.json`, `index/v1/catalogue.json` and one
record per box under `index/v1/packages/<aa>/<bb>/<publisher>/<box>.json`.
Reads work against a local mirror (`/path` or `file://`) or an HTTPS root set
with `NOVA_BOXES_REGISTRY`. There is no default registry URL because no
first-party registry is deployed yet.

`NOVA_BOXES_OFFLINE=1` serves previously cached records and refuses anything
that was never fetched; archive downloads still need the network or a mirror.

## Security

- Fetched archives are verified by SHA-256 before extraction, and paths are
  checked against the destination.
- No signature or provenance verification exists, and publishing is not
  implemented.
- Folder open does not install dependencies, and there are no registry build
  scripts.
- Checksums establish integrity, not authorship.

## Planned

Reviewed publishing (a pull request validated by CI, rebuilt from the reviewed
commit), transitive import maps, cache refresh and signed provenance are future
work. See the [status page](../../status/).
