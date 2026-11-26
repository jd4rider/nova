---
title: Contributing
description: Build, test and document changes without overstating maturity.
---

## Principles

1. Preserve working code and uncommitted user changes.
2. Prefer a small runnable increment over a large unverified scaffold.
3. Record consequential architecture decisions in `docs/adr/`.
4. Never mark a planned feature implemented. Unsupported syntax must be rejected or clearly labeled.
5. Do not invent FreeBuff APIs, benchmark results, screenshots, releases or deployment success.

## Build and test

```sh
(cd examples/nova-tool && ../../bootstrap/nova build)
sh examples/nova-tool/tests/codegen.sh
sh examples/nova-tool/tests/diff.sh
sh examples/nova-tool/tests/boxes.sh

cd editor
npm run typecheck
npm run build

cd ../apps/website
npm install --cache /tmp/nova-npm-cache
npm run check
npm run build
npm run test
```

The website's `npm run test` checks links and representative routes, then compiles every code fence marked `nova check`. Run it after changing examples or docs.

## Website changes

- Use the base-safe `href()` helper for custom pages. Do not hardcode root-relative URLs.
- Docs use Starlight content collections under `src/content/docs`.
- Feature status lives in `src/data/maturity.ts`; keep pages consistent with it.
- Real screenshots only. If Studio has not been run and verified, do not add a mock screenshot.
- Test both `/` and a project base such as `/nova/`.

## Language changes

Add focused lexer/parser/sema/codegen tests and update `docs/grammar.ebnf` and the language reference together. Diagnostics need stable codes and spans. Backend changes should pass the LLVM verifier and native execution tests.

## Reporting security issues

Do not open a public issue with secrets or an exploit. Use the repository's private security advisory channel when a remote is configured; otherwise contact the maintainer who provided the source.

## Current gaps

The most useful contributions are automated Studio/LSP tests, incremental LSP sync, user modules, generics, DAP, benchmarks and release packaging. See [Status](../status/).
