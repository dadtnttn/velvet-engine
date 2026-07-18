# Performance Notes

## Design targets (aspirational)

| Area | Direction |
|------|-----------|
| 2D sprite throughput | Batching in `velvet-render` |
| Story text | Typewriter + layout caching in `velvet-text` |
| Play world | Simple kinematic step; avoid O(n²) where easy |
| Script | Bytecode VM with caps |

These are **not** contractual frame-time SLAs.

## Measuring

```bash
cargo run -p velvet-bench --release
```

Scenarios:

1. **Script compile** — parse+compile a medium sample source repeatedly  
2. **Story pump** — many `tick`/`advance` operations on a compact program  
3. **Play step** — spawn many entities, step world for N frames  

Record host CPU model and release vs debug (debug is intentionally slower; workspace `opt-level` for dev is 1).

## Known hotspots / risks

- **Physics rebuild each step** in play world can dominate large solid counts.
- **GPU** submission cost appears only in windowed runners with real frames — benches do not cover it.
- **Audio** mixing cost not included in velvet-bench.
- **String-heavy VN lines** allocate; reuse buffers in host shells when possible.

## Guidance

- Prototype in debug; profile ship targets in `--release`.
- Prefer fewer huge textures; atlas where possible.
- Cap simultaneous particles / projectiles (action module).
- Use headless runners in CI for functional checks, not FPS claims.

## Future

- Optional Tracy/puffin hooks
- Render stats already partially exist (`velvet-render` stats/profile modules) — wire to HUD in demos
