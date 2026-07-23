# Velvet Engine — Integration and crate-justification matrix

This document answers two different questions that must not be confused:

1. **Is a crate justified?** It needs a clear responsibility, a stable boundary, direct tests, or an independently usable product surface.
2. **Is the engine connected?** Subsystems must meet through explicit contracts and end-to-end tests. They should **not** all import one another; that would create cycles and make the engine harder to change.

Velvet follows a one-way dependency model:

```text
foundation
  ↓
engine services
  ↓
content + language + gameplay modules
  ↓
product orchestration (Story / CLI / Studio / Runtime / demos)
```

Cargo prevents dependency cycles. The integration suite proves the important seams where independent modules meet.

## Retention rule

A crate remains separate when at least one of these is true:

- it defines a reusable public contract with more than one consumer;
- it is an optional capability that users should be able to exclude;
- it has a distinct stability or safety boundary;
- it is a compiler phase whose input/output can be tested independently;
- it is a binary/product boundary;
- it materially reduces rebuild scope or isolates platform dependencies.

A small crate is a **merge candidate** when it has only one consumer, no independent tests, no optionality, and no meaningful API boundary. Line count alone is not a justification. Thin compiler crates are kept only while their phase contracts remain independently tested.

## Foundation crates

| Crate | Responsibility and reason to exist | Connected through | Verification |
|---|---|---|---|
| `velvet-core` | Shared configuration, diagnostics, plugin contracts and engine-level errors. It must stay free of product policy. | `velvet-app`, runtime services, CLI diagnostics | workspace unit tests; `story_core_wiring.rs` |
| `velvet-math` | Deterministic vectors, transforms, bounds, easing and RNG used by render and gameplay without pulling either one in. | render, scene, play, action, anim, cellular | crate tests; `ecs_scene.rs`; `full_pipeline.rs` |
| `velvet-time` | Fixed-step and timer policy, separated so simulations do not depend on a window loop. | app schedules, animation, gameplay loops | crate tests; app/integration tests |
| `velvet-events` | Typed event transport and history shared by otherwise independent systems. | app, scene and gameplay orchestration | crate tests; `ecs_scene.rs` |
| `velvet-crypto` | Bounded hashing/signing helpers used by packaging or deterministic metadata without coupling those users to an external service. | build/project tooling | crate tests and strict input limits |

## Engine-service crates

| Crate | Responsibility and reason to exist | Connected through | Verification |
|---|---|---|---|
| `velvet-app` | Owns schedules, resources and system execution. It orchestrates services but does not implement rendering or gameplay. | core, time, events and product hosts | crate tests; `story_core_wiring.rs` |
| `velvet-assets` | Handles virtual paths, loaders, bundles and hot-reload policy. Platform/file concerns stay out of render and story code. | image, audio, render, project/build | crate tests; asset/image tests |
| `velvet-image` | CPU image and SVG/path processing independent from GPU upload. | assets, render, editor | crate tests; editor and render builds |
| `velvet-text` | Text layout, markup, shaping and GPU glyph descriptors. It is usable by UI and story without depending on either product. | UI, story product frames, render | crate tests; `script_text_ui.rs` |
| `velvet-render` | GPU-facing cameras, textures, sprites, batching and presentation descriptors. | app/product presenters, text, scene | crate tests; `render_audio_input.rs` |
| `velvet-audio` | Mixer, buses, voices and music transitions, isolated from narrative commands. | story host commands and app runtime | crate tests; `render_audio_input.rs` |
| `velvet-input` | Bindings, actions, replay and virtual controls. Window events are translated here before gameplay sees them. | app, play, editor/runtime hosts | crate tests; `render_audio_input.rs` |
| `velvet-ui` | Retained UI tree and widget/layout contracts. Product-specific screens remain outside it. | text, input, story/product shells | crate tests; `script_text_ui.rs` |
| `velvet-ecs` | Entity/component storage and queries, independent from a specific game genre. | scene and gameplay modules | crate tests; `ecs_scene.rs` |
| `velvet-scene` | Hierarchy, prefabs and scene transitions above ECS but below product story/game rules. | ECS, assets, app | crate tests; `ecs_scene.rs` |

