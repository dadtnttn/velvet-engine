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

## Status

| Feature | Status |
|---------|--------|
| Project new / scaffold | Implemented |
| Hierarchy + symbols | Implemented |
| Check diagnostics | Implemented |
| Interactive shell | Implemented |
| Docking panel model | Implemented |
| Visual canvas drag | Implemented (data + session API; OS paint is brief window probe) |
| Advanced region preserve on drag | Implemented |
| Live GPU WYSIWYG theme editor | Not claimed (drag correctness + panels are the bar) |
