# Nova Boxes architecture

Nova Boxes is the package manager and registry ecosystem for Nova. A package is
a **box**; the tooling is written in Nova and exposed through `nova box`.

## Principles

- **Nova first.** Manifest parsing, SemVer, resolution and report rendering live
  in `libraries/nova-boxes` and run in-process through `nova-tool`. No package
  logic is implemented in another language.
- **Boxes, not crates.** `Box.toml` is the manifest, `Box.lock` the lockfile,
  `.box` the archive, and `nova box` the command surface. Existing `nova.toml`
  files remain readable during migration; new projects use `Box.toml`.
- **Registry as data.** The index is static files on GitHub Pages; archives are
  immutable GitHub Release assets. Reads need no account and no server.
- **Trust boundaries.** Publishing is a reviewed pull request, never an
  anonymous upload, and archives are verified by SHA-256 before extraction.

## Layers

| Layer | Location | Responsibility |
|---|---|---|
| Manifest and identity | `libraries/nova-boxes/src/boxes.nova` | `[package]`, `[bin]`, `[dependencies]`, name rules |
| SemVer | `libraries/nova-boxes/src/semver.nova` | parsing, ordering, caret/tilde/range/wildcard requirements |
| Resolution | `libraries/nova-boxes/src/resolve.nova` | path dependencies, requirement checks, cycles |
| JSON | `libraries/nova-boxes/src/json.nova` | strict RFC 8259 reader and compact writer |
| Registry | `libraries/nova-boxes/src/registry.nova` | names, sharded index records, catalogue, fetch, cache |
| Reports | `libraries/nova-boxes/src/report.nova` | `Box.lock`, tree, metadata |
| Archives | `libraries/nova-boxes/src/archive.nova` | deterministic tar + stored gzip, entry listing, safe extraction |
| Command surface | `examples/nova-tool` + `nova box` | tool modes and CLI dispatch |

## Decisions

1. **Resolution is deterministic.** Boxes are discovered depth-first in manifest
   order and deduplicated by normalized path, so identical inputs produce
   identical lockfiles.
2. **Path normalization happens before identity checks.** `a/../b` and `b` are
   the same box, which is required for cycle detection.
3. **Registry dependencies are reported, not guessed.** Until the sparse index
   client lands, a version-only dependency produces an explicit error instead
   of being silently ignored.
4. **Two manifests are accepted.** `Box.toml` wins; `nova.toml` is read for
   existing projects. There is no automatic rewrite.
5. **The index is sharded by the name's hash.** A client fetches `config.json`
   once, then one record per box at `index/v1/packages/<aa>/<bb>/<encoded>`.
   Search reads a separate `catalogue.json`; a sparse index cannot enumerate
   itself, and pretending otherwise would mean downloading every record.
6. **Reads work against a directory or HTTPS.** The same code reads a local
   mirror with ordinary file I/O and a server with `http_get`. Local reads are
   how the tests exercise the protocol without network access.
7. **No default registry URL.** Until a registry is deployed, an unset
   `NOVA_BOXES_REGISTRY` is an error rather than a guess at someone's domain.

## Planned layers

SHA-256 and raw byte I/O are host primitives (`gui.sha256`,
`gui.sha256_bytes`, `gui.read_bytes`, `gui.write_bytes`); archive layout,
compression framing, CRC32, JSON, name encoding and path ordering are Nova.
Reviewed publishing is specified in `REGISTRY_PROTOCOL.md` and tracked in
`PROGRESS.md`; it is not implemented and no command pretends otherwise.
