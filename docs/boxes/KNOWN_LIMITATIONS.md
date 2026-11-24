# Known limitations

These are current gaps, not design intentions. Commands that would need them
fail with an explicit message.

- **No registry access yet.** `nova box search`, `info`, `add NAME@VERSION`,
  `fetch`, `update` and `publish` are not implemented. A version-only
  dependency is rejected with "registry-only".
- **Archives are uncompressed.** `.box` is a valid gzip stream, but deflate
  uses stored blocks, so files are archived, not shrunk.
- **No compression.** Deflate blocks are stored, so `.box` files are larger
  than a normal `.tar.gz`.
- **`unpack` restores files only.** Symlinks, hardlinks and device entries are
  rejected; ownership and permissions are not restored. Names containing `..`,
  absolute paths and backslashes are refused before any write.
- **No authentication.** `nova box login/logout` are absent; no token is read
  or stored.
- **Small CLI surface.** `nova box tree/check/lock/metadata/resolve` run today;
  `add`, `remove`, `fetch`, `update`, `search`, `info`, `publish`, `login`
  and `audit` report unsupported with a non-zero exit code.
- **Lockfile scope.** `nova box lock` writes one `Box.lock` per manifest, but
  builds do not yet consult it (`--locked` is not integrated with Boxes).
- **`nova.toml` is still accepted.** New projects should use `Box.toml`.
- **Studio and the language server** resolve imports by path only; they do not
  query Nova Boxes.
- **The website** has no package catalogue page yet.

Nothing here is mocked: unsupported commands report unsupported.
