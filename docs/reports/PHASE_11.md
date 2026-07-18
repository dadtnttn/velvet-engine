# Phase 11 Report — Templates & Documentation Bulk

## Goal

Make `templates/` real content roots and expand technical docs so onboarding and
status reporting do not depend on tribal knowledge.

## Delivered

### Templates
Each of the following has `velvet.project`, `scripts/main.vel`, `README.md`, `assets/`:

- `visual-novel` — choice + two endings
- `narrative-adventure` — item flag loop
- `top-down-rpg` — NPC dialogue set
- `top-down-action` — lifecycle hooks + banner scenes

### Docs
- `docs/architecture/MODULES.md` — crate map + maturity
- Tutorials `01_hello`, `02_visual_novel`, `03_topdown`
- Reports PHASE_9–12, LINE_COUNT, SECURITY, PERFORMANCE, LIMITATIONS

## Honest status

- Templates are **content**, not full runnable Cargo packages (examples/ remain the demos).
- Tutorial code samples may need small API adjustments as crates evolve — prefer examples as source of truth for compile-checked hosts.
- Documentation line counts intentionally contribute to repo size; they are not “fake LOC” in crates.

## Exit criteria

| Criterion | Status |
|-----------|--------|
| Four templates non-empty | Done |
| Tutorials for three paths | Done |
| Module reference doc | Done |
