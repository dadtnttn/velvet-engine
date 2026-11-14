# Velvet 2.5 — núcleo estable y honesto (**alpha+ / beta-candidate language core**)

## Qué es

Velvet **2.5** no es “VS2 terminado” ni “+30k LOC”. Es el hito **post-limpieza**:

1. **VS2** con no-ops **explícitos** (diagnósticos estructurados + source map).
2. **Velvet Story** (`.vstory`) bajando a **`StoryProgram`** de producto (`velvet-story`).
3. Pruebas e2e con **asserts exactos**, sin relleno.

## Pipeline preferido (escritores) — columna única

```text
.vstory → parser → AST → sema
       → StoryProgram          ← IR canónica
            └→ StoryPlayer / VnSession   ← el runtime (lógica: Velvet Script 3, `logic "…"`)
```

Instrucciones de presentación (`sound`, `pause`, `with`/`transition`, `return`) ya bajan a `StoryOp` reales (no `Nop`).

```bash
velvet story check stories/main.vstory
velvet story build stories/main.vstory   # resumen del StoryProgram
velvet story run stories/main.vstory --choice 0   # product path first
```

## VS2: unsupported ya no es silencio

`struct` / `enum` / `character` / `state` / `screen` / `mod` / `use` / field access
→ `DiagCode::UnsupportedHir` con **file:line:col** en `Vs2Unit.diags`.

## Qué no es 2.5

- Typeck / borrow checker completo estilo Rust.
- Layout completo de structs en la VM.
- Studio GUI total.
- Regenerar generadores de padding.

## Madurez

| Capa | Etiqueta |
|------|----------|
| Lenguaje núcleo (script + story lower) | alpha+ / **beta-candidate** del núcleo |
| Engine / Studio | sin cambio de etiqueta global |
| VS2 “finished” | **No** |
