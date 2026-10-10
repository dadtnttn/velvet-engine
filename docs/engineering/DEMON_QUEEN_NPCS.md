# Demon Queen: NPCs 3D y navegación de sirvientas

Guía para agentes que modifiquen el castillo 3D. Fuente de comportamiento:
`demos/demon-queen/src/exploration3d.rs`; fuente del overlay:
`demos/demon-queen/src/bin/map_overlay.rs`.

## Arquitectura

`App::tick` llama `update_maid_agents` solo dentro del castillo. Cada actor es
un `MaidAgent` y conserva su estado visual/social; cada actor tiene un
`MaidPath` independiente para no compartir destino ni sincronizar movimientos.

La ruta no es una lista de waypoints. `maid_zone` toma los límites calculados
desde los suelos de `worlds/queen-room.world.json`; `maid_navigation_grid`
convierte esos límites en una `GridNav` de `velvet-play`. Las celdas se bloquean
con `room_position_blocked` usando paredes, muebles, jugador, otra maid y
puertas. Para una patrulla, todas las puertas se consideran cerradas.

`maid_reachable_points` hace flood-fill desde la celda actual. El destino se
elige solo de ese componente conectado, por lo que la maid recorre su zona sin
intentar atravesar una puerta. `patrol_phase` alterna sectores del componente
para que la patrulla cubra el espacio; `astar` calcula el camino y
`smooth_path` elimina giros innecesarios. `update_maid_agent` aplica el camino,
desliza por X/Z si hay contacto y borra/replantea la ruta si queda bloqueada.

Las puertas solo se abren por proximidad de la Reina. No hacer que el NPC las
abra automáticamente sin diseñar antes una transición de zona explícita.

## Diálogos

`data/npc_dialogues.json` es banco estático incluido con `include_str!`. Se carga
una sola vez en `NPC_DIALOGUE_BANK` (`OnceLock`). Cada línea contiene:

```json
{"speaker":"Lira","context":"idle","text":"La despensa está lista."}
```

Contextos actuales: `idle`, `queen`, `conversation`, `bump`. La función
`authored_maid_line` filtra por `speaker`/`context`; no colocar `String` dentro
de `MaidAgent`, porque el runtime espera `&'static str`.

Ollama puede generar líneas offline, pero el juego no ejecuta un LLM por frame.
El movimiento debe seguir siendo determinista y verificable con A*.

## Burbujas

`npc_speech_bubbles` proyecta la cabeza del NPC a coordenadas lógicas
`1920×1080` y devuelve `NpcSpeechBubble`. El overlay usa `speech_hash` para
invalidar solo cambios reales. `draw_npc_speech_bubble` mantiene el cuerpo en
tamaño legible, `wrap_label_text` separa por palabras y el recuadro crece en
vertical según el número de líneas.

## Assets y nombres

- `maid_animated.glb` → Lira.
- `maid_walking4.glb` → Nemi.
- Posiciones iniciales: `initial_maid_agents`.
- Escala común: `MAID_WORLD_SCALE`.

Para añadir otra maid, actualizar `MAID_AGENT_COUNT`,
`initial_maid_agents`, `initial_maid_paths`, render/mapeo de modelo y el banco
JSON. Darle una fase de patrulla distinta para evitar sincronía.

## Ajustes y verificación

- `MAID_NAV_CELL`: precisión/coste de la cuadrícula.
- `MAID_COLLISION_RADIUS`: radio físico y separación.
- `MAID_NAV_REPLAN_SECONDS`: reintento cuando no existe path; un path válido se
  conserva hasta llegar a destino.
- `MAID_QUEEN_GREETING_RADIUS`: saludo y reverencia.

Desde la raíz de Velvet Engine:

```powershell
cargo check -p demon-queen --bin demon-queen
cargo test -p demon-queen
cargo build -p demon-queen --release --bin demon-queen
```

Prueba manual: dejar las maids varios minutos, comprobar patrulla dentro de su
componente sin puertas, ausencia de atravesamiento de muebles y recuperación
cuando la Reina bloquea el paso.

