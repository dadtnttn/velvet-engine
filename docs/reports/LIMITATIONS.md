# Limitations (Honest) — production track

This document states what Velvet Engine **does not** claim today, and what **was proven** with logs.

## Proven this continue pass (SCRATCH evidence)

Scratch root used for this verification pass:
`%LOCALAPPDATA%\Temp\grok-goal-bc18e6bac70e\implementer`

| Capability | Proof |
|------------|--------|
| Build / test / fmt / clippy gates | `gate_summary.txt` BUILD=0 TEST=0 FMT=0 CLIPPY=0 |
| **Project-directory play** (`velvet play <project_dir>`) | `project_play.log` — VN Ending: Warm Lights; narrative “ending reached” |
| RPG/action template install + script check | `templates_rpg_action.log` |
| Export `--build --release` + launch outside tree | `export_run.log` — `EXPORT_RUN_EXIT=0` |
| **Export zip archive** containing host binary | same log — `velvet-export-hello-velvet-host.zip` lists `hello-velvet.exe` |
| Studio `patch-visual` + advanced preserved | `studio_author.log` |
| Export zip unit tests (shipped APIs) | `roundtrip_tests.log` — `write_directory_zip` / dry_run archive entries |
| Round-trip visual↔advanced document model | `roundtrip_tests.log` — velvet-document + editor document_edit |

## What this pass shipped

- Real **zip archive** on host export (`write_directory_zip` / `list_zip_entries` in `velvet-build`); dry-run and real build both emit archive; unit tests assert binary entry presence.
- Clearer **project-dir play** logging (`playing project … entry …`) via shipped `velvet play <dir>`.
- Studio/editor **command parity** for visual patch remains the real `velvet-studio` / `velvet-editor` binary (`patch-visual`), same `velvet-document` APIs as CLI.

## S1–S5 product VN (2026-07-17)

- `VnSession` product host: Say/Choice/Save/Load/Prefs/Confirm/History/Presentation/BGM/Rollback/Skip/Auto
- `velvet play --windowed` + `velvet recheck-replay`; see `RENPY_PARITY.md` and `VN_LANGUAGE_SUBSET.md`
- Full pixel-perfect GPU dialogue canvas still not claimed; headless product path is the regression bar

## S6–S9 remainder (2026-07-17)

- **i18n:** `tl/<lang>/strings.json`, `set_language`, `velvet play --lang es`, template auto-ES scaffold
- **Launcher:** `velvet launch` / `velvet-studio launch` (check → play → export zip)
- **Sample:** `samples/short-novella` (8 scenes, 3 endings, gallery) — short multi-branch, not 60 min commercial
- **Platforms:** multi dry-run OK; **real** linux-x64 needs `rustup target add` (failure logged); Android/WASM **not** claimed
- **Docs:** `docs/tutorials/04_first_vn_launch.md`, `TROUBLESHOOTING_EXPORT_PLAY.md`
- **Rich text on product path:** `VnSession::show_dialogue_line` runs `say_plain_and_cps` (strips `{color…}`, applies `{cps=N}`)
- **Product UI frames:** `build_product_ui_frame` (namebox/body metrics/choices/lang-menu) logged as `[ui]` on play
- **CJK measure:** unicode-width metrics + font path attachment (SC/HI); **not** full HarfBuzz shaping
- **Web product export:** static HTML + Node `play.mjs` with real `WEB_RUN_EXIT=0` (not Android APK / interactive browser polish)
- **Mini launcher menu:** `velvet menu` (text UI, not egui docking IDE)

## Still incomplete / limited (honest)

### Studio product UI (**ALPHA** — same band as cellular)

- **Maturity:** Studio is **alpha**, labeled the same way as cellular/sand. Expect breaking changes to GUI, VScript surface, and `velvet.studio.json` / `scripts/screens/*`.
- Softbuffer Studio ships **triple mode** (Visual / VScript / Nodes), per-screen documents, layer graph, save `velvet.studio.json`, delete/resize/undo, script typing, F9 play smoke.
- **Not claimed:** full egui docking IDE, multi-select Figma tools, in-window LSP, or GPU WYSIWYG theme editor.
- Dual-mode **regions** remain the document model (`@visual` / `@advanced`); each **pantalla/layer** has its own file under `scripts/screens/`.
- Details: [`docs/editor/STUDIO.md`](../editor/STUDIO.md).

### Language / tools

- HIR/types crates remain thin scaffolds.
- LSP covers common features, not every LSP 3.17 optional capability.
- VS Code extension is a thin client.
- Text uses **rustybuzz** when a system font loads, else the engine cluster shaper (not unicode-width alone).

### Export

- Host package + **zip archive** for the built binary works (`hello-velvet`).
- **Web** interactive browser player + Node runner to ending works.
- **Android** writes real manifest/gradle layout; APK build needs SDK + gradlew (dry-run is the default honest path).
- **Linux cross** entry is real; fails honestly without `rustup target add` and still emits dry-run metadata.
- No signed store installers.

### Integrations

- Live2D-compatible attach/show on presentation (no Cubism SDK runtime).
- Steam hooks no-op safely without client (no full Steamworks).
- Netcode loopback message round-trip only (not production multiplayer).

### Gameplay / demos

- Commercial-scale story packages are not claimed; samples + templates cover core loops.
- Windowed “play any template as full game” still uses examples for Play/Action maps when a display is available.
- **Product VN path proven (2026-07-18):** `velvet play samples/short-novella --choice 0` reaches `Ending: Free Ride` with `[ui]` / `[gpu-paint] say=true`; `--windowed` runs real `WindowRunner` (exit=0) then headless product continue; `cargo test -p velvet-story -- product` (13) and `product_play_sample_reaches_ending_line` PASS; host export zip + out-of-tree `hello-velvet.exe` EXIT 0.

### Explicitly deferred (not Ren’Py product-parity blockers for short VN ship)

- Commercial 30–60 minute authored novella content package.
- Signed Android APK / store installers (needs SDK + gradlew on host).
- Full Cubism Live2D SDK runtime; full Steamworks client; production multiplayer.
- 1:1 Ren’Py Screen Language / ATL / full style system clone.
- Ecosystem/community/docs volume of Ren’Py.

## Process

- Prefer commands + tests over screenshots.
- Do not mark a platform supported without a real run log.
- CLI + data-path Studio commands are the checkable bar; full visual Studio GUI is out of gating unless implemented and proven.
- Final Ren’Py **product** acceptance (P0/P1/P2.1 + host export + honest LIMITATIONS) is documented in `RENPY_PARITY.md` (2026-07-18 re-proof).
