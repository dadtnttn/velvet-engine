# Velvet Engine — Vision Document

**Version:** 0.1.0  
**Status:** Active  
**Date:** 2026-07-17

## Purpose

Velvet Engine is a modular, data-oriented game engine written in Rust, specialized for two experience families that share a single core:

1. **Narrative experiences** — visual novels, choice-driven adventures, and dialogue-heavy stories (Velvet Story).
2. **Interactive 2D play** — top-down RPGs, exploration adventures, and fast action games inspired by the feel of titles such as Hotline Miami (Velvet Play, Velvet RPG, Velvet Action).

Games may freely combine modules: an RPG can embed branching dialogue; a visual novel can host minigames; an action scene can interrupt with narrative cutscenes.

## Product Identity

| Name | Role |
|------|------|
| Velvet Engine | Full ecosystem |
| Velvet Core | Application loop, plugins, services |
| Velvet Story | Narrative / visual novel systems |
| Velvet Play | General 2D gameplay |
| Velvet RPG | RPG systems on Play |
| Velvet Action | Top-down action on Play |
| Velvet Script | Domain language for narrative and logic |
| Velvet Studio | Visual editor |
| Velvet CLI | Terminal tooling (`velvet`) |
| Velvet Runtime | Packaged game executable |

## Non-Goals (v0.x)

- Full 3D rendering pipeline.
- Networking / multiplayer as a first-class subsystem.
- Console certification support.
- Live2D or skeletal animation as core features (plugin extension points only).
- Copying content, assets, or proprietary systems from other engines or games.

## Design Principles

1. **Modularity** — install only the crates and plugins a project needs.
2. **Performance** — 60 FPS target for dense 2D scenes on reasonable hardware.
3. **Memory safety** — minimal `unsafe`; each use documented with invariants.
4. **Data-oriented** — configuration and content driven by files and Velvet Script.
5. **Readable narrative language** — writers can author scenes without full programming expertise.
6. **Plugin extensibility** — Rust plugins with dependency resolution and version checks.
7. **Engine/game separation** — game code never forked into engine crates.
8. **Reproducible builds** — locked dependencies, documented toolchains.
9. **Understandable errors** — file, line, column, and recovery where possible.
10. **Cross-platform desktop first** — Windows, Linux, macOS; mobile/WASM later.
11. **Documentation from day one** — ADRs, tutorials, and public API docs.
12. **Automated tests** — unit, integration, golden, and benchmarks for critical paths.
13. **Stable saves** — versioned formats and migrations when feasible.

## Success Criteria (Project Completion)

- Workspace builds and runs demos on desktop.
- ≥ 50,000 significant lines of own code (production + tests/tools).
- Core, Render, Assets, Audio, Input, UI, ECS, Scene, Script, Story, Play, RPG, Action, Studio, CLI, Runtime present and functional.
- Six playable demos: Hello Velvet, Visual Novel, Branching Story, Top-down RPG, Action Arena, Hybrid Demo.
- Honest limitation and security reports.

## Audience

- Indie narrative designers and writers.
- 2D game programmers (Rust and Velvet Script).
- Tooling developers extending Velvet Studio / CLI.
- Contributors following CONTRIBUTING.md and ADRs.
