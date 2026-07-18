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
| **velvet-story** | VN runtime: dialogue, choices, saves, gallery, glossary |
| **velvet-play** | 2D play: tilemaps, physics, AI, pathfinding, particles |
| **velvet-cellular** | From-scratch cellular material sim (Noita-like author core, no external physics) |
| **velvet-rpg** | Stats, inventory, quests, shops, leveling, equipment |
| **velvet-action** | Combat, weapons, projectiles, arena waves, dash/combo |
| **velvet-project** | `velvet.project`, module graph, validation |
| **velvet-build** | Pack, localization, desktop export manifests |
| **velvet-cli** | `velvet` CLI |
| **velvet-editor** | `velvet-studio` tooling shell |
| **velvet-runtime** | Ship host binary |
| **velvet-test-utils** | Shared headless helpers |
| **velvet-integration-tests** | Cross-crate scenarios |
| **velvet-bench** | Wall-clock microbenchmarks |

Products: **Velvet Engine**, **Velvet Story**, **Velvet Play**, **Velvet RPG**, **Velvet Action**, **Velvet Script**, **Velvet Studio**, **Velvet CLI**, **Velvet Runtime**.
