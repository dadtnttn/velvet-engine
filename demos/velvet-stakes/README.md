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
Arte **original** (referencias de estilo; no se empaquetan capturas del usuario):

- `data/ui/menu_bg.jpg` — lobby  
- `data/ui/logo_emblem.jpg` — emblema  
- `data/ui/buttons/plate_selected.jpg` / `plate_normal.jpg`  
- `data/ui/buttons/icon_*.jpg` — star / cards / chip / gear / power  

Módulo `src/ui/buttons.rs`: placas + iconos + marco dorado con diamantes.  
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
