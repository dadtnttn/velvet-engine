# ADR 0007 — Document regions for visual/advanced round-trip

## Status

Accepted (Phase 2)

## Context

Velvet Studio needs simplified and advanced modes editing the **same** project files. A full lossless CST of Velvet Script is large; product needs non-destructive visual edits soon.

## Decision

Introduce `velvet-document` with explicit region markers in comments:

```text
// @visual id=…
// @advanced id=…
// @protected id=…
// @end
```

Visual tools may rewrite property lines inside `@visual` regions only. Advanced/protected bodies are preserved and re-emitted.

## Consequences

- + Simple, testable, git-friendly.
- + Works without full language CST.
- − Nested script AST inside advanced is opaque to visual tools (by design).
- − Marker discipline required in templates.

## Alternatives rejected

1. Separate `.velui` JSON + `.vel` code — dual source of truth, sync bugs.
2. Full rowan CST now — too large for Phase 2; can layer later.
