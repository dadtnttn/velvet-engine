# Contributing to Velvet Engine

Thank you for contributing.

## Prerequisites

- Rust stable (see `rust-toolchain.toml`)
- `cargo fmt`, `cargo clippy`
- Optional: `tokei`, `cargo-deny`

## Workflow

1. Read `docs/architecture/VISION.md` and open ADRs under `docs/adr/`.
2. Keep changes aligned with the current development phase when possible.
3. Prefer small, compiling commits.
4. Add tests for behavioral changes.
5. Update docs when public APIs or formats change.
6. Run before opening a PR:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Code standards

See `docs/architecture/CONVENTIONS.md`.

## Architecture decisions

Non-trivial design choices require an ADR in `docs/adr/`.

## License

Contributions are dual-licensed MIT OR Apache-2.0, matching the project.
