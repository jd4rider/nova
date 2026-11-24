# Nova Boxes progress

Status is evidence-based: a milestone is only marked done when its tests pass.

| Milestone | State | Evidence |
|---|---|---|
| Manifest model and name validation | done | `tests/boxes.sh` |
| SemVer parse/order/requirements | done | `tests/boxes.sh` (9 cases) |
| Path dependency resolution | done | nested app → util → math |
| Cycle detection and normalization | done | c1 ↔ c2 fixture |
| Deterministic lockfile, tree, metadata | done | lock compared byte-for-byte |
| `nova box` CLI dispatch | done | tree/check/lock/metadata/resolve/pack/verify |
| Sparse index client | not started | — |
| `.box` archive pack/verify/unpack | done | gzip/tar/SHA-256/traversal tests |
| Registry publishing (reviewed PR) | not started | — |
| Cache, fetch, offline mode | not started | — |
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
```

Path dependencies are resolved from `Box.toml`/`nova.toml`; version-only
dependencies are reported as unsupported rather than silently dropped.

## Next

1. Sparse index reader and `nova box search/info`.
2. `.box` archive writer/verifier with path-safety tests.
3. Cache + `fetch`, then compiler import resolution from fetched boxes.
