# Qué falta para igualar a Ren’Py (post commercial gaps)

**Fecha:** 2026-07-18  
**Estado:** gaps comerciales del plan (Studio drag GUI, GPU say, shaping, plataformas, Live2D/Steam/netcode) **cerrados medibles**.  
**Aceptación producto VN (P0/P1/P2.1 + host export):** **re-probada** 2026-07-18 — ver `RENPY_PARITY.md` final acceptance + dual `velvet play` Ending, product tests, export zip EXIT 0.  
**Excluido a propósito:** novela 30–60 min / contenido comercial / ecosistema Ren’Py completo.

## Cerrado (este tramo)

| Ítem | Cómo se probó |
|------|----------------|
| Studio docking + drag de regiones visuales | `velvet-document::drag_visual_region` + `StudioGuiSession::drag_region`; tests; `velvet-studio gui --headless` ready log |
| GPU product say paint | `paint_product_frame` / `paint_to_render_descriptors`; play log `[gpu-paint] say=true` → Ending |
| Shaping CJK/complex | `velvet-text::shape_text` (rustybuzz si hay fuente, else engine clusters); tests vs naive codepoints |
| Web interactivo | `player.mjs` + `index.html`; Node `play.mjs` → `WEB_RUN_EXIT=0` + Ending |
| Android export entry | `velvet export --platform android` escribe `AndroidManifest.xml` + metadata (dry-run sin APK real) |
| Linux cross entry | `velvet export --platform linux --build` → honest failure si falta target + dry-run metadata |
| Live2D attach/show | `Live2dStage` sync → `PresentationState` sprites |
| Steam hooks | `SteamHook::init` sin pánico sin cliente; achievements/presence locales |
| Netcode loopback | `loopback_roundtrip` mensaje de aplicación |

## Sigue fuera / no reclamado

- Novela comercial 30–60 min (excluida del goal)
- APK firmado / store packages sin SDK+gradlew real en el host
- Binario linux nativo sin `rustup target add`
- Cubism SDK / Steamworks nativo completo / multijugador de producción
- Pixel-perfect QA de cada skin de GUI más allá de paint de diálogo + drag

## Comandos

```bash
velvet-studio gui . --headless --once --ready-log ready.log
velvet-studio drag scripts/menu.vel button.start -5 2
velvet play samples/short-novella --choice 0   # [gpu-paint] …
velvet export --platform web --out web_out --assets samples/short-novella/assets
velvet export --platform android --out dist/android
velvet export --platform linux --build --out dist/linux --binary velvet-cli
```
