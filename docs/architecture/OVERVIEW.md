# Velvet Engine — Architecture Overview

## Layered Model

```text
┌─────────────────────────────────────────────────────────────┐
│  Velvet Studio / Velvet CLI / Game Projects / Demos         │
├─────────────────────────────────────────────────────────────┤
│  Velvet Story │ Velvet Play │ Velvet RPG │ Velvet Action    │
├─────────────────────────────────────────────────────────────┤
│  Velvet Script (lexer → parser → HIR → compiler → VM)       │
│  Velvet UI │ Velvet Scene │ Velvet ECS                      │
├─────────────────────────────────────────────────────────────┤
│  Velvet Render │ Velvet Text │ Velvet Audio │ Velvet Input   │
│  Velvet Assets                                              │
├─────────────────────────────────────────────────────────────┤
│  Velvet App │ Velvet Core │ Events │ Time │ Math            │
├─────────────────────────────────────────────────────────────┤
│  Platform: wgpu, winit, OS audio/input backends             │
└─────────────────────────────────────────────────────────────┘
```

## Crate Map

### Foundation

| Crate | Responsibility |
|-------|----------------|
| `velvet-math` | Vectors, matrices, rects, colors, transforms (no game deps) |
| `velvet-time` | Clocks, fixed/variable timesteps, timers |
| `velvet-events` | Typed event buses, double-buffered queues |
| `velvet-core` | Errors, config, diagnostics, services, plugin registry types |
| `velvet-app` | `App` builder, schedules, states, main loop orchestration |

### Media & I/O

| Crate | Responsibility |
|-------|----------------|
| `velvet-assets` | Handles, loaders, cache, hot-reload, virtual paths |
| `velvet-render` | wgpu 2D renderer, cameras, batching, post-FX profiles |
| `velvet-text` | Rich text, typewriter, fonts, layout, glyph cache |
| `velvet-audio` | Buses, music/SFX/voice, spatial 2D, streaming |
| `velvet-input` | Actions, contexts, remapping, devices |

### World

| Crate | Responsibility |
|-------|----------------|
| `velvet-ecs` | Entities, components, systems, queries, deferred commands |
| `velvet-scene` | Scene graph, load/unload, additive scenes, prefabs |
| `velvet-ui` | Game + editor widgets, layout, themes, focus |

### Language

| Crate | Responsibility |
|-------|----------------|
| `velvet-script-lexer` | Tokenization |
| `velvet-script-syntax` | Green/red tree (rowan-style) for tools |
| `velvet-script-parser` | CST/AST with error recovery |
| `velvet-script-ast` | Typed AST nodes |
| `velvet-script-hir` | High-level IR after name resolution |
| `velvet-script-types` | Type checker |
| `velvet-script-bytecode` | Opcode definitions and modules |
| `velvet-script-compiler` | HIR → bytecode |
| `velvet-script-vm` | Sandboxed VM, coroutines, limits |
| `velvet-script-format` | Formatter |
| `velvet-script-lsp` | Language server |

### Gameplay Modules

| Crate | Responsibility |
|-------|----------------|
| `velvet-story` | Characters, dialogue, choices, rollback, galleries |
| `velvet-play` | Maps, physics façade, AI utilities, cameras |
| `velvet-rpg` | Stats, inventory, quests, parties |
| `velvet-action` | Combat, weapons, perception, score, quick restart |

### Tools & Runtime

| Crate | Responsibility |
|-------|----------------|
| `velvet-project` | `velvet.project` format, project model |
| `velvet-build` | Asset pipeline, packaging, export helpers |
| `velvet-cli` | `velvet` command-line interface |
| `velvet-editor` | Velvet Studio |
| `velvet-runtime` | Player/runtime binary entry |
| `velvet-test-utils` | Shared test helpers and fixtures |

## Plugin Model

Every subsystem registers through `Plugin`:

```rust
pub trait Plugin: Send + Sync + 'static {
    fn name(&self) -> &'static str;
    fn dependencies(&self) -> &[PluginId];
    fn build(&self, app: &mut App) -> Result<(), PluginError>;
    fn finish(&self, app: &mut App) -> Result<(), PluginError> { Ok(()) }
}
```

- Topological sort with cycle detection.
- Conditional enablement via project config.
- Version compatibility checks at registration.
- Core never imports Story/Play/RPG/Action.

## Schedules

Default system schedules (order fixed; labels extendable):

```text
PreStartup → Startup → PostStartup
PreUpdate → FixedUpdate* → Update → PostUpdate
PreRender → Render → PostRender
```

Fixed timestep for physics/gameplay determinism; variable for rendering and UI.

## Data Flow (Frame)

1. Poll window/input → Input plugin updates action state.
2. Advance time; run FixedUpdate N times; run Update.
3. Asset hot-reload ticks; script budgets consume coroutine work.
4. Scene/UI prepare draw lists.
5. Render submits batches; Audio mixes buses.
6. Diagnostics record frame stats.

## Extension Points

- Custom `Plugin` crates outside the workspace.
- Custom importers in Assets.
- Custom materials/shaders in Render.
- Host functions exposed to Velvet Script under sandbox policy.
- Future: Live2D, skeletal animation, networking as optional plugins.

## Save Format Boundary

Internal ECS/component layouts may change. Saves go through **versioned DTOs** in dedicated modules (`save` packages), never raw world dumps.

## Security Boundaries

- VM sandbox: instruction/memory/recursion limits; no arbitrary FS/network by default.
- `unsafe` requires SAFETY comments and tests.
- `cargo-deny` enforces licenses and advisories in CI.
