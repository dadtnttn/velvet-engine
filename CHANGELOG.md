# Changelog

All notable changes to Velvet Engine are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [0.1.0] - Unreleased

### Changed

- **Sand / cellular stack marked ALPHA** (`0.1.0-alpha.1`): `velvet-cellular`,
  demos `cellular-arena`, example `cellular-lab`, template `cellular-sandbox`,
  and docs (`CELLULAR.md`, MODULES, README). Not a stable product surface.

### Added

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
- **3D image FX** in `velvet-anim`: perspective billboards (`Pose3D`,
  `project_image`), foil phase, card flip helper, **pack-open generator**
  (`PackOpenFx`) + story `call anim.pack_open` for TCG-style pack reveals.
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
