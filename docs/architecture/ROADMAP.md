# Development Roadmap

Phases are sequential. Keep the tree compiling after every phase.

| Phase | Name | Exit criterion |
|-------|------|----------------|
| **0** | Definition & architecture | Workspace builds; docs & ADRs; CI skeleton |
| **1** | App & core | Plugins, schedules, empty window, tested loop |
| **2** | Render, input, audio, assets | Hello Velvet |
| **3** | ECS & scenes | Load/update/unload scenes; prefab tests |
| **4** | Velvet Script basic | Compile + VM + diagnostics |
| **5** | Velvet Story | Visual Novel demo end-to-end |
| **6** | Velvet Play | Controllable player, map, collisions |
| **7** | Velvet RPG | Playable RPG loop demo |
| **8** | Velvet Action | Action Arena loop |
| **9** | Velvet Studio (MVP) | Project/scene/inspector/run |
| **10** | Script tooling | Format, LSP basics, richer language |
| **11** | Saves, localization, export | Desktop export + i18n commands |
| **12** | Polish, hybrid demo, reports | 50k LOC, docs, security/perf reports |

## Where 0.3.0 stands (2026-10-16)

The phases above are done. Since then the engine grew a 3D track, driven by Demon Queen: the toon
3D renderer (HDR, shadows, skinning, GPU crowds, water, upscaling, native resolution), AV1 video and
pictures (`velvet-avif`), the VLZ container, navigation and automatic QA, Markdown guiones and the
Velvet Editor. See [CHANGELOG](../../CHANGELOG.md) and the [module map](MODULES.md).

| Track | Status | Notes |
|-------|--------|-------|
| **3D renderer** (`velvet-render` 3D) | **active** | Driven by Demon Queen's places, battles and campaign map. |
| **Media and packs** (`velvet-avif`, `velvet-video`, `velvet-vlz`) | **active** | Release builds ship AVIF/AV1 and VLZ packs. |
| **Velvet Editor** (`velvet-editor3d`) | **alpha** | Docked editor around the engine's own viewport. |
| **Velvet JS** (JavaScript framework) | **proposed** | [Plan](JS_FRAMEWORK_PLAN.md) · [ADR 0009](../adr/0009-rust-and-js-runtimes.md): grow `tools/velvet-three-kit` into a JS framework, then split into Rust and JS runtimes sharing their formats. |

## Near-term priorities (post Phase 12 track)

| Priority | Item | Status | Notes |
|----------|------|--------|-------|
| **Policy** | **Tools first** | **active** | See `docs/architecture/TOOLS.md`. Crates = tools; `demos/` = examples; `recipes` = optional samples. |
| **Language** | **VS3 official general logic** | **active** | `docs/language/VELVET_SCRIPT_3.md` + `docs/language/VS3_ROADMAP.md` + ADR 0008. Engine bindings follow `docs/architecture/VS3_ENGINE_RULES.md`; classic story stays supported. |
| **Language** | Classic `.vel` / StoryProgram | **supported** | Novels / product VN path; do not break. |
| **Now** | **Card tools** (`velvet-cards`) | **active** | Catalog, deck validate, zones, CLI — not a card game. |
| **Now** | **Anim / 3D image tools** (`velvet-anim`) | **active** | Timeline, Pose3D, project_image, story track/pose3d — not premade cutscenes. |
| **Now** | **Action tools** (`velvet-action`) | **active** | Aim, loadout, hitscan, weapons, combat — room loop only in `recipes` / demos. |
| Demo | `velvet-stakes` / `card-duel` / `hotline-rush` / `velvet-novella` | **examples** | Illustrate tools; not the API. |
| Later | Full TCG rules / Hotline product game | **out of scope near-term** | Games are built *with* tools, not shipped as the engine surface. |
| Later | Web / multi-target hosts (incl. web-oriented) | **design only** | VS3 game logic first; no Web3 product in first usable cut. Browser games: see the [Velvet JS plan](JS_FRAMEWORK_PLAN.md). |

## Parallelizable work (within a phase)

- Tests and documentation can trail implementation by a few commits but must land before phase close.
- Benchmarks land when the subsystem exists (not before).

## Definition of Done (per phase)

1. Code compiles (`cargo check --workspace`).
2. Relevant tests pass.
3. Progress report (template in project brief) written under `docs/reports/`.
4. No silent placeholders for claimed features.
5. CHANGELOG updated.
