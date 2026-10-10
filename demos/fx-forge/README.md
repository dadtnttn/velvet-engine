# Velvet FX Forge

BitFX Forge-like **procedural pixel VFX generator** — 100% Velvet Engine.

Loopable pixel-art effects (fire, smoke, magic, lightning, portals, DNA helix, etc.) sin dibujar frame a frame. Basado en `crates/velvet-fx`.

## Run

```bash
cargo run -p fx-forge --release
# headless CI check (no window needed for export test)
cargo run -p fx-forge --release -- --headless
```

The UI uses a system font, without borrowing assets from another demo.
Set `VELVET_FONT` to a TTF path if no supported system font is installed.

## Controls (teclado)

| Tecla | Acción (como BitFX) |
|-------|---------------------|
| `R` | **Randomize** — genera preset totalmente aleatorio (headline feature) |
| `M` | Mutate — variación leve del preset actual |
| `1..8` | Presets ejemplo (Fire, Smoke, Magic, Lightning, Portal, DNA, Embers, Heal) |
| `P` | Siguiente paleta (15 paletas) |
| `K` | Siguiente path (Point, Line, Circle, Wave, Spiral, DNA, etc.) |
| `+ / -` | Scale X/Y (el `path width/height` slider que pide la gente a BitFX) |
| `[ / ]` | Frequency (apriete onda/espiral) |
| `, / .` | FPS 6..30 |
| `F` | Cicla canvas 32×32 → 96×96 → 64×96 ... |
| `N` | Cicla frames 8/12/16/24/32 |
| `C` | Toggle additive/glow |
| `Space` | Play / Pause |
| `H` | Toggle help overlay |
| `S` | **Export spritesheet** → `demos/fx-forge/output/<name>.png` + `.json` + `_preset.json` |
| `J` | Save preset JSON solo |
| `Esc` | Salir |

## Export

`S` escribe en `fx_export/`:

- `<name>.png` — spritesheet tiling todos los frames (columns ≈ sqrt(frames), usamos `div_ceil`)
- `<name>.json` — `AtlasMeta` { frame_width, frame_height, frames, fps, columns, rows, sheet_width, sheet_height, palette, seed, duration }
- `<name>_preset.json` — `FxPreset` completo para re-import / tweaks

También exportable vía API:

```rust
use velvet_fx::{FxPreset, FxSim};
let preset = FxPreset::fire();
let sim = FxSim::new(preset);
let sheet = velvet_fx::export::export_spritesheet(&sim, None).unwrap();
std::fs::write("out.png", velvet_fx::export::export_png(&sheet.0).unwrap()).unwrap();
```

O CLI futuro: `cargo run -p fx-forge -- --preset dna --xScale 1.5 --frames 48` (roadmap: wire `velvet-cli fx` subcommand).

## Library `velvet-fx`

`crates/velvet-fx` — crate independiente, sin `winit`/`wgpu`:

- `FxPreset` (serde JSON) — dimensions, frames, fps, palette, path, emitter, particles, seed, additive
- `palette` — 15 paletas (Fire, Frost, Toxic, Arcane, Electric...)
- `path` — PathKind (Point, Circle, Wave, Spiral, **DNA**, Grid, Bezier...) con `scale_x/scale_y` (los sliders pedidos)
- `sim::FxSim::render_frame_at(t)` — determinista, loopable (Seamless/PingPong), pixel-perfect 1–2px squares
- `randomize::random_preset(seed)` / `mutate_preset(base, seed)`
- `export::{export_png, export_spritesheet, export_spritesheet_to_files}`

## Tests

```bash
cargo test -p velvet-fx
cargo check -p velvet-fx -p fx-forge
cargo clippy -p velvet-fx -p fx-forge -- -D warnings
```

## Roadmap

- Import preset JSON (L) en la UI
- `velvet fx` CLI subcommand (ya especulado por el autor de BitFX)
- Paleta custom por picker + import de particle shapes
- Isométrico preview
- Integración directa como panel en `velvet-studio` (`crates/velvet-editor`)
