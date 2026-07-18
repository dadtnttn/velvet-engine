# Top-Down Action Template

Skeleton for arena / twin-stick style games using `play` + `action`.

## Contents

- `velvet.project`
- `scripts/main.vel` — lifecycle hooks (`on_level_start`, `on_wave_cleared`, …)
- `assets/` — sprites, SFX, particles

## Runtime

Weapons, projectiles, enemy perception, hitstop, and score are in
`velvet-action`. See `examples/action-arena` for a runnable demo. This template
is the content + project file you copy when starting a new action title.

```bash
velvet new my-arena --template top-down-action
```
