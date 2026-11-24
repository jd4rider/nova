# Nova Boxes registry protocol (design)

**State: the read side is implemented** in `libraries/nova-boxes/src/registry.nova`
(`search`, `info`, `fetch`, checksum verification, cache) and tested against a
filesystem mirror. Publishing is still design only: no workflow uploads or
indexes anything yet.

## Transport

- HTTPS only. Reads are unauthenticated.
- Sparse index: a client fetches `config.json` once, then one record per
  package. It never downloads the whole index.
- The index is static files served from GitHub Pages. Archives are GitHub
  Release assets addressed by immutable URLs recorded in the index.

## `config.json`

```json
{
  "protocol_version": 1,
  "index": "https://OWNER.github.io/nova-boxes/index/",
  "publishing": "github-pr",
  "capabilities": ["sparse-index", "sha256", "yank"]
}
```

A client that does not know `protocol_version` must stop with an actionable
error rather than guessing.

## Catalogue

Path: `index/v1/catalogue.json`. Search has to read something that lists names;
a sparse index by definition cannot enumerate itself. The catalogue is a
convenience view, not an authority: `info` and `fetch` always read the box's own
record.

```json
{
  "packages": [
    {"name": "@nova/json", "version": "1.0.0", "description": "JSON reader"}
  ]
}
```

## Package records

Path: `index/v1/packages/<aa>/<bb>/<encoded-name>.json`, where `aa` and `bb`
are the first two and next two lowercase hex digits of the SHA-256 of the
canonical package name (UTF-8, lowercase, no scope separator normalization
beyond the rules below). `encoded-name` is the canonical name percent-encoded as one path segment:
`[a-z0-9_.@-]` stay literal and `/` becomes `%2F` (for example
`@nova%2Fjson.json`). The client derives the same path locally, so a mirror
must preserve case in the encoding (uppercase hex).

```json
{
  "name": "@publisher/example",
  "versions": [
    {
      "version": "1.2.3",
      "dependencies": [{"name": "@nova/json", "req": "^1.0.0"}],
      "license": "MIT",
      "nova": ">=0.1.0",
      "archive": "https://github.com/OWNER/nova-boxes/releases/download/.../example-1.2.3.box",
      "sha256": "<64 hex>",
      "size": 12345,
      "source": "https://github.com/OWNER/example/tree/<commit>",
      "published": "2026-01-01T00:00:00Z",
      "yanked": false
    }
  ]
}
```

Records are append-only. A version is never overwritten; yanking flips a flag
and is recorded in the same file. `archive` may be absolute or relative to the
registry root; relative paths are what a local mirror and a Pages-hosted index
use, and they resolve without a second configuration field.

## Identity

- Canonical names are ASCII lowercase, `[a-z0-9_-]`, must not start with a
  digit, and are at most 64 characters.
- `@nova/` is reserved for first-party boxes; `@publisher/` is a community
  namespace. Unscoped names are not allowed in the registry.
- Two names that differ only by case or Unicode normalization are the same
  name and are rejected at validation time.

## Archives (`.box`)

Deterministic gzip-compressed tar containing exactly one root directory with
`Box.toml`, source, `README.md`, license files and declared assets. Entries are
sorted by path; timestamps are zero; permissions are normalized; gzip headers
carry no name or timestamp. Limits: 10 MiB compressed, 100 MiB expanded,
10 000 entries, no absolute paths, no `..`, no links or device files.

Extraction verifies the SHA-256 of the downloaded bytes, checks every path
against the destination, and unpacks under
`<cache>/src/<encoded-name>/<version>`. The archive is kept at
`<cache>/archives/<encoded-name>-<version>.box`. `NOVA_BOXES_CACHE` selects the
cache root; the default is `~/.nova/boxes`. A checksum mismatch leaves the
downloaded file for inspection and returns a non-zero exit code.

## Publishing

1. `nova box pack` builds the archive; `nova box publish --dry-run` validates
   and prints the submission.
2. The client opens a pull request containing a submission JSON (name,
   version, source repository, commit, subdirectory, expected SHA-256).
3. CI validates the submission without write permissions and without executing
   untrusted code in a privileged job.
4. A maintainer merges. A privileged workflow rebuilds from the reviewed
   commit, verifies the hash, uploads the release asset, then updates the index
   and catalogue. If the index update fails, the orphan asset stays
   undiscoverable and the job can resume idempotently.

Clients report "submitted, awaiting review" until the index actually advertises
the version. There is no direct anonymous upload.

## Failure behaviour

- Existing locks keep working when a version is yanked, with a warning.
- New resolution never selects a yanked version.
- Checksums establish integrity, not authorship. Authorship comes from the
  reviewed submission and the source commit recorded in the index.
