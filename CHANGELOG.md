# Changelog

All notable changes to Velvet Engine are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- **`velvet-pixel`**: hand-shaded pixel art built in code. Shaded primitives and signed-distance
  bodies (`velvet_pixel::sdf`) go into a material buffer, lit with banded ramps and dithering, with
  cast shadows, rim light and selective outlines. It is the toolkit Demon Queen's loading screens
  were drawn with, moved into the engine unchanged (same pixels).
- **`velvet_ui::world_tags`**: labels anchored to things in the world. They sit over their thing,
  keep off the HUD and off each other (the important ones choose first), hang from a pole, glide
  to their place and fade instead of jumping.
- **`velvet_render::retarget`**: clips from one rig onto another (bone maps, rest-direction
  alignment, unmapped bones riding their parents), with a Mixamo-to-Mixamo map and the
  quaternion helpers it uses.

### Changed

- Demon Queen left the engine's repository for its own private one beside it; it uses the
  engine's crates by path.

## [0.3.0] - 2026-10-16

Velvet Engine 0.2 was never tagged on its own: it shipped inside Demon Queen 0.2.0. This release
gathers everything since 0.1.0, and the workspace version moves to 0.3.0.

### Added

- **3D renderer** (`velvet-render`): toon shading with depth ink lines and grading
  (`ToonStyle3D`, `Finish3D`), HDR with bloom, cached directional shadows, blend modes, up to 32
  point lights with per-mesh light masks, GPU skinning, screen-space global illumination and
  ambient occlusion, reflections with culling, toon fire (`ToonFire3D`), water bodies with ripples
  and spray (`WaterLife3D`), BC7 textures, FXAA and FSR 1 upscaling, and output at the screen's
  native resolution.
- **GPU crowds** (`velvet-render::crowd3d`): thousands of animated soldiers of one model drawn as a
  single mesh; a compute pass animates them (walk, swing, thrust, shot, fall, cape, flight) from
  eight numbers per instance (`upload_crowd`, `set_crowd`).
- **`Material3D::sky`**: an unlit sky dome that is never hazed.
- **AV1** (`velvet-avif`): AVIF pictures and AV1 clips in IVF, decoded by rav1d (pure Rust, no
  system codec).
- **VLZ** (`velvet-vlz`): a block-compressed, random-access resource container; specification in
  [`docs/formats/VLZ_FORMAT.md`](docs/formats/VLZ_FORMAT.md).
- **Navigation and QA**: `velvet-nav` (navmeshes from a game's own walkability, A* with a funnel
  pass, ORCA crowds) and `velvet-qa` (a walker that plays a zone and reports where it gets stuck and
  the frame times).
- **World generation**: `velvet-terrain` (noise, erosion, meshing, LOD, biomes), `velvet-liquid`
  (shallow water, SPH), `velvet-dungeon` (deterministic dungeon blueprints).
- **Guiones** (`velvet-guion`): dialogue and visual-novel scenes written as Markdown, played as
  conversations or compiled to story nodes; stage directives include the new `@recuerdo` (a
  remembered picture, shown with a ribbon and kept in the gallery).
- **Velvet Editor** (`velvet-editor3d`, **alpha**): docked scene tree, inspector, files and
  scripts around the engine's own 3D viewport.
- **Video** (`velvet-video`): a frame-sequence player for openings and cutscenes.
- **Docs**: [`docs/README.md`](docs/README.md) index, the module map now covers all 65 crates,
  `docs/formats/` for shared file formats, and the plan for a JavaScript framework
  ([`JS_FRAMEWORK_PLAN.md`](docs/architecture/JS_FRAMEWORK_PLAN.md),
  [ADR 0009](docs/adr/0009-rust-and-js-runtimes.md)).

### Changed

- Workspace version **0.3.0** (`velvet-cellular` stays `0.1.0-alpha.1`).
- The VLZ specification moved to `docs/formats/VLZ_FORMAT.md`.

### Demon Queen (flagship demo)

Source only in this repository; its story, locales and art stay private. Highlights of this cycle:
the castle court (audiences, works, lessons, training, the cell, nights) and a dating sim whose
rewards are remembered pictures; the war table with quick battles and a turn-based campaign
(`demos/demon-queen/war`: battle simulation, campaign rules, field and map scenery); battle banners
that follow their troops through a reusable world-anchored label layout; campaign armies drawn as
formations, destination chips, multi-turn marches, and militias that make every taken province a
fight; redesigned Persona-style menus; terrain detail and lighting for the battlefields. The game's
GPU HUD modules moved from `src/bin/` to `src/overlay/` (`src/bin/` holds only binaries).

## [0.1.0] - Unreleased

### Added

- **Typed application states** (`velvet-app`): `init_state` / `set_state` /
  `state` / `in_state`, pending transitions applied at frame start with
  `StateTransitioned` events through the double-buffered registry.
- **Change-filtered ECS queries** (`velvet-ecs`): `iter_changed`,
  `query2_changed`, `map_changed`, `for_each_changed_mut`, `count_changed`,
  and the reusable `ChangedCursor` over existing per-entity tick tracking.
- **VS3 typed ABI slice**: `ModuleDescriptor` / `FunctionDescriptor` (params,
  result type, purity, sync, cost, capabilities), `AbiRegistry` with
  duplicate-service rejection, request validation, and a `SimulatedHost`
  proven end-to-end driving real VS3 tasks without VelvetEngine.
- **VS3 engine adapters crate** (`velvet-script-vs3-engine`): the checked
  `velvet-math` bridge moved out of the language core (VS3 roadmap phase 3),
  plus first host modules `velvet.time` and `velvet.input`: typed descriptors
  with reference snapshot hosts validated per request, driven end-to-end by
  real tasks under capability policies.

