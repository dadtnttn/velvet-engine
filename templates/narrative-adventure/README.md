# Narrative Adventure Template

Hybrid story project: branching dialogue with light world-state (`has_key`,
`reputation`, `forest_depth`). Enables `story` + `play` + `ui` so you can later
attach maps and triggers from `velvet-play`.

## Layout

- `velvet.project`
- `scripts/main.vel` — crossroads loop with a key item flag
- `assets/` — backgrounds and UI chrome

## Next steps

1. Add a tilemap via the `top-down-rpg` example patterns.
2. Bridge `velvet-story` events to play triggers.
3. Validate: `velvet project info --validate`.
