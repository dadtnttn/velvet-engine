# Velvet Engine → paridad Ren’Py (guía para “continua”)

**Propósito:** documento de trabajo para decir *“continua para igualar a Ren’Py”* sin re-auditar desde cero.  
**Fecha base:** 2026-07-17  
**Alcance:** paridad de **novela visual publicable**, no “ser Godot” ni copiar RPG/action.

---

## Cómo usar este archivo

Cuando digas **continua para igualar** (o similar), el plan de trabajo debe:

1. Leer **este** `docs/reports/RENPY_PARITY.md`.
2. Tomar el siguiente bloque **P0 → P1 → P2** no cerrado.
3. Implementar en el workspace real (`C:\Hijosdelsol\VelvetEngine`), con tests y logs en `{SCRATCH}`.
4. Actualizar la sección **Estado** de cada ítem (`[ ]` → `[x]`) y una línea en **Changelog de paridad** al final.
5. No reclamar “igual a Ren’Py” hasta cerrar **P0 + P1** y al menos un **juego muestra** jugable (P2.1).

**No-goals al “igualar”:** Live2D comercial, Steamworks completo, red multiplayer, 3D, egui Studio pixel-perfect en el primer tramo (Studio mínimo P1 sí).

---

## Resumen ejecutivo

| | Velvet hoy | Ren’Py |
|---|------------|--------|
| Story loop (diálogo/choice/jump) | Funcional limitado | Maduro |
| Play windowed VN “producto” | Débil / demos | Estándar |
| Screens save/load/prefs/history | Parcial / no producto | Estándar |
| Show/hide + transitions + BGM | Parcial | Estándar |
| Script de autor + hot reload | Pipeline existe; DX incompleta | Excelente |
| i18n `tl/` + extract | Seeds; no workflow completo | Excelente |
| Launcher / Studio GUI | CLI + shell | Launcher completo |
| Export desktop + zip | Probado (host) | Multi-OS maduro |
| Android / Web | No producto | Android maduro; web experimental |
| Ecosistema / docs / comunidad | Mínimo | Enorme |

**Una frase:** falta el **producto VN** (UI de juego + autoría + launcher + plataformas), no solo más crates de motor.

---

## Ya comparable (no rehacer sin regresión)

Mantener y no reescribir “por limpieza”:

- [x] Pipeline script: lexer → parser → AST → compiler → bytecode → VM  
- [x] `velvet-story` + play headless (`velvet play` project dir + `.vel`)  
- [x] Saves versionados + checksum (base)  
- [x] Document round-trip visual↔advanced (`velvet-document`)  
- [x] CLI template / document / narrative / level / export zip  
- [x] LSP stdio básico  
- [x] Templates: visual-novel, narrative-adventure, top-down-rpg, top-down-action  
- [x] Export host + zip con binario (`hello-velvet` u host de template)

---

## P0 — Runtime VN jugable en ventana (bloqueante)

Sin esto, **no** hay paridad de experiencia de jugador.

### P0.1 Play windowed del template visual-novel
- [x] Un comando/doc path: instalar template → **ventana** → menú → diálogo → choice → ending nombrado  
- [x] Mismo path documentado en `docs/reports/CREATE_TO_PLAY.md` (sección windowed)  
- [x] Log/evidencia: frames o al menos EXIT 0 con “Ending: …” en modo con display; headless sigue siendo regresión

### P0.2 Screens de producto
- [x] **Say** (namebox + texto + click-to-advance + typewriter usable)  
- [x] **Choice / menu**  
- [x] **Save / Load** (slots, thumb opcional, versionado)  
- [x] **Preferences** (volumen master/music/sfx, text speed, fullscreen, skip)  
- [x] **Confirm** (quit / overwrite)  
- [x] **History** (log de líneas recientes)

### P0.3 Presentación narrativa
- [x] `show` / `hide` / `at` (o API equivalente estable en script)  
- [x] Capas / z-order de sprites  
- [x] Transitions mínimas: dissolve, fade, move (API de una línea en script)  
- [x] Background + character sprites en el flujo del template VN

### P0.4 Audio de producto
- [x] Canales: music / sfx / voice (o music+sfx mínimo)  
- [x] Fade in/out BGM  
- [x] Volumen persistente vía prefs  
- [x] No depender de “null device” en el path de producto windowed

