# Velvet Story — lenguaje narrativo para escritores (**ALPHA**)

> Capa amigable para autores. **IR canónica: `StoryProgram`**. Runtime: **`StoryPlayer`**.
> La lógica que no cabe en las variables de la historia se escribe en **Velvet Script 3**
> (ver [Lógica en Velvet Script 3](#lógica-en-velvet-script-3)).

## En 15 minutos

```velvet-story
scene start

background bedroom
show luna happy

luna:
    Hola.

choice:
    "Seguir":
        goto next

scene next
narrator:
    Fin.
end
```

## Pipeline de producto (columna única)

```text
.vstory → lexer → parser → AST → sema
       → StoryProgram              ← IR canónica
            └→ StoryPlayer         ← runtime (CLI `story run`)
```

Hay un solo camino de ejecución. La antigua salida derivada a OpVs2 (Velvet Script 2) se quitó
el 2026-11-11.

## CLI

```bash
velvet story check stories/main.vstory
velvet story --lang en check stories/main.vstory   # diags es|en|ja|de|zh
velvet story build stories/main.vstory              # resumen del StoryProgram
velvet story run stories/main.vstory --choice 0     # StoryPlayer, sin interfaz
velvet play stories/main.vstory                     # same product boot (.vstory via story-lang)
velvet-runtime stories/main.vstory 0 256            # packaged host headless play
velvet story format stories/main.vstory
velvet story dump-ast stories/main.vstory
velvet story dump-program stories/main.vstory       # el StoryProgram en JSON
velvet story studio-model stories/main.vstory
velvet story extract-loc stories/main.vstory --out tl/source.json
```

Variable de entorno: `VELVET_STORY_LANG=en` (el flag `--lang` tiene prioridad).

## Lógica en Velvet Script 3

Una historia puede declarar arriba su archivo de lógica, en Velvet Script 3 (`// @edition 3`),
con una ruta entre comillas relativa al archivo que la declara:

```velvet-story
logic "reglas.vel"

scene start
set premio = logic.premio
if logic.rico:
    narrator:
        Te sobra el oro.
goto logic.siguiente
```

```velvet
// @edition 3
function premio(v: map) { return v["oro"] * 2 }
function rico(v: map) { return v["oro"] >= 3 }
function siguiente(v: map) {
    if v["premio"] >= 10 { return "tesoro" }
    return "calle"
}
```

- `logic.nombre` llama a la función `nombre` con un solo argumento: un mapa con las variables de
  la historia. Vale como valor (`set`, comparaciones) y como condición (`if logic.rico:`).
- `goto logic.nombre` salta a la escena cuyo nombre devuelve la función; si devuelve `""`, algo
  que no es texto o el nombre de una escena que no existe, la escena sigue con la línea siguiente
  (lo último se avisa al ejecutar). El mapa de ramas cuenta un `goto logic.x` como un salto
  posible a cualquier escena.
- Cada llamada usa una sesión nueva: la lógica no guarda estado propio. Las variables siguen siendo
  de la historia, así que las partidas guardadas no cambian: el hash del programa no incluye la
  lógica, y editar el `.vel` no las invalida.
- Al comprobar la historia, un `logic.x` (en un `goto`, un valor o una condición) sin `logic "…"`
  es el diagnóstico `VST070`, `logic` sin ruta entre comillas es `VST071`, y un archivo de lógica
  que no existe es `VST072`. Al compilar se comprueba además que el archivo compila y exporta cada
  función usada; el error lleva la línea de la llamada.
- El reproductor web (`web_story`) no ejecuta Velvet Script 3: un `goto logic.x` le llega como
  `logic_jump` y lo pasa de largo.

Los guiones en Markdown usan el mismo archivo de lógica con `@decide` (ver
[GUION.md](../formats/GUION.md)).

## Diagnósticos multiidioma y multi-doc

- Códigos estables `VSTxxx`; solo el texto humano se traduce.
- API de proceso: `set_diag_locale` / env (CLI simple).
- API aislada (Studio / hilos concurrentes): `with_diag_locale`,
  `CheckOptions::with_locale`, `check_source_with` — el locale efectivo es
  **thread-local por contexto**, no solo un `RwLock` global.

## Comandos extensibles

Los programadores registran comandos en `CommandRegistry` (p. ej. `combat.start`).
Los escritores llaman:

```velvet-story
call combat.start:
    enemy: forest_guardian
    difficulty: 3
```

## Localización de diálogo

```velvet-story
luna @line.luna_intro_01:
    Buenos días.
```

Si no hay `@id`, se genera un id estable por hash de escena+hablante+texto.

## Errores

Los diagnósticos apuntan a `archivo:línea:columna` del `.vstory` original
(incluye archivos `include` con origen propio cuando aplica).

## Crate

`crates/velvet-story-lang` — ver también [VELVET_2_5.md](./VELVET_2_5.md) y
[VELVET_SCRIPT_2.md](./VELVET_SCRIPT_2.md).

## Guardado (notas)

- Usa **nombres de escena** estables, no índices de instrucción.
- Renombrar escenas requiere migración de saves (documentar en el proyecto).
- Variables con `set` pueden persistirse por nombre vía host / player state.

## Camino de producto `.vel` (runtime `velvet-story`)

El mismo `StoryProgram` alimenta el runtime de novelas visuales
(`StoryPlayer` + `VnSession`): presets say/menu/nvl, mini-ATL,
cinemáticas, galería, sala de música, voces, guardados con foto,
desbloqueos y pantallas de juego. Detalle de sintaxis en
[VN_LANGUAGE_SUBSET.md](../reports/VN_LANGUAGE_SUBSET.md); plantilla en
`templates/visual-novel`.

```bash
velvet narrative preview scripts/main.vel --lines 10
velvet narrative branches scripts/main.vel
velvet narrative check-assets .
velvet play . --lang es
```

## Pantallas de juego y transforms

```vel
screen ticket {
  title: "Ticket"          # prop viva: titulo
  tag: "panel"             # mostrar reemplaza el mismo tag
  text "One evening."      # literal (traducible, typewriter, voz)
  text score               # dinámico: lee la variable `score`
  textbutton keep action Return("keep")
}

transform nod {
  y -6 0.8 ease_in_out, y 0 0.8 ease_in_out, repeat
  on show { scale 1.1 0.2 }
}

scene main {
  show luna with nod
  screen show ticket with fade
  screen call ticket into pick   # modal: resultado en `pick` y `_return`
  screen hide ticket with fade
}
```

- `with` acepta mini-ATL (`move|zoom|shake|fade [segs]`) o un `transform`;
  en pantallas produce fade de entrada/salida (0.30/0.35 s).
- Acciones conocidas validadas: `Return(x)`, `Jump("escena")`,
  `Show("pantalla")` / `Hide("pantalla")`, `Set("var", valor)`;
  otros nombres son acciones custom del host.
- Typewriter (`reveal_game_screen`), voces (`voice_clips_for_screen`,
  la pantalla es la clave de `voice_cast.json`), historial y paint con
  `font_scale` incluidos; rollback cruza llamadas modales.
- i18n: `extract-story` genera `screen:<id>:<n>:screen`; `validate-story`
  las comprueba igual que el diálogo.
