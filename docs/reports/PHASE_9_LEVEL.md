# FASE 9 — Editor 2D (modelo de nivel)

## Objetivo

Construir un nivel RPG/acción desde datos de Studio.

## Implementación

`velvet_document::{LevelDocument, LevelEditor}`:

- Capas de tiles (paint, fill, collision flag)
- Entidades (player, npc, enemy, door, chest…) con props
- Cámaras con follow
- Rects de colisión
- Scaffolds `top_down_scaffold` / `action_scaffold` (5 enemigos)
- Undo/redo en `LevelEditor`
- JSON serialize/deserialize
- `validate()` integridad básica

## Integración plantillas

`velvet template install` para RPG/Action escribe:

- `scenes/town.level.json` o `scenes/warehouse.level.json`

## Criterio de salida

- [x] Construir nivel town con player+npc+door  
- [x] Action scaffold 5 enemigos  
- [x] Round-trip JSON  
- [x] Tests  

## Limitaciones

- No hay vista GPU de pintura en Studio aún; el **documento de nivel** es el path real editable/exportable.
