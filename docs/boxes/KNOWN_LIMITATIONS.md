# Known limitations

These are current gaps, not design intentions. Commands that would need them
fail with an explicit message.

- **Publishing is not implemented.** `nova box publish` and `login` report
  unsupported. There is no registry repository, Pages site or release workflow
  yet, and no default registry URL: `NOVA_BOXES_REGISTRY` must be set.
- **Only direct dependencies resolve in the compiler.** `import "name"` loads
  a declared path dependency or a registry box pinned in `Box.lock`, but
  transitive boxes are not added to the import map. Version solving happens in
  `nova box update`, not at build time.
- **Registry builds need two commands first.** `nova box update` writes the pin
  and `nova box fetch` fills the cache; compilation never uses the network, so
  a missing box is an actionable error.
- **Cache freshness.** Online reads of `config.json`, the catalogue and records
  are cached under `NOVA_BOXES_CACHE`, and `NOVA_BOXES_OFFLINE=1` serves them.
  There is no expiry, refresh command or offline archive fallback.
- **HTTPS is exercised indirectly.** The HTTP branch is tested against a local
  static server through the same `http_get`/`curl` path. Proxies, custom CAs,
  redirects across hosts and TLS failures have not been tested here.
- **Archives are uncompressed.** `.box` is a valid gzip stream, but deflate
  uses stored blocks, so files are archived, not shrunk.
- **`unpack` restores files only.** Symlinks, hardlinks and device entries are
  rejected; ownership and permissions are not restored. Names containing `..`,
  absolute paths and backslashes are refused before any write.
- **No authentication.** No token is read or stored; reads are anonymous.
- **`tree` and `check` stay path-only.** Only the compiler's import loader and
  `nova box update` consult registry records.
- **`nova.toml` is still accepted.** `Box.toml` wins when both exist; there is
  no automatic migration. `nova new` still scaffolds `nova.toml`.
- **Registry manifests are not supported as roots.** `nova box add` only adds
  scoped registry names; path dependencies remain the local-development path.
- **Studio and the language server** resolve imports by path only; they do not
  query Nova Boxes.
- **The website** has no package catalogue page yet.

Nothing here is mocked: unsupported commands report unsupported.
