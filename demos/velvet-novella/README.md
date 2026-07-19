# Velvet Novella — Luz de Estación

Novela visual corta con **menú de título** y host de producto:

`Title menu` → `VnSession` → `paint_product_session` → softbuffer

## Run

```bash
cargo run -p velvet-novella --release
cargo run -p velvet-novella --release -- --headless
```

Or `run.bat` from the demo folder / repo root.

## Title menu

| Input | Action |
|-------|--------|
| ↑ / ↓ (W/S) | Mover selección |
| Enter / Space / Click | Confirmar |
| Esc | Salir |

Opciones:

1. **Nueva partida** — entra a la historia  
2. Continuar — (stub)  
3. Galería — (stub)  
4. Opciones — (stub)  
5. **Salir**

Art: `data/ui/menu_bg.jpg` · título con fuente serif (Georgia/Times vía fontdue).

## In-game controls

| Input | Action |
|-------|--------|
| Space / Click / Enter | Advance or confirm choice |
| Up / Down (W/S) | Move choice |
| 1–4 | Select choice arm |
| R | Volver al menú |
| Esc | Quit |

## Story

`story/main.vel` — Spanish branching novella with multiple endings.
