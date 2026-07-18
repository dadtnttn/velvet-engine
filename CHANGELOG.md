# Changelog

All notable changes to Velvet Engine are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [0.1.0] - Unreleased

### Added

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
