```text
FASE COMPLETADA
Objetivo: Velvet Play — jugador, mapa, cámara, colisiones, triggers, interacción
Cambios:
  - velvet-play: TileMap (ASCII), physics AABB/circle/raycast, move_and_collide
  - PlayWorld simulation, camera follow, triggers, interactables, A*, FSM AI helpers
  - PlayPlugin steps world with input actions
Pruebas: 18 unit tests (physics, map, nav, world, etc.)
Comandos: cargo test -p velvet-play
Resultados: OK
Estado: implementado (base 2D); animaciones esqueléticas planificadas
Siguiente: RPG/Action (Fases 7–8, completadas en la misma racha)
```
