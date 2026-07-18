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
