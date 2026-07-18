# Line count report — Velvet Engine

Primary evidence: implementer scratch `tokei_report.txt` and `docs/reports/tokei_report.txt`.

## Summary (own significant Rust)

| Bucket | Lines | Target |
|--------|------:|-------:|
| **Código de producción propio** | **35 075** | ≥ 35 000 |
| **Pruebas y herramientas propias** | **15 840** | ≥ 15 000 |
| Ejemplos | 898 | — |
| Código generado | 0 | — |
| Dependencias externas (source) | 0 | not counted |
| **TOTAL SIGNIFICATIVO** | **51 813** | ≥ 50 000 |

Method: non-blank, non-`//`, non-block-comment Rust lines; `#[cfg(test)]` modules and `tests/` dirs counted as tests. Tools crates: `velvet-cli`, `velvet-editor`, `velvet-bench`, `velvet-test-utils`, `velvet-integration-tests`.

## tokei raw

```bash
tokei crates examples tools tests --exclude target
```

See latest `tokei_report.txt` for full tokei table (Rust Code ≈ 51k+).

## Gating evidence

| Command | Log |
|---------|-----|
| `cargo build --workspace` | `{SCRATCH}/cargo_build_workspace.log` |
| `cargo test --workspace --all-features` | `{SCRATCH}/cargo_build_test.log` |

## Reproduction

```bash
tokei crates examples tools tests --exclude target
cargo build --workspace
cargo test --workspace --all-features
cargo run -p velvet-cli -- --help
cargo run -p hybrid-demo
```