## Content and build crates

| Crate | Responsibility and reason to exist | Connected through | Verification |
|---|---|---|---|
| `velvet-document` | Loss-aware authoring document and visual/advanced regions. It prevents Studio edits from destroying hand-written source. | editor, story-language conversion | crate tests; `document_roundtrip.rs` |
| `velvet-project` | Canonical `velvet.project` model, module graph and validation. | CLI, editor, build and runtime | crate tests; CLI/runtime checks |
| `velvet-build` | Packaging, archive creation, localization and platform manifests. | project model, CLI export, runtime binary | crate tests; export/round-trip tests |

## Velvet Script pipeline

Each compiler phase has a narrow data contract. This is deliberately modular, but these crates must continue to justify themselves with independent tests. If a phase loses independent consumers and tests, it should be merged with its neighbor.

| Crate | Phase boundary | Why it is separate / proof |
|---|---|---|
| `velvet-script-syntax` | Shared token/keyword vocabulary | Prevents lexer, parser and tooling from inventing incompatible names; crate tests |
| `velvet-script-lexer` | UTF-8 source → tokens | Independently fuzzable/testable lexical boundary |
| `velvet-script-ast` | Parsed source representation | Shared by parser, formatter and compiler; exhaustive matches enforced by compilation |
| `velvet-script-parser` | tokens → AST + diagnostics | Parser tests and formatter round trips |
| `velvet-script-resolve` | names/imports → resolved symbols | Independent symbol-resolution tests |
| `velvet-script-types` | Script type vocabulary and checks | Explicit type contract for HIR/compiler; retained while independently tested |
| `velvet-script-hir` | Lowered typed/intermediate model | Decouples source syntax from code generation; lowering tests |
| `velvet-script-bytecode` | Stable opcode/module representation | Compiler and VM meet here without importing each other’s internals |
| `velvet-script-compiler` | AST/HIR → bytecode | Compile tests and `full_pipeline.rs` |
| `velvet-script-vm` | Sandboxed bytecode execution and limits | VM tests and `full_pipeline.rs` execute compiled functions |
| `velvet-script-stdlib` | Audited host-independent standard functions | Name/signature uniqueness and behavior tests |
| `velvet-script-format` | AST → stable source | Round-trip tests cover all statement variants, including host/presentation commands |
| `velvet-script-lsp` | Editor protocol projection | Keeps protocol dependencies out of compiler core; completion/hover tests |
| `velvet-script-i18n` | Script diagnostic/localization vocabulary | Keeps locale data out of parser/compiler logic; crate tests |
| `velvet-script-corpus` | Shared valid/invalid language fixtures | Reused regression corpus; merge if it stops serving multiple phases |
| `velvet-script-vs3` | Official general game-logic language; usable alpha | Edition-gated semantic frontend, bytecode v2, bounded VM, persistent sessions, cooperative tasks, and capability-limited host ABI; classic story runtime remains separate |
| `velvet-script-layers` | Typed story/UI/world/fx/audio layer stack | Shared by Studio and product UI; stack and screen-blueprint tests |

## Narrative and presentation

| Crate | Responsibility and reason to exist | Connected through | Verification |
|---|---|---|---|
| `velvet-story-lang` | Writer-friendly `.vstory`/story surface lowered into executable product representations. | parser/compiler contracts, document/editor, story runtime | parser/lowering tests; `document_roundtrip.rs` |
| `velvet-story` | Product VN runtime: choices, waits, host calls, saves, rollback, localization, presentation frames. | script/story language, style, text, audio/render hosts, gameplay bridges | extensive crate tests; `full_pipeline.rs`; `story_core_wiring.rs`; `story_play_rpg.rs` |
| `velvet-style` | Cascade, values, animations and scriptable visual tokens, separate from any one renderer. | Story product UI, Studio and Velvet Stakes | crate tests; style/story host tests |

## Gameplay modules

These are optional libraries. They connect through data/events/host calls rather than making the core depend on a genre.

