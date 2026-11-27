# Nova WebCore

Status: **not implemented**. This directory records the boundary; it contains no
rendering code yet and must not import or call `runtime/webkit.m`.

WebCore owns:

```text
bytes -> encoding -> HTML tokenizer -> tree builder -> DOM
      -> CSS tokenizer/parser -> selector matching -> cascade/computed style
      -> layout tree -> fragments -> display list -> raster/composite
      -> viewport
```

Initial mode is G1: bundled trusted fixtures only, constrained fixture origin,
network disabled, scripts disabled. Unsupported features fail explicitly.
Remote networking (G2) and script-enabled remote browsing (G3) require the
security gates in `engineering/browser/DIRECTIVE.md` and owner approval.

The first implementation task is `WC-01.001` (trusted fixture harness and
deterministic headless runner), followed by `WC-02.001` (HTML tokenizer).
