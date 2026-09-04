# Mi Novela — plantilla lista estilo Ren'Py

Proyecto jugable de ejemplo. Para hacer TU novela **solo cambia lo de la
tabla**; menus, guardados, rollback, galeria, musica, voces, historial,
idiomas y accesibilidad ya funcionan.

## SOLO CAMBIA ESTO

| Quiero cambiar... | Donde |
|---|---|
| Nombres y colores | `scripts/main.vel`, bloques `character` |
| Retratos | `assets/characters/*.png` (mismo nombre, o cambia `portrait:`) |
| Fondos | `assets/bg/*.png` (o cambia `background "..."`) |
| Dialogos y opciones | `scripts/main.vel`, texto entre comillas y bloques `choice` |
| Finales | escenas `ending_*` + `end "id"` (no olvides su `unlock`) |
| CG desbloqueables | `assets/cg/*.png` + entrada en `gallery.json` + `unlock gallery <id>` |
| Musica | `assets/music/*.wav` + entrada en `music.json` + linea `music` |
| Voces | `voice_cast.json` (`speaker` = id de `character`, `clip_prefix` = inicio del archivo) |
| Traduccion | `tl/es/strings.json` (claves `escena:N:dialogue`; regenera con `velvet localization extract-story`) |

## Comandos de autor

```bash
velvet script check scripts/main.vel
velvet narrative preview scripts/main.vel --lines 10
velvet narrative branches scripts/main.vel
velvet narrative check-assets .
velvet style check styles/ui.vcss                    # hoja de estilo + lint
velvet style dump styles/ui.vcss --class button --state hover
velvet style run styles/ui.vcss dealHand --arg 5     # orquesta el reparto
velvet play .                    # jugar
velvet play . --lang es          # jugar en español
velvet launch .                  # jugar + exportar zip
velvet-studio open .             # abrir en Studio (branches, check-assets, vn-preview)
```

## Estilo (`styles/ui.vcss`)

La hoja de ejemplo usa `calc()`, `clamp()`, `color-mix()`, el hijo
directo (`.say > .namebox`), `@keyframes deal` y un `@script` con
`dealHand` (ver `log: dealing 5` al correrlo). Tokens en `:root`
(`--ink`, `--gold`, `--panel`, `--accent`).

## Idiomas (cualquiera, no solo español)

Esta plantilla trae `tl/en` + `tl/es`. Añade los que quieras — el juego
muestra inglés donde aún no hayas traducido, así que siempre es jugable:

```bash
# al crear el proyecto:
velvet new minovela --template visual-novel --lang fr --lang ja
# o en uno existente (repite para cada idioma):
velvet localization extract-story scripts/main.vel --project . --lang fr
# 1. traduce tl/fr/strings.json (claves escena:N:dialogue, no las renombres)
# 2. valida contra el guion (avisa claves faltantes o viejas):
velvet localization validate-story . --lang fr
# 3. juega:
velvet play . --lang fr
# 4. lista idiomas detectados (lee carpetas tl/*/):
velvet localization langs .
```

El menú de idioma del juego detecta solo las carpetas `tl/`.

## Lo que ya trae

- Pantallas: `screen say_classic|say_minimal|nvl_page|menu` (3 lineas).
- Pantallas de juego: `screen inv(…) { … }` + `screen show|hide|call` (ver ejemplo comentado en `scripts/main.vel`).
- Mini-ATL: `show x at left with shake 0.4 10` (`move|zoom|shake|fade`); tambien `hide x with fade`.
- Transforms con nombre: `transform bob { … }` + `show x with bob` (`parallel`, `choice`, `repeat`, `on show/hide`).
- Texto rico: `{b}{i}{color=#f00}{ruby=furi}`.
- Typewriter + voces tambien en pantallas (`reveal_game_screen`, `voice_clips_for_screen`).
- Traduccion de pantallas: `extract-story` genera claves `screen:<id>:<n>:screen`.
- `unlock gallery|music <id>` en finales + guardado que persiste desbloqueos.
- `gallery.json`, `music.json`, `voice_cast.json` de ejemplo.
- `tl/en` + `tl/es` listos.

## Layout

- `velvet.project` — modules: `story`, `ui`, `audio`
- `scripts/main.vel` — ruta con choice y 2 finales nombrados (`warm`, `cool`)
- `assets/` — placeholders listos para reemplazar (PNG + WAV)

## Limitations

Placeholder art is synthetic. Replace it with your own art before publishing.
A full runtime player shell is provided by the `visual-novel` example crate
in the engine workspace, not by this folder alone.
