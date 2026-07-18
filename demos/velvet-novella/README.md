# Velvet Novella — product VN windowed host

**Luz de Estación** demo driven by the shared product presentation path:

`VnSession` → `build_product_ui_frame` → `paint_product_session` → `rasterize_product_paint` → softbuffer

Same story-state source as `velvet play` (`[ui]` / `[gpu-paint]`).

## Run

```bash
cargo run -p velvet-novella --release
cargo run -p velvet-novella --release -- --headless
```

Or `run.bat` from the repo root.

## Controls

| Input | Action |
|-------|--------|
| Space / Click / Enter | Advance or confirm choice |
| Up / Down (W/S) | Move choice |
| 1–4 | Select choice arm |
| R | Restart |
| Esc | Quit |

## Story

`story/main.vel` — Spanish branching novella with multiple endings.
