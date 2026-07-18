# FASE 8 — LSP de producto (stdio)

## Objetivo

Servidor LSP real por stdio, usable desde VS Code / Studio.

## Estado inicial

- NDJSON legacy (`analyze`/`completions` line protocol).
- Sin Content-Length framing.

## Cambios

| Ítem | Detalle |
|------|---------|
| `velvet-script-lsp::stdio` | JSON-RPC 2.0 + Content-Length |
| Métodos | initialize, didOpen/Change/Close, completion, hover, definition, references, rename, documentSymbol, formatting, publishDiagnostics |
| Binary | default stdio; `VELVET_LSP_LEGACY=1` for old NDJSON |
| Tests | unit + `tests/lsp_stdio_protocol.rs` |
| VS Code stub | `editors/vscode-velvet/` |

## Criterio de salida

- [x] initialize + capabilities  
- [x] didOpen → publishDiagnostics  
- [x] documentSymbol / completion on real `.vel` text  
- [x] Content-Length frame round-trip  
- [ ] Full VS Code E2E in CI (requires npm + PATH binary; documented)

## Comandos

```bash
cargo test -p velvet-script-lsp --all-features
cargo run -p velvet-script-lsp   # stdio LSP
```

Evidence: `{SCRATCH}/lsp_stdio.log`