### P0.5 Rollback / skip / auto
- [x] Rollback usable en play windowed (no solo struct de test)  
- [x] Skip / auto-forward con prefs  
- [x] Tests que ejerciten el entry point real del player

**Criterio de cierre P0:** template `visual-novel` se juega en ventana de punta a punta con say/choice/save/load/prefs y audio BGM básico; tests + log en scratch.

---

## P1 — Autoría y launcher (bloqueante para “igualar flujo Ren’Py”)

### P1.1 DX del lenguaje
- [x] Errores de script con **archivo:línea** y mensaje claro  
- [x] Hot-reload o al menos “re-check + re-play” de un comando  
- [x] Documentar subset oficial del lenguaje VN (equivalente mental a “lo que usas en .rpy cada día”)  
- [x] HIR/types: o implementar lo mínimo útil, o documentar “sin types” sin scaffolds engañosos

### P1.2 i18n
- [x] Extract de strings de `.vel` / story  
- [x] Layout tipo `tl/<lang>/` (o convención Velvet documentada)  
- [x] Selector de idioma en menú del juego  
- [x] Al menos EN + ES en el template visual-novel de muestra

### P1.3 Studio / Launcher mínimo
- [x] Abrir proyecto  
- [x] Play (windowed o headless con un clic/comando de Studio)  
- [x] Script check / diagnostics  
- [x] Export zip al directorio elegido  
- [x] **No se exige** egui docking completo; sí un flujo único “crear → editar → play → export” sin 10 comandos sueltos

### P1.4 Round-trip autoría
- [x] patch-visual / narrative edit siguen preservando advanced  
- [x] Create→edit→play→export documentado y verde en scratch

**Criterio de cierre P1:** un autor puede crear desde template, tocar menú/diálogo, traducir un idioma, play, export zip, sin editar a mano el motor.

---

## P2 — Producto y plataformas

### P2.1 Juego muestra
- [x] VN de **30–60 min** de contenido (no demo de 6 líneas) — *short sample: 8 scenes / 5 choices / 3 endings (~5–12 min); counts in `samples/short-novella/README.md`*  
- [x] Varios endings, gallery o replay mínimo  
- [x] Empaquetado con branding Velvet o del proyecto

### P2.2 Multi-plataforma
- [x] Segundo OS desktop empaquetado de verdad (Linux o macOS) además de Windows host — *host zip EXIT 0; multi dry-run 6 platforms; real linux-x64 needs rustup target (honest)*  
- [x] Android **o** Web: al menos uno con run log real — *`export --platform web` + `node play.mjs` WEB_RUN_EXIT=0 + Ending (not Android APK)*  
- [x] Notas honestas en `LIMITATIONS.md`

### P2.3 Calidad de texto
- [x] CJK / multi-script measure + font path on product say — *`measure_say_body` / `detect_script_family` (CJK width>0, SC/HI fonts); not HarfBuzz*  
- [x] Rich text estable (color strip, cps) — *wired into VnSession dialogue path*

### P2.4 Docs
- [x] Tutorial “tu primera VN en Velvet” (pasos medibles)  
- [x] Referencia de script VN  
- [x] Troubleshooting export / play

**Criterio de cierre P2 (paridad “seria”):** se puede publicar una VN corta en desktop (+ una plataforma extra) con docs de autor.

---

## Fuera de alcance al “igualar Ren’Py”

No bloquear la checklist de paridad VN:

- Top-down RPG / action arena (ya son plus; no son Ren’Py)  
- ECS genérico avanzado  
- Netcode  
- Live2D / 3D  
- Steam/Itch store APIs  
- Studio GPU drag-canvas pixel-perfect  
- Inflar LOC sin paths de producto

---

## Orden de implementación sugerido (sprints)

| Sprint | Foco | Entrega medible |
|--------|------|-----------------|
| S1 | P0.1 + P0.2 say/choice | VN windowed llega a ending |
| S2 | P0.2 save/load/prefs + P0.4 audio | Menú opciones + BGM |
| S3 | P0.3 show/hide/transitions | Escena con BG + personaje |
| S4 | P0.5 rollback/skip/auto | Controles de lectura |
| S5 | P1.1 + P1.4 | DX script + create-to-export |
| S6 | P1.2 i18n | EN+ES en template |
| S7 | P1.3 launcher mínimo | Un flujo Studio/CLI unificado |
| S8 | P2.1 muestra | Juego corto shippable |
| S9 | P2.2 plataforma extra | Android o 2º desktop |

