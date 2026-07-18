# FASE 2 — Formato común y round-trip

## Objetivo

Sincronizar editor visual y código sin destruir lógica avanzada.

## Estado inicial

- No existía modelo de regiones visual/advanced.
- Studio solo CLI; sin patch de propiedades visuales.

## Cambios realizados

### Crate nuevo `velvet-document`

- Parse de marcadores `// @visual` / `// @advanced` / `// @protected` / `// @end`
- Propiedades visuales `key: value`
- `apply_visual_patch` solo muta regiones Visual
- `render_document` re-emite preservando advanced/protected
- `round_trip_visual` helper

### Integración

- `velvet-editor`: `regions`, `patch-visual`, módulo `document_edit`
- `velvet-cli`: `velvet document regions|patch`
- Tests de integración `document_roundtrip.rs`
- Plantilla VN: `templates/visual-novel/scripts/main_menu.vel`

## Criterio de salida

| Criterio | Estado |
|----------|--------|
| Abrir documento | OK (`parse_document`) |
| Modificar visual (text/position) | OK |
| Guardar | OK (`render_document`) |
| Conservar advanced | OK (tests) |
| Reabrir sin pérdida | OK (tests) |
| Protected no editable visualmente | OK |

## Pruebas

```bash
cargo test -p velvet-document --lib
cargo test -p velvet-integration-tests --test document_roundtrip
cargo test -p velvet-editor --bin velvet-studio document_edit
```

Evidencia: `{SCRATCH}/roundtrip_tests.log`

## Ejemplo

```bash
velvet-studio patch-visual templates/visual-novel/scripts/main_menu.vel \
  button.start text "Comenzar"
# advanced on_pressed { game.new(); ... } se conserva
```

## Decisiones

| Decisión | Elección | Razón |
|----------|----------|-------|
| Formato | Marcadores en comentarios `// @…` | No requiere DOS formatos; legible en git |
| Ids estables | `id=button.start` | Enlace visual↔advanced |
| Mutación | Solo propiedades de región Visual | Advanced body byte-stable en spirit (re-emit) |

## Limitaciones

- No es un CST completo de Velvet Script (no reescribe AST completo).
- Regiones anidadas complejas más allá del patrón menú/botón no están optimizadas.
- GUI drag-drop del diseñador llega en Fase 4; aquí está el **motor de round-trip**.

## Siguiente fase

**Fase 3:** administrador de proyectos y plantillas ricas + flujo crear→ejecutar en ≤5 acciones.
