# Velvet Story — lenguaje narrativo para escritores (**ALPHA**)

> Capa amigable sobre **Velvet Script 2**. No es una segunda máquina virtual.
> Los archivos `.vstory` se validan y bajan a HIR / `OpVs2` del stack VS2 existente.

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

## Pipeline

```text
.vstory → lexer → parser → AST → sema → lower → Hir + Vs2Unit → Vs2Host (OpVs2)
```

## CLI

```bash
velvet story check stories/main.vstory
velvet story build stories/main.vstory
velvet story run stories/main.vstory --choice 0
velvet story format stories/main.vstory
velvet story dump-ast stories/main.vstory
velvet story dump-lowered stories/main.vstory
velvet story studio-model stories/main.vstory
velvet story extract-loc stories/main.vstory --out tl/source.json
```

## Comandos extensibles

Los programadores registran comandos en `CommandRegistry` (p. ej. `combat.start`).
Los escritores llaman:

```velvet-story
call combat.start:
    enemy: forest_guardian
    difficulty: 3
```

## Localización

```velvet-story
luna @line.luna_intro_01:
    Buenos días.
```

Si no hay `@id`, se genera un id estable por hash de escena+hablante+texto (no depende del número de línea).

## Errores

Los diagnósticos apuntan a `archivo:línea:columna` del `.vstory` original, en lenguaje natural.

## Crate

`crates/velvet-story-lang` — ver también [VELVET_SCRIPT_2.md](./VELVET_SCRIPT_2.md).

## Guardado (notas)

- Usa **nombres de escena** estables, no índices de instrucción.
- Renombrar escenas requiere migración de saves (documentar en el proyecto).
- Variables con `set` pueden persistirse por nombre vía host state.
