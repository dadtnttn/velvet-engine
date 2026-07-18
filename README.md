<p align="center">
  <img src="docs/assets/velvet-banner.svg" alt="Velvet Engine" width="920"/>
</p>

<p align="center">
  <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/language-Rust-DEA584?style=for-the-badge&logo=rust&logoColor=white" alt="Rust"/></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-9B59B6?style=for-the-badge" alt="License"/></a>
  <a href="rust-toolchain.toml"><img src="https://img.shields.io/badge/rustc-1.80%2B-orange?style=for-the-badge&logo=rust&logoColor=white" alt="Rust 1.80+"/></a>
  <a href="docs/architecture/ROADMAP.md"><img src="https://img.shields.io/badge/status-early%20development-2A1545?style=for-the-badge" alt="Early development"/></a>
  <a href="docs/architecture/CELLULAR.md"><img src="https://img.shields.io/badge/cellular-alpha-E67E22?style=for-the-badge" alt="Cellular alpha"/></a>
  <a href="docs/editor/STUDIO.md"><img src="https://img.shields.io/badge/studio-alpha-E67E22?style=for-the-badge" alt="Studio alpha"/></a>
</p>

<p align="center">
  <strong>Modular Rust game engine</strong> for visual novels, narrative adventures, and 2D interactive play.<br/>
  Shared core · installable modules · dedicated language: <strong>Velvet Script</strong>
</p>

<p align="center">
  <a href="#quick-start">Quick start</a> ·
  <a href="#modules">Modules</a> ·
  <a href="#try-it">Try it</a> ·
  <a href="#documentation">Docs</a> ·
  <a href="#workspace">Workspace</a> ·
  <a href="#license">License</a>
</p>

---

## Overview

Velvet Engine is built for two experience families that share one core:

| Pillar | What you ship |
|--------|----------------|
| **Velvet Story** | Visual novels, branching dialogue, choices, rollback, saves, i18n |
| **Velvet Play / RPG / Action** | Top-down maps, collisions, stats, quests, combat loops |
| **Velvet Cellular** *(alpha)* | Falling-sand / Noita-like materials for authors |

Combine freely: an RPG can embed branching dialogue; a novel can host a minigame; action can cut to narrative.

> **Status:** early development — see [`docs/architecture/ROADMAP.md`](docs/architecture/ROADMAP.md).<br/>
> **Cellular / sand** is **alpha** (`0.1.0-alpha.x`); APIs may break. Details: [`docs/architecture/CELLULAR.md`](docs/architecture/CELLULAR.md).<br/>
> **Velvet Studio** is **alpha** (same maturity band as cellular): usable for prototyping; APIs, UI, and file formats may break. Details: [`docs/editor/STUDIO.md`](docs/editor/STUDIO.md).

---

## Modules

| Module | Role | Maturity |
|--------|------|----------|
| **Core** | App loop, plugins, time, events, config | Active |
| **Render** | wgpu 2D sprites, cameras, letterbox, profiles | Active |
| **Audio** | Buses, voices, music fades | Active |
| **Input** | Actions, bindings, winit map | Active |
| **Story** | Dialogue, choices, rollback, gallery, product VN host | Active |
| **Script** | Lexer → compiler → sandboxed VM | Active |
| **Play** | Tilemaps, physics, A*, cameras | Active |
| **RPG** | Stats, inventory, quests, party | Active |
| **Action** | Weapons, projectiles, perception | Active |
| **Cellular** | Falling sand / materials author core | **Alpha** |
| **Studio** | Softbuffer dual/triple-mode editor shell | **Alpha** |
| **CLI** | Project tools, play, export | Active |

---

## Quick start

