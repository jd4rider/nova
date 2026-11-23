---
title: Projects and dependencies
description: Manifest, lockfile and the current local-only dependency model.
---

## Layout

```text
my-project/
  nova.toml
  nova.lock
  src/main.nova
  tests/
  target/
```

`nova.toml` declares the package, binary and dependencies. The manifest parser and lockfile are tested in `crates/nova-pkg`. Local path dependencies are supported; a registry, publishing, checksums, workspaces and offline cache are **planned**.

```toml
[package]
name = "my-project"
version = "0.1.0"

[bin]
path = "src/main.nova"

[dependencies]
util = { path = "../util" }
```

## Lockfile

The lockfile records resolved local dependencies so repeated builds use the same graph. Do not hand-edit it. No remote package is installed merely by opening a folder.

## Security

- Dependency installation is not triggered by the IDE on folder open.
- There are no build scripts from a registry.
- No signature, provenance or checksum verification exists for a public registry because no public registry exists.
- Do not treat the current path-dependency model as a supply-chain security boundary.

## Planned

Registry resolution with semver, dev dependencies, workspaces, checksums, archive traversal prevention, explicit trust for build scripts, publishing and provenance are future milestones. The [status page](../../status/) tracks them.
