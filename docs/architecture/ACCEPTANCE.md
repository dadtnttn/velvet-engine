# Acceptance Criteria

## Global (project complete)

- [ ] `cargo test --workspace --all-features` passes.
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings` passes.
- [ ] `cargo fmt --all --check` passes.
- [ ] `tokei` reports ≥ 35k production + ≥ 15k tests/tools own LOC (significant total ≥ 50k).
- [ ] Six demos run without panics on the documented reference platform.
- [ ] Architecture, security, and performance reports exist under `docs/`.
- [ ] No undeclared `todo!()` in features marked complete.
- [ ] Limitations listed honestly.

## Phase 0

- [ ] Vision, architecture, dependencies, risks, ADRs, roadmap published.
- [ ] Workspace members compile.
- [ ] CI workflow present.
- [ ] Conventions documented.

## Phase 1

- [ ] `App` + plugins + schedules + events + time.
- [ ] Empty window via `velvet run` (or runtime) on host.
- [ ] Update loop unit-tested.

## Phase 2

- [ ] Sprites, camera, input actions, basic audio.
- [ ] Hello Velvet demo runs.
- [ ] Asset hot-reload works in dev.

## Phase 3

- [ ] ECS entities/components/systems/prefabs.
- [ ] Scene load/update/unload tests.

## Phase 4

- [ ] Velvet Script compile + execute.
- [ ] Errors include file:line:column.
- [ ] Extensive lexer/parser/VM tests.

## Phase 5

- [ ] Visual novel playable end-to-end.
- [ ] Save/load and branching choices.

## Phase 6–8

- [ ] Play baseline; RPG cycle; Action Arena cycle.

## Phase 9+

- [ ] Studio minimum: project, hierarchy, inspector, run.
- [ ] Export desktop package path documented and exercised on Windows.
