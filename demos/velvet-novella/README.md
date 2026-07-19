# Velvet Novella — Luz de Estación

Novela visual corta con **menú de título** y host de producto.

## Pipeline (honesto)

| Capa | Qué usa este demo |
|------|-------------------|
| Archivo | `story/main.vel` |
| Parse | `velvet_script_parser` (AST) |
| Runtime | **`StoryProgram` + `VnSession`** (product IR) |
| Paint | menú 4K TTF · juego `paint_product_session` |
| **VS2** (HIR → types → OpVs2 → VM) | **No** es el runtime de este demo (VS2 sigue en alpha) |

Misma extensión `.vel`, pero **no** es “todo el juego en VS2 completo”.

```
Title menu (4K compose)
  → VnSession / StoryPlayer (product)
  → paint_product_session → softbuffer (bilinear from 4K)
```

## Render

- **Internal:** `3840×2160` (4K UHD) for menu + product paint  
- **Window default:** 1920×1080, letterbox **bilinear** from 4K  
- Menu fonts: Constantia / Segoe UI via **fontdue** (not 8-bit bitmap)

## Run

```bash
cargo run -p velvet-novella --release
cargo run -p velvet-novella --release -- --headless
```

## Title menu

| Input | Action |
|-------|--------|
| ↑ / ↓ (W/S) | Mover selección |
| Enter / Space / Click | Confirmar |
| Esc | Salir |

1. **Nueva partida**  
2. Continuar (stub)  
3. Galería (stub)  
4. Opciones (stub)  
5. **Salir**

## In-game

| Input | Action |
|-------|--------|
| Space / Click / Enter | Advance / confirm |
| Up / Down | Choice |
| R | Volver al menú |
| Esc | Quit |

## Story

`story/main.vel` — Spanish branching novella, several endings.
