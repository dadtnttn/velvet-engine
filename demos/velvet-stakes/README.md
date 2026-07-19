# Velvet Arcana — Nightfall Casino (local)

Balatro-style demo with **casino title menu** (reference art), illustrated cards,
and deal animation.

## Run (local)

```powershell
cd C:\Hijosdelsol\VelvetEngine
cargo run -p velvet-stakes --release
```

Headless:

```powershell
cargo run -p velvet-stakes -- --headless
```

## Menu (módulos propios)

UI en `src/ui/` (`theme`, `menu`, `hud`).  
Arte **original** generado (la imagen de referencia solo inspiró el estilo):

- `data/ui/menu_bg.jpg` — lobby  
- `data/ui/menu_panel.jpg` — columna de botones  
- `data/ui/logo_emblem.jpg` — emblema  

Botones: **START RUN · COLLECTION · SHOP · OPTIONS · QUIT**

## Cards

`data/art/`: strike, guard, fireball, focus, bash  

## Play controls

| Key | Action |
|-----|--------|
| ↑↓ / Enter | Menu |
| 1–8 | Select |
| P | Play hand |
| D | Discard |
| Esc | Pause / back |
