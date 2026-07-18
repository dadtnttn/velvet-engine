# Template: cellular-sandbox

Author-facing starter for **velvet-cellular** (Noita-like / falling-sand core).

This is **not** a full game — it is a sandbox you extend:

1. Register custom materials (or use builtins).
2. Paint the world / load a layout.
3. Step the sim + optional rigid bodies.
4. Render via `session.render` into your own host (wgpu, pixels crate, etc.).

## Minimal code shape

```rust
use velvet_cellular::prelude::*;

fn main() {
    let mut session = CellularSession::with_builtins(WorldConfig::default());
    session.gen_arena(0, 0, 48, 32);
    session.select_preset("Sand");
    session.brush_down(0, 20);
    session.brush_drag(10, 18);
    session.brush_up();
    session.spawn_enemy("slime", 5.0, 12.0);
    session.splatter(-4, 14, 3); // blood
    loop {
        session.tick(1.0 / 60.0);
        let _buf = session.render(-64, -16, 128, 96);
        // upload `_buf.pixels` to your GPU texture
    }
}
```

### Brush modes

Paint, Erase, Replace, Heat, Cool, Ignite, **Bleed**, Dig — see `BrushMode`.

### Enemies

`slime`, `bat`, `brute`, `crawler` or `register_enemy(EnemyDef::new(...))`.

## Project files

- `velvet.project` — project meta (template kind `cellular-sandbox`)
- `src/main.rs` — optional skeleton if you copy into a Cargo package
- `materials/` — place for your JSON material packs (future / hand-authored)

## Docs

See `docs/architecture/CELLULAR.md` in the Velvet Engine repo.
