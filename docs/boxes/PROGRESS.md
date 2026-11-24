# Nova Boxes progress

Status is evidence-based: a milestone is only marked done when its tests pass.

| Milestone | State | Evidence |
|---|---|---|
| Manifest model and name validation | done | `tests/boxes.sh` |
| SemVer parse/order/requirements | done | `tests/boxes.sh` (9 cases) |
| Path dependency resolution | done | nested app → util → math |
| Cycle detection and normalization | done | c1 ↔ c2 fixture |
| Deterministic lockfile, tree, metadata | done | lock compared byte-for-byte |
| `nova box` CLI dispatch | done | tree/check/lock/update/metadata/resolve/pack/verify |
| JSON reader and writer | done | compact round trip against Python's `json` |
| Sparse index client | done | sharded record lookup, search, info; local and HTTP reads |
| Manifest editing | done | `add`/`remove` keep other lines and comments |
| Registry-backed `update` | done | `Box.lock` records versions, sources and SHA-256 |
| `.box` archive pack/verify/unpack | done | gzip/tar/SHA-256/traversal tests |
| Cache and `fetch` | done | SHA-256 verified fetch unpacks into the cache |
| Offline mode | partial | local mirrors work; no `--offline` flag yet |
| Registry publishing (reviewed PR) | not started | — |
| Studio and language-server integration | not started | — |
| Website catalogue | not started | — |

## What works today

```
nova box tree         # dependency graph for the current project
nova box check        # validate manifest, requirements, cycles
nova box lock         # write/print Box.lock
nova box metadata     # machine-readable graph
nova box pack [OUT]   # deterministic .box archive (default Box.box)
nova box verify [FILE] # list entries and print SHA-256
nova box unpack [ARCHIVE] [DIR]  # extract with CRC and path checks
nova box search [QUERY]          # catalogue search
nova box info NAME               # versions, dependencies, checksums
nova box fetch NAME[@REQ]        # download, verify and unpack into the cache
nova box add NAME[@REQ]          # add or replace a dependency in Box.toml
nova box remove NAME             # remove a dependency from Box.toml
nova box update                   # resolve path and registry deps into Box.lock
```

Path dependencies are resolved from `Box.toml`/`nova.toml`; version-only
dependencies are still rejected by local resolution because fetching requires a
registry record. Registry commands read a mirror:

```
NOVA_BOXES_REGISTRY=/path/to/mirror nova box info @nova/app
NOVA_BOXES_REGISTRY=https://host/index-root nova box search math
NOVA_BOXES_CACHE=$HOME/.nova/boxes nova box fetch @nova/app@^1.0.0
```

A local mirror is a directory (or `file://` URL) laid out exactly like the
served index. There is no default registry URL because no first-party registry
is deployed yet; an unset variable produces an actionable error instead of a
guess.

## Next

1. Compiler import resolution from fetched boxes (`nova build` currently reads
   `nova.toml`, refuses dependencies and does not consult `Box.lock`).
2. Yarn-style yank warning when an existing lock pins a version the index now
   marks yanked.
3. Nova Studio box panel and language-server completion for registry names.
4. Website catalogue page generated from the same index.
5. Reviewed publishing workflow, after the repository and Pages exist.
