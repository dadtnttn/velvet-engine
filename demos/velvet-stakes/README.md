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

## Menu

Background: `data/ui/menu_bg.png`  
Buttons: **START RUN · COLLECTION · SHOP · OPTIONS · QUIT**

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
