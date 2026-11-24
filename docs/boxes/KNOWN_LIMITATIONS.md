# Known limitations

These are current gaps, not design intentions. Commands that would need them
fail with an explicit message.

- **Publishing is not implemented.** `nova box publish` and `login` report
  unsupported. There is no registry repository, Pages site or release workflow
  yet, and no default registry URL: `NOVA_BOXES_REGISTRY` must be set.
- **Builds ignore `Box.lock`.** `nova box update` resolves path and registry
  dependencies into a lockfile, but `nova build` still refuses any project with
  `[dependencies]` and resolves imports by path only. `tree` and `check` remain
  path-only as well.
- **No yank warning.** `update` never selects a yanked version, but an existing
  lock that pins one is not flagged.
- **No cached index or `--offline`.** Every command re-reads `config.json` and
  the record it needs; there is no catalogue cache and no offline mode beyond
  pointing `NOVA_BOXES_REGISTRY` at a local mirror.
- **HTTPS is exercised indirectly.** The HTTP branch is tested against a local
  static server through the same `http_get`/`curl` path. Proxies, custom CAs,
  redirects across hosts and TLS failures have not been tested here.
- **Archives are uncompressed.** `.box` is a valid gzip stream, but deflate
  uses stored blocks, so files are archived, not shrunk.
- **`unpack` restores files only.** Symlinks, hardlinks and device entries are
  rejected; ownership and permissions are not restored. Names containing `..`,
  absolute paths and backslashes are refused before any write.
- **No authentication.** No token is read or stored; reads are anonymous.
- **`nova.toml` is still accepted.** New projects should use `Box.toml`.
- **Registry manifests are not supported as roots.** `nova box add` only adds
  scoped registry names; path dependencies remain the local-development path.
- **Studio and the language server** resolve imports by path only; they do not
  query Nova Boxes.
- **The website** has no package catalogue page yet.

Nothing here is mocked: unsupported commands report unsupported.
