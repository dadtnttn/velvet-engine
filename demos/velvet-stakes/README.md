# Velvet Arcana — Nightfall Casino (local)

Balatro-style demo. **Author languages** drive the product:

| Layer | File | Role |
|-------|------|------|
| **Velvet Script 2** | `data/ui/main_menu.vel` | Typed menu structure, copy, actions, icons, shortcuts, enabled state |
| **Velvet Story** | `data/story/main.vstory` | Flow: title → play → Night Market → next blind → settlement |
| **Velvet Style** | `data/styles/casino.vcss` | CSS look + JS-lite `@script` (`dealHand`, `menu.open`) |
| **Rust host** | `src/` | Window, paint, input, `stakes.*` + `style.*` commands |

## Run (local)

```powershell
cd C:\Hijosdelsol\VelvetEngine
cargo run -p velvet-stakes --release
```

### Dev / live reload (HTML-like)

Keep the game open and edit author files on disk — no quit/relaunch for the VS2 menu, styles, images, or story:

```powershell
cargo run -p velvet-stakes --release -- --dev
```

| Watch | Path | On change |
|-------|------|-----------|
| VS2 menu | `data/ui/main_menu.vel` | Reparse typed blueprint; bad edits keep the last valid menu |
| Style | `data/styles/casino.vcss` | Reparse; UI/motion use new sheet (bad parse keeps last good) |
| Images | `data/ui/*`, `data/art/*.jpg` | Reload buffer for next paint |
| Title | VS2 copy + system display serif | One-line art-deco wordmark anchored at left |
| Menu BG | `data/ui/menu_bg_city.png` | Panoramic neon skyline, lounge, bar, and card table (16:9) |
| Gameplay BG | `data/ui/gameplay_bg_night_broker.png` | Clean neon duel plate behind the live HUD and cards |
| Market BG | `data/ui/night_market_bg.png` | Clean shopkeeper environment behind live stock and controls |
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

1. `main_menu.vel` compiles to a typed `ScreenBlueprint`
2. `.vstory` calls `stakes.boot` → loads `casino.vcss`
3. VS2 button actions route story by stable names, independent of visual order
4. Menu buttons resolve descendant selectors and `:hover`, `:focus`, `:active`, `:disabled`
5. `stakes.deal` → `@script fn dealHand` + `@keyframes deal`

## Menu

UI paint in `src/ui/` (`theme`, `menu`, `hud`, `buttons`).  
Art under `data/ui/` (original assets).

Buttons: **NEW RUN · COLLECTION · NIGHT MARKET · OPTIONS · LEAVE TABLE**

## Cards

`data/art/`: strike, guard, fireball, focus, bash  

## Play controls

| Key | Action |
|-----|--------|
| Mouse / ↑↓ / W S / Enter | Hover, navigate, and activate menu |
| C / S / O / Esc | Authored menu shortcuts |
| 1–8 | Select |
| P | Play hand |
| D | Discard |
| Esc | Pause / back |
| Mouse | Select cards; Play Hand, Discard, and Pause buttons |
| Market: ←→ / A D / Enter | Select and buy a card |
| Market: R | Reroll stock with progressive cash cost |
| Market: C / Space | Continue to the next blind |