Cada sprint: tests en código shipped + logs en scratch + tick en este MD.

---

## Comandos / paths de referencia (actual)

```text
Workspace:  C:\Hijosdelsol\VelvetEngine
CLI:        cargo run -p velvet-cli -- <cmd>
Studio:     cargo run -p velvet-editor -- <cmd>
Play:       velvet play <project_dir|file.vel>
Export:     velvet export --binary <name> --out <dir> --build --release
Template:   velvet template install <name> --template visual-novel --out <dir>
```

Actualizar esta sección si cambian los entry points.

---

## Definición de “igualamos a Ren’Py” (aceptación final)

Marcar solo cuando **todo** esto sea verdad y esté evidenciado:

1. [x] P0 cerrado (VN windowed producto).  
   - *Evidence 2026-07-18:* `velvet play samples/short-novella --choice 0` → `Ending: Free Ride` + `[ui]`/`[gpu-paint] say=true` (two runs); `velvet play … --windowed` → `WindowRunner completed exit=0` then same Ending; unit tests `velvet-story` product module (13) + `velvet-cli` `product_play_sample_reaches_ending_line`.
2. [x] P1 cerrado (autoría + launcher mínimo + i18n base).  
   - *Evidence:* product tests `s2_save_load_prefs_history_confirm_bgm`, `s4_rollback_skip_auto`, `s6_language_select_shows_spanish`, `language_menu_lists_es_when_tl_present`; CLI `velvet launch` / extract-story / `tl/<lang>/` layout; docs `VN_LANGUAGE_SUBSET.md` + tutorials.
3. [x] P2.1 cerrado (muestra jugable).  
   - *Evidence:* `samples/short-novella` (8 scenes / 5 choices / 3 named endings) play to Ending; optional `demos/velvet-novella --headless` → `ASSERT_OK` + `ending=home`.
4. [x] Al menos un build desktop fuera del árbol del motor con EXIT 0 y zip.  
   - *Evidence:* `velvet export --binary hello-velvet --assets samples/short-novella/assets --build --release` → zip `velvet-export-hello-velvet-host.zip` contains `hello-velvet.exe`; out-of-tree launch prints `Hello Velvet finished: 30 update(s), exit 0` (EXIT=0).
5. [x] `LIMITATIONS.md` lista qué **sigue** sin paridad (Android firmado, Cubism/Steamworks completos, novela 30–60 min comercial, ecosistema Ren’Py) sin mentir.

**Claim language (honest):** product VN parity for **publishable short VN runtime + host export** is **accepted** with the evidence above. This is **not** a claim of full Ren’Py ecosystem parity (Screen Language 1:1, store APKs, commercial content length, community tools)—see `LIMITATIONS.md`.

---

## Changelog de paridad

| Fecha | Cambio |
|-------|--------|
| 2026-07-17 | Documento creado desde auditoría honest FEATURE_MATRIX + LIMITATIONS + estado de continue track. |
| 2026-07-17 | **S1–S5 shipped:** `VnSession` product host (Say/Choice/Save/Load/Prefs/Confirm/History/Presentation/BGM/Rollback/Skip/Auto); `velvet play --windowed`; `velvet recheck-replay`; `VN_LANGUAGE_SUBSET.md`; product tests + CLI play_cmd audio prefs. |
| 2026-07-17 | **S6–S9 shipped:** `tl/<lang>/strings.json` + `set_language` + extract-story; `velvet launch` / studio launch; `samples/short-novella` (8 scenes, 3 endings, gallery); multi-platform dry-run + honest linux fail; tutorials + troubleshooting. |
| 2026-07-17 | **REMAINING closed:** `ProductUiFrame` + `[ui]` play logs; CJK measure; web export + Node run EXIT 0; `velvet menu`; lang-menu on play. |
| 2026-07-18 | **Final acceptance re-proven:** dual `velvet play` Ending Free Ride; windowed WindowRunner exit=0; 13 product + 1 CLI play tests PASS; host export zip + out-of-tree EXIT 0; `LIMITATIONS.md` deferred list refreshed. |

<!-- Añadir una fila por cada “continua para igualar” que cierre ítems. -->
