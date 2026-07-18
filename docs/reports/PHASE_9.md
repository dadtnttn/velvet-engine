# Phase 9 Report — Tooling Hardening (Project, Build, CLI)

## Goal

Make project configuration and build tooling trustworthy enough for daily
prototype work: validation, module dependency resolution, richer pack/export,
and a modular CLI.

## Delivered

### `velvet-project`
- Module registry with hard requires + soft recommends
- Transitive dependency resolution (topo order)
- `validate_project` / `validate_root` with severity-coded issues
- Enable/disable helpers and template-oriented RON generators

### `velvet-build`
- Pack include/exclude globs (`**/*.tmp`, `raw/**`, …)
- Localization: JSON + simple PO + properties
- Multi-platform export dry-run manifests

### `velvet-cli`
- Split into modules: `doctor`, `script_cmd`, `loc_cmd`, `pack_cmd`, `export_cmd`, `new_cmd`
- Doctor checks workspace crates + templates content
- Export `--multi` / `--platform`, pack `--exclude` / `--include`
- Project info `--validate`

## Tests

```bash
cargo test -p velvet-project -p velvet-build
cargo test -p velvet-cli
```

## Honest status

- Export **build** path still shells out to cargo and is best-effort for binary discovery.
- PO support is a **subset** (no plural forms, no msgctxt blocks beyond comments).
- Project validation does not yet schema-version the RON format.

## Exit criteria

| Criterion | Status |
|-----------|--------|
| Module deps resolve for rpg/action/story | Done |
| Pack filters work in unit tests | Done |
| Multi-platform dry-run JSON | Done |
| CLI modules compile with tests | Done |
