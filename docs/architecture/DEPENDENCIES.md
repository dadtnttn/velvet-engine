# Justified Dependencies

All third-party crates must earn their place. Prefer std + our crates when adequate.

## Workspace-wide (shared)

| Crate | Used by | Justification |
|-------|---------|---------------|
| `serde` | nearly all | Industry standard serialization; enables RON/JSON configs and save DTOs |
| `serde_json` | project, CLI, tools | Human-readable interchange and debug dumps |
| `ron` | config, assets metadata | Compact, Rust-friendly config without JSON noise |
| `thiserror` | libraries | Typed errors with low boilerplate; public API friendly |
| `anyhow` | CLI, editor, runtime only | Flexible error context in apps; **not** in library crates |
| `tracing` | core, runtime | Structured diagnostics; spans for frame/systems |
| `tracing-subscriber` | app, runtime, CLI | Console/file logging setup |
| `bitflags` | input, render, ecs | Efficient flag sets |
| `smallvec` | ecs, script, render | Reduce heap allocs for small collections |
| `indexmap` | script, assets | Deterministic ordered maps for stable dumps |
| `parking_lot` | core, assets | Faster mutexes for hot paths (optional; evaluate vs std) |
| `uuid` | assets, saves, project | Stable identifiers across reloads |
| `sha2` | assets, saves | Checksums for integrity |
| `camino` | project, assets, CLI | UTF-8 paths; fewer Windows path footguns |

## Platform / Media

| Crate | Justification |
|-------|---------------|
| `winit` | Mature cross-platform windowing and input events |
| `wgpu` | Portable modern GPU API (Vulkan/Metal/DX12/GL/WebGPU) |
| `bytemuck` | Safe pod casts for GPU buffers |
| `image` | Decode common image formats for assets |
| `kira` | Pure-Rust audio with spatial features; good fit for games |
| `cosmic-text` | Unicode/layout/shaping foundation for Velvet Text |
| `glyphon` | Efficient wgpu text rendering on top of cosmic-text |
| `egui` + `egui-wgpu` + `egui-winit` | Fast path for Studio panels; not the only UI long-term |
| `notify` | Filesystem watch for hot-reload in dev |
| `gilrs` | Gamepad abstraction layered under velvet-input |

## Script / Language Tools

| Crate | Justification |
|-------|---------------|
| `logos` | Fast, maintainable lexer generation for Velvet Script |
| `rowan` | Lossless syntax trees for formatter, LSP, IDE fidelity |
| `lsp-types` + `tower-lsp` | Standard LSP server stack |
| `text-size` | Span types compatible with rowan tooling |

## Tools

| Crate | Justification |
|-------|---------------|
| `clap` | Ergonomic CLI parsing with subcommands for `velvet` |
| `walkdir` | Recursive project walks for assets/localization |
| `toml` | Cargo/toolchain adjacent config where needed |
| `regex` | Validation and localization extraction helpers |

## Testing / Benchmarks

| Crate | Justification |
|-------|---------------|
| `pretty_assertions` | Readable test diffs |
| `insta` | Snapshot/golden tests for parser and compiler |
| `criterion` | Statistical benchmarks for hot paths |
| `proptest` / `quickcheck` | Property tests for math, lexer edge cases (selected crates) |

## Explicitly deferred

| Candidate | Why deferred |
|-----------|--------------|
| `bevy_ecs` | Own ECS keeps control of API, serialization, and script interop |
| `rapier2d` | May integrate later behind physics façade; start with custom simple 2D |
| `tokio` | Prefer lightweight async (pollster/futures) unless Studio needs full runtime |
| `nalgebra` / `glam` | Start with `velvet-math`; can adopt glam later if profiling demands |

## Version policy

- Prefer crates with active maintenance and dual/Apache-MIT-compatible licenses.
- Pin major versions in workspace `Cargo.toml` `[workspace.dependencies]`.
- `cargo deny check` in CI for licenses, bans, and advisories.
