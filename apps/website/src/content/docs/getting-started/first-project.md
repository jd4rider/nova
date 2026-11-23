---
title: Your first project
description: Understand the Nova project manifest, source layout and task commands.
---

`nova new hello` creates a binary project with this layout:

```text
hello/
  nova.toml
  src/main.nova
  tests/arithmetic.nova
  .gitignore
```

## Manifest

```toml
[package]
name = "hello"
version = "0.1.0"

[bin]
path = "src/main.nova"

[dependencies]
```

`nova.toml` is required for project tasks and language-server startup. `nova.lock` is written by the package layer when dependencies are resolved. Local path dependencies are supported by the manifest parser, but a package registry, workspaces and registry semver resolution are **planned**, not available.

## Tasks

```sh
nova check   # parse and type-check without linking
nova run     # build and execute the declared binary
nova build   # emit the native executable
nova test    # discover @test functions and run each in its own process
nova fmt     # format source conservatively
nova tree    # show the project structure
```

`nova test` currently discovers functions annotated with `@test`, compiles a separate binary per test and reports the actual process status. There is no test explorer or machine-readable report format yet.

```nova title="tests/arithmetic.nova"
import std.core { assert }

@test
fn arithmetic_works() {
    assert(1 + 1 == 2)
}
```

`assert` and `assert_eq` are builtins. A failed assertion exits non-zero and includes the source line.

## Build artifacts

- `target/nova/` contains native executables.
- No dependency installation happens when a folder is opened.
- There is no incremental compilation yet; each task performs its own build.

## Next

- [CLI reference](../../toolchain/cli/)
- [Project manifests](../../toolchain/projects/)
