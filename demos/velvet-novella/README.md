# Velvet Novella — Luz de Estación

Novela visual corta con **menú de título** y host de producto.

## Pipeline (honesto)

| Capa | Qué usa este demo |
|------|-------------------|
| Archivo | `story/main.vel` |
| Parse | `velvet_script_parser` (AST) |
| Runtime | **`StoryProgram` + `VnSession`** (product IR) |
| Paint | menú TTF a resolución de ventana · juego `paint_product_session` |
| **VS2** (HIR → types → OpVs2 → VM) | **No** es el runtime de este demo (VS2 sigue en alpha) |

Misma extensión `.vel`, pero **no** es “todo el juego en VS2 completo”.

```
Title menu (compose = tamaño ventana, máx. 1920 arista)
  → VnSession / StoryPlayer (product)
  → paint_product_session → softbuffer (1:1 o letterbox si cap)
```

## Render

- **Compose:** tamaño de la ventana (adaptativo), arista máx. 1920 px (softbuffer CPU)  
- **Ventana:** arranca **maximizada** a la pantalla primaria  
- **Sin 4K fijo** (evita lag al mover/redimensionar)  
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
