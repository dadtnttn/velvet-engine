# Velvet Engine — Module map

| Crate | Role |
|-------|------|
| **velvet-core** | Config, errors, plugins, diagnostics, profiling |
| **velvet-app** | `App` loop, schedules, systems, resources, exclusive queue |
| **velvet-math** | Vec2/3, transforms, AABB, Mat3/4, easing, curves, RNG |
| **velvet-time** | Time, fixed step, timers, pause layers, frame limiter |
| **velvet-events** | Typed event channels, history, registry |
| **velvet-assets** | Handles, loaders, hot reload, bundles, missing policy |
| **velvet-render** | wgpu 2D: batch, camera, letterbox, particles, post stack |
| **velvet-text** | Rich markup, typewriter, layout, RTL/ruby/icon helpers |
| **velvet-audio** | Buses, mixer, music crossfade, spatial 2D, DSP helpers |
| **velvet-input** | Actions, bindings, replay, chords, virtual controls |
| **velvet-ui** | Retained UI tree, layout, widgets, dialogue controller |
| **velvet-ecs** | Entities, components, queries, events, change detection |
| **velvet-scene** | Scenes, hierarchy, prefabs, transitions, async load FSM |
| **velvet-script-*** | Lexer → parser → AST → HIR/types → bytecode → VM → format/LSP |
| **velvet-story** | VN runtime: `StoryProgram` / `StoryPlayer` / saves / gallery |
| **velvet-story-lang** | Writer `.vstory` → `StoryProgram` (product IR); Studio model; **boot** for CLI/runtime |
| **velvet-play** | 2D play: tilemaps, physics, AI, pathfinding, particles |
| **velvet-cellular** | **ALPHA** — falling-sand / cellular sim (Noita-like author core; APIs unstable) |
| **velvet-rpg** | Stats, inventory, quests, shops, leveling, equipment |
| **velvet-action** | **Tools**: weapons, projectiles, aim/loadout/hitscan, arena, dash; optional `recipes` room loop |
| **velvet-cards** | **Tools**: catalog, deck validation, zones (not a game) |
| **velvet-anim** | **Tools**: tweens, director, Pose3D, project_image, Timeline; optional `recipes` |
| **velvet-project** | `velvet.project`, module graph, validation |
| **velvet-build** | Pack, localization, desktop export manifests |
| **velvet-cli** | `velvet` CLI |
| **velvet-editor** | **ALPHA** — `velvet-studio` softbuffer editor (Visual/Script/Nodes; APIs/UI unstable) |
| **velvet-runtime** | Ship host binary |
| **velvet-test-utils** | Shared headless helpers |
| **velvet-integration-tests** | Cross-crate scenarios |
| **velvet-bench** | Wall-clock microbenchmarks |

Products: **Velvet Engine**, **Velvet Story**, **Velvet Play**, **Velvet RPG**, **Velvet Action**, **Velvet Cards** (authoring tools), **Velvet Script**, **Velvet Studio**, **Velvet CLI**, **Velvet Runtime**.