**Requirements:** [Rust](https://www.rust-lang.org/) stable (see `rust-toolchain.toml`).

```bash
# Clone
git clone https://github.com/dadtnttn/velvet-engine.git
cd velvet-engine

# CLI
cargo build -p velvet-cli --release
cargo run -p velvet-cli -- --help

# Hello path
cargo run -p hello-velvet --release
```

### Product visual novel (headless)

```bash
cargo run -p velvet-cli --release -- play samples/short-novella --choice 0
```

### Windowed novella demo

```bash
cargo run -p velvet-novella --release
```

### Cellular / sand demo *(alpha)*

```bash
cargo run -p cellular-arena --release
# or: demos/cellular-arena/run.bat
```

---

## Try it

| Path | Command | Notes |
|------|---------|--------|
| Hello Velvet | `cargo run -p hello-velvet --release` | Core plugins smoke |
| Short novella | `velvet play samples/short-novella --choice 0` | Product `VnSession` → named ending |
| Windowed novel | `cargo run -p velvet-novella --release` | Softbuffer VN UI |
| Cellular arena | `cargo run -p cellular-arena --release` | **Alpha** sand / cave run |
| Cellular lab | `cargo run -p cellular-lab --release` | Headless author lab |
| Studio GUI *(alpha)* | `cargo run -p velvet-editor --release -- gui templates/visual-novel --interactive` | Visual / Script / Nodes |
| Visual novel example | `cargo run -p visual-novel --release` | CLI story player |
| Branching story | `cargo run -p branching-story --release` | Multi-ending checks |
| Top-down RPG | `cargo run -p top-down-rpg --release` | Play + RPG loop |
| Action arena | `cargo run -p action-arena --release` | Combat sandbox |

After building the CLI once:

```bash
cargo run -p velvet-cli --release -- play samples/short-novella --choice 0
cargo run -p velvet-cli --release -- play samples/short-novella --windowed --choice 0
```

---

## Documentation

| Topic | Link |
|-------|------|
| Vision | [docs/architecture/VISION.md](docs/architecture/VISION.md) |
| Architecture | [docs/architecture/OVERVIEW.md](docs/architecture/OVERVIEW.md) |
| Modules map | [docs/architecture/MODULES.md](docs/architecture/MODULES.md) |
| Roadmap | [docs/architecture/ROADMAP.md](docs/architecture/ROADMAP.md) |
| Velvet Script | [docs/language/VELVET_SCRIPT.md](docs/language/VELVET_SCRIPT.md) · **VS2 (alpha, rust-like):** [VELVET_SCRIPT_2.md](docs/language/VELVET_SCRIPT_2.md) |
| Velvet Story | Writer layer over VS2 (`.vstory`): [VELVET_STORY.md](docs/language/VELVET_STORY.md) |
| Cellular (alpha) | [docs/architecture/CELLULAR.md](docs/architecture/CELLULAR.md) |
| Studio (alpha) | [docs/editor/STUDIO.md](docs/editor/STUDIO.md) |
| Ren’Py parity notes | [docs/reports/RENPY_PARITY.md](docs/reports/RENPY_PARITY.md) |
| Limitations (honest) | [docs/reports/LIMITATIONS.md](docs/reports/LIMITATIONS.md) |
| Tutorials | [docs/tutorials/](docs/tutorials/) |
| ADRs | [docs/adr/](docs/adr/) |
| Contributing | [CONTRIBUTING.md](CONTRIBUTING.md) |

Brand assets used on this page: [`docs/assets/`](docs/assets/).

---

## Workspace

```text
crates/      Engine libraries (core, story, play, cellular, cli, …)
demos/       Windowed demos (velvet-novella, cellular-arena)
examples/    Focused demos and labs
templates/   Project starters for velvet new
samples/     Ship-shaped sample projects (short-novella)
docs/        Architecture, language, reports, tutorials
editors/     VS Code language support
```

Measure lines of code (optional, [tokei](https://github.com/XAMPPRocky/tokei)):

```bash
tokei crates demos examples docs tools tests templates --exclude target
```

---

## Design principles

- **Modular:** install only the crates you need.
- **Author-friendly:** narrative scripts and product play path first-class.
- **Honest maturity:** alpha surfaces (**cellular**, **Studio**) are labeled; limitations documented.
- **Rust-native:** workspace crates, tests in-tree, dual license compatible with the ecosystem.

---

## License

Dual-licensed under **MIT OR Apache-2.0**, at your option.

- [LICENSE](LICENSE) — summary
- [LICENSE-MIT](LICENSE-MIT)
- [LICENSE-APACHE](LICENSE-APACHE)

Contributions are accepted under the same terms ([CONTRIBUTING.md](CONTRIBUTING.md)).

---

<p align="center">
  <img src="docs/assets/velvet-mark.svg" alt="Velvet mark" width="64"/><br/>
  <sub>Velvet Engine · early development</sub>
</p>