| Crate | Responsibility and reason to exist | Connected through | Verification |
|---|---|---|---|
| `velvet-play` | Shared 2D map, collision, navigation, camera and interaction primitives. | Story variables/host orchestration, RPG and Action | crate tests; `story_play_rpg.rs`; `full_pipeline.rs` |
| `velvet-rpg` | Stats, inventory, quests, equipment, shops and progression. | Play entities and Story outcomes | crate tests; quest scenarios in `full_pipeline.rs` |
| `velvet-action` | Weapons, projectiles, aim, hitscan, score and optional room recipes. | Play math/entities and story-driven action beats | crate tests; `full_pipeline.rs`; action examples |
| `velvet-cards` | Catalog, deck validation and zones without embedding a complete game. | Velvet Stakes product host | crate tests; `card-duel`; `velvet-stakes` tests |
| `velvet-anim` | Tweens, timelines, transforms and story-host animation commands. | math, story host and UI/render consumers | crate tests; host-command tests |
| `velvet-cellular` | Optional alpha falling-sand simulation with its own world/material contract. | CPU color-buffer bridge, particles/projectiles and cellular demos | large unit/integration suite; `cellular-arena`; `cellular-lab` |

## Product and tooling boundaries

| Crate | Responsibility and reason to exist | Connected through | Verification |
|---|---|---|---|
| `velvet-cli` | Stable command surface for check/play/new/export/launch. | project, build, story and runtime | CLI tests and workspace build |
| `velvet-editor` | Alpha Studio product: visual/script/nodes authoring over shared document and language APIs. | document, project, story-lang, style, layers, LSP | editor tests; `document_roundtrip.rs` |
| `velvet-runtime` | Minimal shippable host binary, kept separate from authoring dependencies. | project/build output and runtime services | all-target build and runtime smoke paths |

## Quality crates

| Crate | Responsibility and reason to exist | Verification |
|---|---|---|
| `velvet-test-utils` | Shared deterministic fixtures without exposing test-only APIs in production crates. | consumed by package/integration tests |
| `velvet-integration-tests` | Cross-crate seam tests. This is the primary proof that modularity has not become disconnection. | seven integration files listed below |
| `velvet-bench` | Wall-clock and workload probes isolated from correctness tests. | builds under all targets; run explicitly for performance work |

## Proven cross-crate seams

| Seam | Contract under test | Test file |
|---|---|---|
| compiler → bytecode → VM | source compiles and functions execute with VM limits | `full_pipeline.rs` |
| story language → runtime → choice/save/load/localization/rollback | authored story reaches deterministic endings and survives serialization | `full_pipeline.rs`, `story_core_wiring.rs` |
| story → play → RPG/action | narrative variables unlock interactions, quests and action outcomes | `story_play_rpg.rs`, `full_pipeline.rs` |
| script → text → UI | script-facing presentation data becomes layout/UI output | `script_text_ui.rs` |
| render + audio + input | independent services can be composed in one host path | `render_audio_input.rs` |
| ECS → scene hierarchy | entities, components and scene transitions share consistent identity | `ecs_scene.rs` |
| Studio document ↔ source | visual edits round-trip while advanced regions are preserved | `document_roundtrip.rs` |

## Product proof surfaces

Demos and examples are not counted as engine internals, but they prove that public APIs are consumable:

- `velvet-novella` — narrative product host and menu/presentation path;
- `velvet-stakes` — cards + style + layers + product UI integration;
- `card-duel` — minimal cards consumer;
- `cellular-arena` and `cellular-lab` — interactive and headless cellular consumers;
- `visual-novel`, `branching-story`, `top-down-rpg`, `action-arena`, `hotline-rush`, `hybrid-demo`, `hello-velvet` — focused API examples.

## Required quality gate

The workspace is considered integrated only when all four commands pass from the repository root:

```bash
cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features
cargo test --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

CI runs the same gate and also builds API documentation with warnings denied. A subsystem is not “connected” merely because it compiles; an important seam needs an integration test or a product consumer listed above.