### Fixed

- **Workspace quality gate restored for demos**: removed invisible U+200B
  characters from `sam-tomboy` localization strings, fixed rustfmt drift in
  `sam-tomboy` / `summoner-emporium`, and cleared all clippy warnings so the
  full workspace passes `clippy -D warnings` again.

### Changed

- **Sand / cellular stack marked ALPHA** (`0.1.0-alpha.1`): `velvet-cellular`,
  demos `cellular-arena`, example `cellular-lab`, template `cellular-sandbox`,
  and docs (`CELLULAR.md`, MODULES, README). Not a stable product surface.

### Added

- **VS3 versioned local packages**: `velvet.package.toml`, stable
  `package.module` imports, semantic-version constraints, offline path dependency
  resolution, canonical `velvet.lock`, SHA-256 source integrity, stale-lock
  rejection, CLI `vs3 lock`, and `Seventeen` as a locked package proof.
- **VS3 host/task hardening**: strict capability/service identifiers, validated
  request construction, non-empty pending tickets, terminal task state, preserved
  frontend warnings, and boundary tests for zero request budgets.
- **VS3 explicit function exports**: `export function` defines the public
  module/host API while unexported helpers remain private; modules without exports
  retain the legacy all-functions-public behavior for gradual migration.
- **VS3 nominal source modules**: `import "path.vel" as module`, nested
  `module.function()` calls, isolated private state, deterministic internal names,
  cycle/path diagnostics, filesystem root containment, CLI support, formatter/LSP
  awareness, and `Seventeen` as the product-level integration proof.
- **velvet-stakes `--dev` live reload**: watches `data/styles/*.vcss`,
  `data/ui/*`, `data/art/*`, `data/story/*.vstory` via `HotReloader` +
  `LiveDevSession` — reapply styles/images (and soft story re-boot on title)
  without process restart. `cargo test -p velvet-stakes` covers the path.
- **velvet-stakes driven by `.vstory` + `.vcss`**: flow in
  `demos/velvet-stakes/data/story/main.vstory`; casino look/motion in
  `casino.vcss`; Rust host exposes `stakes.*` and forwards `style.*`
  (`dealHand` deal, `menu.open` emit).
- **`.vcss` = CSS + JS-lite**: stylesheets combine CSS (selectors, `@keyframes`,
  cascade) with `@script` (`let`/`fn`/`for`/`play`/`animate`/`on`). Story
  `style.call` / `style.emit`. Docs: `docs/language/VELVET_STYLE.md`.
- **velvet-cards** authoring tools (catalog, deck validation, seeded shuffle,
  library/hand/discard zones) + CLI `velvet cards validate|zones` — tools for
  authors, not a playable card game. Hotline Miami–like remains roadmap
  **deferred / future** (`docs/architecture/ROADMAP.md`).
- Demo **card-duel** (`demos/card-duel`): windowed card duel with title menu,
  how-to, battle, pause, and result screens (uses `velvet-cards` zones).
- Demo **velvet-stakes** (`demos/velvet-stakes`): Balatro-like poker (chips×mult,
  blinds, select/play/discard) with menus — fan pre-alpha, not affiliated.
- **velvet-anim** animation/VFX tools: poses, tweens, presets (deal, fade, shake…),
  multi-target director, `.vanim` scripts, story host `anim.fx` / `anim.move` /
  `anim.script` (docs: `docs/language/VELVET_ANIM.md`).
- **3D image tools** in `velvet-anim`: `Pose3D` channels, `project_image`,
  `ImageBillboard`, keyframe `Timeline` / `ChannelTrack`, story
  `anim.billboard` / `anim.pose3d` / `anim.track`. Premade pack cutscenes
  removed from the host API; optional samples only under `recipes`.
- **Tools-first across cards / action / anim**: `docs/architecture/TOOLS.md`;
  Hotline room loop → `velvet_action::recipes`; `WeaponLoadout` / `WeaponKits`
  as tools; demos stay examples only.
- Phase 0: workspace, architecture docs, ADRs, CI skeleton, dual license.
- Phase 1 (core): `velvet-math`, `velvet-time`, `velvet-events`, `velvet-core`, `velvet-app`
  (plugins, schedules, resources, headless + winit window runners).
- CLI `velvet` with `doctor`, `version`, `run` (`--headless`), `init`, `project info`.
- `velvet-project` RON project model.
- Phase 2: `velvet-render` (wgpu GpuContext, sprites, cameras, letterbox, profiles),
  `velvet-audio` (buses/voices/fades), `velvet-input` (actions + winit map),
  `velvet-assets` (handles/loaders).
- Phase 3 (start): `velvet-ecs` (entities, components, commands, queries).
- Example `hello-velvet` uses real Input/Assets/Audio/Render plugins (headless).
- Phase 4: Velvet Script lexer, parser, AST, bytecode, compiler, VM (limits/sandbox);
  CLI `velvet script check|run`; example `examples/hello-script.vel`.
- Phase 5: Velvet Story runtime (dialogue, choices, vars, history, prefs),
  versioned saves, `visual-novel` and `branching-story` demos.
- Phase 6: Velvet Play (tilemaps, physics, camera, triggers, A*, PlayWorld).
- Phase 7: Velvet RPG (stats, inventory, quests, party, shops) + top-down-rpg demo.
- Phase 8: Velvet Action (weapons, projectiles, perception, score) + action-arena demo.
- Scaffold crates for remaining script tooling / studio polish.
