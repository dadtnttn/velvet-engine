# Velvet Engine

**Velvet Engine** is a modular Rust game engine for:

1. Visual novels, narrative adventures, and choice-driven stories (**Velvet Story**).
2. 2D interactive games — top-down RPG, exploration, and fast action (**Velvet Play / RPG / Action**).

One shared core, installable modules, and a dedicated language: **Velvet Script**.

> Status: early development. Phase roadmap lives in [`docs/architecture/ROADMAP.md`](docs/architecture/ROADMAP.md).

## Features (planned / in progress)

| Module | Role |
|--------|------|
| Velvet Core | App loop, plugins, time, events |
| Velvet Render | wgpu 2D sprites, cameras, profiles |
| Velvet Story | Dialogue, choices, rollback, galleries |
| Velvet Play | Maps, collisions, 2D gameplay |
| Velvet RPG | Stats, inventory, quests |
| Velvet Action | Top-down combat and perception |
| Velvet Script | Narrative + gameplay language |
| Velvet Studio | Visual editor |
| Velvet CLI | `velvet` tooling |

## Quick start

```bash
# Requirements: Rust stable
cargo build -p velvet-cli
cargo run -p velvet-cli -- --help

# Run the Hello Velvet example (as it becomes available)
cargo run -p hello-velvet
```

## Documentation

- [Vision](docs/architecture/VISION.md)
- [Architecture overview](docs/architecture/OVERVIEW.md)
- [Dependencies](docs/architecture/DEPENDENCIES.md)
- [Roadmap](docs/architecture/ROADMAP.md)
- [ADRs](docs/adr/)
- [Contributing](CONTRIBUTING.md)

## Workspace layout

```text
crates/     Engine libraries and tools
examples/   Playable demos
templates/  Project templates for `velvet new`
docs/       Architecture, language, tutorials
tests/      Workspace-level integration tests
tools/      Helper scripts
```

## License

MIT OR Apache-2.0. See [LICENSE](LICENSE).

## Line counts

Measure with [tokei](https://github.com/XAMPPRocky/tokei) (exclude `target/`):

```bash
tokei crates examples docs tools tests templates --exclude target
```
