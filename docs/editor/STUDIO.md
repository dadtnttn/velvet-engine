# Velvet Studio

Velvet Studio is the project tooling surface for Velvet Engine. It ships as the `velvet-studio` binary with:

1. **Docking GUI model** (`velvet-studio gui`) — hierarchy, assets, visual canvas, inspector, scripts, console.
2. **Canvas drag** — same pure API as the document layer (`drag_visual_region` / `StudioGuiSession::drag_region`).
3. **CLI shell** for project tools (hierarchy, check, assets, patch-visual, launch).

## GUI

```bash
# Headless ready path (CI / no display)
velvet-studio gui ./my_game --headless --once --ready-log ready.log

# Demo drag on a visual region (preserves @advanced)
velvet-studio gui ./my_game --document scripts/main_menu.vel \
  --drag-region button.start --dx -4 --dy 2 --save

# Brief OS window attempt
velvet-studio gui ./my_game --window --once
```

Dock panels (always initialized when ready):

| Panel | Zone |
|-------|------|
| hierarchy | left |
| assets | left |
| canvas | center |
| inspector | right |
| scripts | right |
| console | bottom |

Drag math lives in `velvet-document` (`drag_visual_region`, `hit_test_visual`, `resize_visual_region`). The GUI only owns selection, pointer state, and save.

## CLI drag (same path)

```bash
velvet-studio drag path/to/menu.vel button.start -5 3
```

## Other commands

```bash
velvet-studio new my_game --template visual-novel --out .
velvet-studio open ./my_game
velvet-studio hierarchy ./my_game
velvet-studio check ./my_game
velvet-studio regions file.vel
velvet-studio patch-visual file.vel button.start text "Play"
velvet-studio launch ./my_game --choice 0 --lang es
```

Templates: `visual-novel` | `narrative-adventure` | `top-down-rpg` | `top-down-action`

## Architecture

```text
velvet-editor (lib + velvet-studio bin)
  ├── gui            — StudioGuiSession, docking, drag host
  ├── document_edit  — disk patch + drag_region_on_disk
  ├── studio         — shell / StudioApp
  ├── project_browser, commands, console, inspector, …
  └── velvet-document — regions, UiDesigner, drag geometry
```

## Screen layers (árbol de pantallas)

Layers are a **tree**: root pantallas + **subcapas** (e.g. Main Menu → Settings,
Scene → Decisions). Each node has its own **pixel resolution**. The design surface
letterboxes to that aspect; widgets keep percent layout; the inspector shows **px**.

Default tree (visual novel):

```text
Main Menu
  ├── Nueva partida
  ├── Continuar
  ├── Configuracion
  └── Salir / confirm
Scene
  ├── Dialogue
  └── Decisions
HUD / Overlay
```

| Action | Control |
|--------|---------|
| Cycle layers | `[` `]` or PageUp / PageDown |
| Select layer | Click row in **LAYERS** |
| Expand / collapse | Click left third of a parent row (`+` / `-`) |
| Unlock / lock | `U` |
| New sublayer under active | `N` |
| Add mobile sublayer | `M` (390×844 under menu) |
| Res presets | `Ctrl+3` HD · `Ctrl+4` phone · `Ctrl+5` landscape |

**main_menu** re-locks when another root branch is active. Resolution changes
animate the canvas frame (~280ms).

## Dual mode (simplified + advanced)

| Mode | Key / API | Role |
|------|-----------|------|
| **Simplified** | default; `1` in window | Drag-and-drop canvas on `@visual` regions |
| **Advanced** | `2` / Tab toggle | Same `.vel` file as script buffer (`set_advanced_source`) |

```bash
# Headless dual-mode ready (CI)
velvet-studio gui ./my_game --headless --once --ready-log ready.log

# Interactive dual-mode window (Tab = toggle, S = drop button, drag = move)
velvet-studio gui ./my_game --interactive
# equivalent: velvet-studio gui ./my_game --window --once=false

# Brief OS window paint then exit (CI / probe)
velvet-studio gui ./my_game --window --once
```

Mutations always go through `velvet-document` (`UiDesigner` / `drag_visual_region`) so `@advanced` is preserved.

## Status

| Feature | Status |
|---------|--------|
| Project new / scaffold | Implemented |
| Hierarchy + symbols | Implemented |
| Check diagnostics | Implemented |
| Interactive shell | Implemented |
| Docking panel model | Implemented |
| Dual mode simplified/advanced | Implemented (`StudioEditorMode`) |
| Visual canvas drag + palette drop | Implemented (session + window paint) |
| Advanced region preserve on drag | Implemented |
| Windowed dual-mode softbuffer host | Implemented |
| Live GPU WYSIWYG theme editor | Not claimed (softbuffer dock paint is the bar) |
