# Velvet Arcana — Nightfall Casino (local)

Balatro-style demo. **Author languages** drive the product:

| Layer | File | Role |
|-------|------|------|
| **Velvet Story** | `data/story/main.vstory` | Flow: title → blinds → play → result |
| **Velvet Style** | `data/styles/casino.vcss` | CSS look + JS-lite `@script` (`dealHand`, `menu.open`) |
| **Rust host** | `src/` | Window, paint, input, `stakes.*` + `style.*` commands |

## Run (local)

```powershell
cd C:\Hijosdelsol\VelvetEngine
cargo run -p velvet-stakes --release
```

### Dev / live reload (HTML-like)

Keep the game open and edit author files on disk — no quit/relaunch for styles, images, or story:

```powershell
cargo run -p velvet-stakes --release -- --dev
```

| Watch | Path | On change |
|-------|------|-----------|
| Style | `data/styles/casino.vcss` | Reparse; UI/motion use new sheet (bad parse keeps last good) |
| Images | `data/ui/*`, `data/art/*.jpg` | Reload buffer for next paint |
| Story | `data/story/main.vstory` | Soft re-boot when on title (or flagged until title) |

Console prints `dev: reloaded …` lines; window title shows `DEV`.

> Rust `.rs` still needs a rebuild — live mode is for **author assets** (`.vcss`, images, `.vstory`), not cargo recompile.

Headless:

```powershell
cargo run -p velvet-stakes -- --headless
cargo run -p velvet-stakes -- --headless --dev
```

Lib tests (reload path):

```powershell
cargo test -p velvet-stakes
```

## Story ↔ CSS wiring

1. `.vstory` calls `stakes.boot` → loads `casino.vcss`
2. `style.emit` / `event: "menu.open"` → `@script on("menu.open")`
3. `stakes.deal` → `@script fn dealHand` + `@keyframes deal`
4. Menu/HUD resolve `.button` / `.button:selected` from the same sheet

## Menu

UI paint in `src/ui/` (`theme`, `menu`, `hud`, `buttons`).  
Art under `data/ui/` (original assets).

Buttons: **START RUN · COLLECTION · SHOP · OPTIONS · QUIT**

## Cards

`data/art/`: strike, guard, fireball, focus, bash  

## Play controls

| Key | Action |
|-----|--------|
| ↑↓ / Enter | Menu (story resume) |
| 1–8 | Select |
| P | Play hand |
| D | Discard |
| Esc | Pause / back |
