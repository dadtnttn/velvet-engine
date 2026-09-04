# Velvet Story — lenguaje narrativo para escritores (**ALPHA**)

> Capa amigable para autores. **IR canónica: `StoryProgram`**. Runtime de producto:
> **`StoryPlayer`**. OpVs2 se deriva de esa IR (debug / fallback), no es un
> segundo lenguaje ni un lower paralelo del AST.

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
            ├→ StoryPlayer         ← runtime de producto (CLI `story run`)
            └→ OpVs2 (derivado)    ← host secundario / dump-lowered / fallback
```

No uses “AST → lower → OpVs2 only” como arquitectura primaria: el camino de
autores es **StoryProgram → StoryPlayer**.

## CLI

```bash
velvet story check stories/main.vstory
velvet story --lang en check stories/main.vstory   # diags es|en|ja|de|zh
velvet story build stories/main.vstory              # StoryProgram + OpVs2 summary
velvet story run stories/main.vstory --choice 0     # product path first
velvet play stories/main.vstory                     # same product boot (.vstory via story-lang)
velvet-runtime stories/main.vstory 0 256            # packaged host headless play
velvet story format stories/main.vstory
velvet story dump-ast stories/main.vstory
velvet story dump-lowered stories/main.vstory       # secondary OpVs2 dump
velvet story studio-model stories/main.vstory
velvet story extract-loc stories/main.vstory --out tl/source.json
```

Variable de entorno: `VELVET_STORY_LANG=en` (el flag `--lang` tiene prioridad).

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
