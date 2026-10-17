# Velvet Engine — Module map

This file is the compact inventory of the 65 crates in `crates/`, grouped by family. The
responsibility, retention rule, dependency direction and cross-crate proof for every family are
documented in [INTEGRATION_MATRIX.md](INTEGRATION_MATRIX.md). A crate is not justified by line count
alone.

Maturity labels: **ALPHA** surfaces may change their APIs, UI and file formats without notice.

## Core

| Crate | Role |
|-------|------|
| **velvet-core** | Config, errors, plugins, diagnostics, profiling, engine version |
| **velvet-app** | `App` loop, schedules, systems, resources, typed states, exclusive queue |
| **velvet-math** | Vec2/3, transforms, AABB, Mat3/4, easing, curves, RNG |
| **velvet-time** | Time, fixed step, timers, pause layers, frame limiter |
| **velvet-events** | Typed event channels, history, registry |
| **velvet-ecs** | Entities, components, queries, events, change detection |
| **velvet-scene** | Scenes, hierarchy, prefabs, transitions, async load FSM |
| **velvet-assets** | Handles, loaders, hot reload, bundles, missing policy |
| **velvet-input** | Actions, bindings, replay, chords, virtual controls |
| **velvet-audio** | Buses, mixer, music crossfade, spatial 2D, DSP helpers |

## Rendering, media and formats

| Crate | Role |
|-------|------|
| **velvet-render** | wgpu renderer. 2D: batches, cameras, letterbox, particles, post stack. 3D forward: toon shading with ink lines, HDR + bloom, cached shadows, up to 32 lights, GPU skinning, GPU crowds (`crowd3d`: thousands of animated soldiers as one mesh), water, SSAO and reflections, FSR1/FXAA upscaling, native-resolution output |
| **velvet-render-rtx** | Nvidia-specialised Vulkan RTX backend, separate from the wgpu forward renderer |
| **velvet-scene3d** | Declarative `.scene3d.json` scenes, AABB physics, character controllers, animation blending, 3D particles, 3D saves |
| **velvet-anim** | **Tools**: tweens, director, Pose3D, `project_image`, timelines; optional `recipes` |
| **velvet-fx** | Procedural pixel-art VFX forge (loopable, palette-driven) |
| **velvet-image** | Image probe / decode / encode for script and style hosts |
| **velvet-avif** | AV1 decoding on rav1d (pure Rust, no system codec): AVIF pictures and AV1 clips in IVF |
| **velvet-video** | Frame-sequence cinema player (openings, cutscenes) |
| **velvet-vlz** | VLZ (Velvet Compressed): block-compressed, random-access resource container ([format](../formats/VLZ_FORMAT.md)) |

## World and simulation

| Crate | Role |
|-------|------|
| **velvet-play** | 2D play: tilemaps, physics, camera, AI, pathfinding, particles |
| **velvet-nav** | Navmeshes sampled from a game's own walkability, A* with a funnel pass, ORCA crowds |
| **velvet-terrain** | Pure-math 3D terrain synthesis: noise, erosion, meshing, LOD, biomes |
| **velvet-liquid** | Pure-math liquid simulation: shallow-water heightfields and SPH particles |
| **velvet-dungeon** | Deterministic dungeon blueprints, independent of renderer and assets |
| **velvet-cellular** | **ALPHA** — falling-sand / cellular simulation (Noita-like author core) |
| **velvet-rpg** | Stats, inventory, quests, shops, leveling, equipment |
| **velvet-action** | **Tools**: weapons, projectiles, aim/loadout/hitscan, arena, dash; optional `recipes` |
| **velvet-cards** | **Tools**: catalog, deck validation, zones (not a game) |

## Story, text and UI

| Crate | Role |
|-------|------|
| **velvet-story** | VN runtime: `StoryProgram` / `StoryPlayer` / saves / gallery |
| **velvet-story-lang** | Writer `.vstory` → `StoryProgram` (product IR); Studio model; boot for CLI/runtime |
| **velvet-guion** | Guiones: dialogue and visual-novel scenes written as Markdown, played as conversations (stage directives such as `@fondo`, `@recuerdo`) and compiled to story nodes |
| **velvet-text** | Rich markup, typewriter, layout, RTL / ruby / icon helpers |
| **velvet-document** | Preservable document model for visual / advanced / protected region round-trips |
| **velvet-ui** | Retained UI tree, layout, widgets, dialogue controller |
| **velvet-style** | **Tools**: `.vcss` CSS + JS-lite (cascade, `@keyframes`, `@script`); story `style.load` / `style.call` |

## Velvet Script

| Crate | Role |
|-------|------|
| **velvet-script-lexer** · **-syntax** · **-parser** | Lexer, syntax tables, parser with error recovery |
| **velvet-script-ast** · **-hir** · **-resolve** · **-types** | Typed AST, high-level IR, name resolution and import graph, static type checker |
| **velvet-script-compiler** · **-bytecode** · **-vm** | Compiler, bytecode modules, sandboxed virtual machine |
| **velvet-script-stdlib** · **-layers** · **-i18n** | Typed prelude, layer stack runtime, message catalogs (extract / validate) |
| **velvet-script-format** · **-lsp** · **-corpus** | Formatter, language-server helpers, corpus of real sample programs |
| **velvet-script-vs3** | Velvet Script 3 — the official game-logic language (tasks, host ABI, packages) |
| **velvet-script-vs3-engine** | **Optional** engine adapters for VS3: `velvet-math` conversions, `velvet.time` / `velvet.input` host modules |
| **velvet-vs3-app** | Declarative VS3 application runtime: windows, resources, input, 2D primitives |
| **velvet-crypto** | Sandbox crypto for scripts: hash / HMAC and a hybrid classical + post-quantum seal |

## Tooling and products

| Crate | Role |
|-------|------|
| **velvet-project** | `velvet.project`, module graph, validation |
| **velvet-build** | Pack, localization, desktop / Android export manifests |
| **velvet-cli** | The `velvet` command line |
| **velvet-editor** | **ALPHA** — Velvet Studio, softbuffer editor (Visual / Script / Nodes) |
| **velvet-editor3d** | **ALPHA** — Velvet Editor: docked scene tree, inspector, files, scripts and the engine's own 3D viewport (egui) |
| **velvet-runtime** | Ship host binary |
| **velvet-qa** | Automatic QA: a walker that plays a zone over its navmesh and reports where it gets stuck and the frame times |
| **velvet-test-utils** | Shared headless helpers |
| **velvet-integration-tests** | Cross-crate scenarios |
| **velvet-bench** | Wall-clock microbenchmarks |

Products: **Velvet Engine**, **Velvet Story**, **Velvet Play**, **Velvet RPG**, **Velvet Action**,
**Velvet Cards** (authoring tools), **Velvet Script**, **Velvet Studio**, **Velvet Editor**,
**Velvet CLI**, **Velvet Runtime**.

Outside `crates/`: `tools/velvet-three-kit` is the JavaScript (Three.js) kit the 3D prototypes are
built with; see the [JavaScript framework plan](JS_FRAMEWORK_PLAN.md) for where it is going.
