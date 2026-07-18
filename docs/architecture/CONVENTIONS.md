# Engineering Conventions

## Language

- Rust stable toolchain (see `rust-toolchain.toml`).
- Edition 2021 (or workspace edition).
- `#![warn(missing_docs)]` on public library crates where practical.

## Naming

- Crates: `velvet-<area>`.
- Plugins: `FooPlugin`.
- Systems: snake_case functions registered with schedule labels.
- Velvet Script files: `.vel`, `.velscene`, `.velui`, `.velproject`, `.velprefab`.

## Errors

- Libraries: `thiserror` enums.
- Binaries: `anyhow` at boundaries.
- Prefer `Result` over panics; panics only for programming bugs.

## Modules

- Prefer many small modules over mega-files.
- `lib.rs` re-exports the public API deliberately.

## Testing

- Unit tests in the same file or `tests/` submodule.
- Integration tests in crate `tests/` or workspace `tests/`.
- Golden tests for parser/compiler with `insta` where useful.

## Unsafe

- Avoid by default.
- Every `unsafe` block: `// SAFETY:` comment, invariants, and a test if feasible.

## Commits / PRs (when used)

- One logical change per commit when possible.
- Reference ADR IDs for architectural shifts.

## Documentation language

- Primary engineering docs: English or Spanish acceptable; this repo uses **Spanish for vision/narrative docs and English for API identifiers**.
- Code identifiers: English.
