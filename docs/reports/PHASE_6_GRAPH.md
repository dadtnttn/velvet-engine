# FASE 6 — Grafo narrativo

## Objetivo

Visualizar y validar historias ramificadas (nodos/aristas).

## Implementación

`velvet_document::NarrativeGraph`:

- Nodos Scene / Ending con posición editable
- Aristas Jump / Call / Choice / Condition
- `from_narrative` / `from_source`
- `validate`: missing targets, unreachable, cycles
- `move_node`, `connect`, `apply_graph_jump`

## Criterio de salida

- [x] Grafo desde historia con 2 ramas  
- [x] Detección de saltos inexistentes  
- [x] Detección de inalcanzables  
- [x] Detección de ciclos  
- [x] Tests unitarios  

## Limitaciones

- GUI de canvas del grafo aún no (datos + validación listos para Studio).
- Sync hacia source vía `apply_graph_jump` / re-emit narrative (no layout en `.vel`).
