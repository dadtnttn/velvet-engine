# FASE 4 — Diseñador UI (modelo de datos)

## Objetivo

Crear/modificar un menú principal **sin editar código a mano**, sin destruir advanced.

## Implementación

`velvet_document::UiDesigner`:

- `open` / `list_widgets` / `set_text` / `set_position` / `set_image` / `set_action_property`
- undo / redo stacks
- `VisualAction` encoding for simplified action chains
- All mutations go through `apply_visual_patch` → same file format as advanced mode

Studio helper: `document_edit::design_set_button`

## Criterio de salida

- [x] Test `create_modify_menu_without_destroying_advanced`  
- [x] Round-trip still green  
- [ ] GPU drag canvas (Fase 4 GUI polish — next)

## Honestidad

This is the **real mutation path** for simplified mode. A pixel canvas/egui dock is not required for the data-plane acceptance of “modify menu without hand-editing”; it is the next presentation layer on top of `UiDesigner`.
