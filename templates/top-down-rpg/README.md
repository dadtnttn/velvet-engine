# Top-Down RPG Template

Project skeleton for a 2D RPG: `play` + `rpg` + `story`.

## What this template includes

- `velvet.project` with RPG modules
- `scripts/main.vel` — NPC dialogue scenes (`talk_villager`, `talk_elder`, …)
- Empty `assets/` for tilesets, sprites, and UI

## What you still wire in code

Tilemaps, party stats, inventory UI, and save slots are implemented by engine
crates (`velvet-play`, `velvet-rpg`) and demonstrated in `examples/top-down-rpg`.
This template is the **content root** those systems load.

## Commands

```bash
velvet project info --validate
velvet script check scripts/main.vel
velvet-studio open .
```
