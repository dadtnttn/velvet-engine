# Visual Novel Template

Starter Velvet project for dialogue-driven stories.

## Layout

- `velvet.project` — modules: `story`, `ui`, `audio`
- `scripts/main.vel` — sample route with a choice and two endings
- `assets/` — place backgrounds, portraits, and music here

## Suggested next steps

1. Replace character names and colors.
2. Add portraits under `assets/characters/`.
3. Expand `state { }` with route flags.
4. Run `velvet script check scripts/main.vel`.
5. Open in Studio: `velvet-studio open .`

## Limitations

This template is content-only. A full runtime player shell is provided by the
`visual-novel` example crate in the engine workspace, not by this folder alone.
