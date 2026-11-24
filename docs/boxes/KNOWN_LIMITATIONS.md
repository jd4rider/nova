# Known limitations

These are current gaps, not design intentions. Commands that would need them
fail with an explicit message.

- **Publishing is not implemented.** `nova box publish` and `login` report
  unsupported. There is no registry repository, Pages site or release workflow
  yet, and no default registry URL: `NOVA_BOXES_REGISTRY` must be set.
- **No manifest editing.** `nova box add NAME@VERSION`, `remove` and `update`
  are absent; dependencies are still edited by hand.
- **`fetch` does not write the lockfile.** It downloads and unpacks a box, but
  resolution ignores fetched boxes, so a version-only dependency is still
  rejected by `tree`/`check`/`lock`.
- **HTTP reads are unverified in this environment.** The HTTPS path uses the
  host `curl` through `http_get`; the tests exercise the identical code against
  a filesystem mirror. Proxy configuration, redirects and TLS errors have not
  been tested here.
- **No yank caching or offline flag.** Every command re-reads the index; there
  is no cached catalogue, no `--offline` and no warning when a locked version
  was yanked after the lock was written.
- **Archives are uncompressed.** `.box` is a valid gzip stream, but deflate
  uses stored blocks, so files are archived, not shrunk.
- **`unpack` restores files only.** Symlinks, hardlinks and device entries are
  rejected; ownership and permissions are not restored. Names containing `..`,
  absolute paths and backslashes are refused before any write.
- **No authentication.** No token is read or stored; reads are anonymous.
- **Lockfile scope.** `nova box lock` writes one `Box.lock` per manifest, but
  builds do not consult it (`--locked` is not integrated with Boxes).
- **`nova.toml` is still accepted.** New projects should use `Box.toml`.
- **Studio and the language server** resolve imports by path only; they do not
  query Nova Boxes.
- **The website** has no package catalogue page yet.

Nothing here is mocked: unsupported commands report unsupported.
