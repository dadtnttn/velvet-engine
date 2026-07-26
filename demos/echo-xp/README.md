# ECHO//XP — Analog Horror OS Investigation Demo

> [!IMPORTANT]
> Binary artwork and audio are intentionally excluded from the repository. Use your own
> licensed media under `data/assets/`; the demo source remains usable without publishing
> those files. A redistributable replacement pack may be documented separately later.

**ECHO//XP** is a diegetic Windows XP operating system simulation investigation game built inside **Velvet Engine**.

## Concept
The player acts as an archive operator investigating **CASE 001 — MARA V.**. Through an early-2000s desktop terminal interface, the operator reads assignment emails, inspects case dossiers, analyzes anomalous photographic evidence, listens to audio recordings, restores deleted system files, and submits discrepancy verdicts to Archive Control.

## Key Features
- **Diegetic OS Interface**: Fully functional desktop shell with Start menu, taskbar, tray clock, wallpaper, desktop icons, and window management (focus, drag, z-index, minimize, close).
- **Analog Horror Narrative**: Psychological dread through subtle continuity discrepancies, dynamic file shifts, identity anomalies, corrupt Clippy assistant interactions, and glitch overlays.
- **Velvet Script 3 Integration**: Authoritative narrative rules, evidence progression, anomaly states, save state export/import, and ending triggers live in VS3 (`.vel`).
- **Software Renderer**: Built with `softbuffer`, rendering retro XP window themes, 9-slice frames, pixel fonts, icon atlases, and CRT corruption effects.
- **Headless Acceptance Suite**: Automated deterministic test mode (`--headless`).
- **Screen Capture Tool**: Automated stage screenshot generator (`--capture-screen`).

## Controls
- **Left Mouse Click**: Select icons, press buttons, focus windows, navigate menus.
- **Double Click**: Launch applications from desktop icons.
- **Titlebar Drag**: Click and hold window titlebars to move them around the desktop.
- **Escape**: Close Start Menu.
- **Alt + F4**: Exit application.

## How to Run

### 1. Import Assets
Run the Python asset importer with a compatible archive that you obtained and are allowed
to use. The repository does not provide or download that archive:
```bash
python demos/echo-xp/tools/import_winxp_assets.py path/to/your-assets.zip
```

### 2. Launch Interactive Game
```bash
cargo run -p echo-xp
```
or double-click `demos\echo-xp\run.bat`.

### 3. Run Headless Acceptance Tests
```bash
cargo run -p echo-xp -- --headless
```

### 4. Generate Screen Captures
```bash
cargo run -p echo-xp -- --capture-screen desktop artifacts/echo-xp/desktop.png
cargo run -p echo-xp -- --capture-screen inbox artifacts/echo-xp/inbox.png
cargo run -p echo-xp -- --capture-screen case-file artifacts/echo-xp/case-file.png
cargo run -p echo-xp -- --capture-screen photo artifacts/echo-xp/photo.png
cargo run -p echo-xp -- --capture-screen tape artifacts/echo-xp/tape.png
cargo run -p echo-xp -- --capture-screen classifier artifacts/echo-xp/classifier.png
cargo run -p echo-xp -- --capture-screen ending artifacts/echo-xp/ending.png
```

## Architecture
- `src/main.rs`: CLI parser (`--headless`, `--capture-screen`), winit event loop, frame presenter.
- `src/app.rs`: Main application loop and engine integration.
- `src/assets.rs`: Asset manager loading textures, fonts, audio buffers, icon atlases.
- `src/audio.rs`: System event sound player (`rodio`).
- `src/input.rs`: Mouse position, click, double click, and key state tracker.
- `src/model.rs`: VS3 snapshot parser (`FrameView`) and application kinds.
- `src/render.rs`: 800x600 software pixel buffer renderer.
- `src/save.rs`: Persistence manager (`APPDATA/VelvetEngine/echo-xp/save.json`).
- `src/windows.rs`: Desktop Window Manager.
- `src/desktop.rs`: Desktop icons, taskbar, start menu, tray clock.
- `src/apps/`: Individual applications (Inbox, Case Files, Photo Viewer, Tape Player, Classifier, Recycle Bin, Dialogs).
- `data/`: VS3 script modules (`game.vel`, `module.vel`, `state.vel`, `flow.vel`, `case001.vel`, `anomalies.vel`, `persistence.vel`, `acceptance.vel`).
