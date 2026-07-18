# ADR 0004: Velvet Script Multi-Crate Pipeline

## Status

Accepted

## Context

Narrative authors and programmers share one language. Tools need lossless trees; runtime needs bytecode.

## Decision

Split the pipeline:

```text
lexer → syntax (rowan) → parser → AST → HIR → types → bytecode → VM
```

Plus `format` and `lsp` crates consuming syntax/AST.

Identity: not a Python clone; explicit blocks, game-oriented constructs (`character`, `scene`, `choice`, `component`, `system`).

## Consequences

- More crates; clearer testing.
- Parser recovery supports Studio and LSP.
- VM enforces instruction/memory/recursion limits.
