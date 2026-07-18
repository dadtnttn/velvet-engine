# Tutorial 03 — Top-Down Play (RPG / Action)

Combine `velvet-play` world stepping with optional RPG dialogue or action combat.

## Choose a template

**RPG-leaning**

```bash
velvet new ruins --template top-down-rpg --out .
```

**Action-leaning**

```bash
velvet new arena --template top-down-action --out .
```

## Project modules

`top-down-rpg` enables approximately:

```text
play, rpg, story
```

Dependencies resolve to include `ecs` / `core` when validated:

```bash
velvet project info --path ruins --validate
```

You should see a resolved module order in the validation output.

## Dialogue content (RPG)

Edit `scripts/main.vel` scenes such as `talk_elder`. Host code typically:

1. Builds a `TileMap` and `PlayWorld`
2. Spawns a player with `spawn_player`
3. On interact, loads the matching story scene and presents UI

Reference implementation: `examples/top-down-rpg`.

## World step (conceptual Rust)

```rust
use velvet_math::Vec2;
use velvet_play::prelude::*;

let map = TileMap::/* construct or load */;
let mut world = PlayWorld::new(map);
world.spawn_player(Vec2::new(64.0, 64.0), 120.0);
world.set_player_input(Vec2::new(1.0, 0.0));
world.step(1.0 / 60.0);
```

See crate docs and unit tests in `velvet-play` for map construction helpers available in your revision.

## Action loop

`examples/action-arena` shows:

- weapon fire / projectiles
- enemy perception
- score and quick restart

Template hooks in `scripts/main.vel`:

- `on_level_start`
- `on_wave_cleared`
- `on_player_down`

Wire them from your game code after compiling with `velvet-script-compiler` or by calling story scenes for banners.

## Export dry-run

```bash
velvet export --out dist --binary my-game --assets assets --multi
```

Writes per-platform dry-run folders and `multi-platform-export.json`. This does
**not** cross-compile unless you pass `--build` with toolchains installed.

## Performance note

For a quick microbench of world stepping and script compile (engine workspace):

```bash
cargo run -p velvet-bench --release
```

## Limits

- No built-in multiplayer.
- Tilemap editor is not part of Studio MVP (use external tools + loaders).
- Physics is a lightweight façade, not a full Box2D clone.

You now have the three starter paths: hello tooling, VN content, and top-down modules.
