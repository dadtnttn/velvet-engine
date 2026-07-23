# 17

Juego de acción narrativa cenital de 15–30 minutos, creado con VelvetEngine. Toda la simulación de juego está escrita en VS3 (`.vel`); el host de escritorio únicamente traduce dispositivos, presenta píxeles y audio, y persiste datos.

## Ejecutar

Desde la raíz de VelvetEngine:

```powershell
cargo run -p seventeen
```

Verificación automatizada, sin abrir una ventana:

```powershell
cargo run -p seventeen -- --headless
```

Captura determinista para revisar el render:

```powershell
cargo run -p seventeen -- --capture artifacts/seventeen.png
```

## Controles

| Acción | Teclado y ratón | Mando |
|---|---|---|
| Mover | WASD o flechas | stick izquierdo |
| Apuntar | ratón | stick derecho |
| Atacar | clic izquierdo | RT o X |
| Impulso | clic derecho, Shift o Espacio | LB o B |
| Interactuar | E | A |
| Recargar | R | Y |
| Armas | 1, 2, 3 | cruceta izquierda, abajo, derecha |
| Pausa | Esc | Start |
| Pantalla completa | F11 | ajuste del menú |

## Contenido terminado

- Splash, menú principal, nueva partida con confirmación, continuar, ayuda, ajustes, pausa, archivo de memorias, finales y créditos.
- Cinco salas: Despertar, Control, Archivo, Purga y Cámara Cero.
- Pistola de pulsos, escopeta magnética y hoja de fase con desvío de proyectiles.
- Vigilantes, Sabuesos, Ecos y el jefe Cero con dos fases.
- Muerte, resurrección, adaptación por número de muertes, mutación de Purga y repetición de rutas mediante Ecos.
- Tres memorias, dos decisiones finales y variantes dependientes de las muertes.
- Guardado atómico versionado en `%LOCALAPPDATA%\Velvet Grid Studio\17\save.json`.
- Teclado/ratón y mando, pantalla completa, pausa al perder foco y letterboxing 16:9.
- Audio procedural con degradación limpia si no existe dispositivo de salida.
- Opciones separadas de volumen, alto contraste, reducción de sacudidas, distorsión y destellos; el guardado también puede borrarse desde Ajustes con confirmación.
- Patrulla, detección y línea de visión para Vigilantes y Sabuesos; cámara lenta breve al limpiar encuentros.

## Estructura

```text
demos/seventeen/
  data/game.vel   simulación, contenido, IA y narrativa VS3
  src/main.rs     ciclo de aplicación y puente VS3
  src/input.rs    teclado, ratón y mando
  src/model.rs    snapshots tipados recibidos desde VS3
  src/render.rs   raster procedural, HUD y efectos
  src/audio.rs    ambiente y efectos sintetizados
  src/save.rs     configuración y guardado versionado
```

## Contrato VS3-first

[`data/game.vel`](data/game.vel) es la autoridad sobre:

- estado del jugador, armas, munición y puntuación;
- salas, colisiones y progresión;
- IA, daño, proyectiles, jefe y resurrección;
- memoria, diálogo, decisiones y finales;
- datos serializables del guardado;
- eventos abstractos de presentación como `pistol`, `death` o `boss_phase`.

El host Rust no decide resultados de combate ni progresión. Consume `snapshot()` y convierte eventos abstractos en imagen, sonido y vibración visual. Esta separación mantiene VS3 como lenguaje general: no se añadieron palabras clave de escena, sprite, enemigo o arma.

Las mejoras generales al lenguaje/motor realizadas para esta demo son:

- vistas clonadas seguras de `list` y `map` para hosts (`list_items`, `map_entries`, `map_get`);
- análisis correcto de valores dinámicos obtenidos de colecciones en aritmética y llamadas tipadas;
- conservación estática de enteros en `abs`, `min`, `max` y `clamp` cuando todos sus argumentos son enteros.

## Criterio de aceptación automatizado

`--headless` compila el `.vel` real y valida:

1. inicio e introducción;
2. muerte y resurrección;
3. carga de las cinco salas;
4. recuperación de tres memorias;
5. los dos finales;
6. ida y vuelta del formato de guardado.

## Agregar una habitación

1. Añade el nombre a `room_name()` y su definición a `setup_room()` en `data/game.vel`.
2. Define enemigos, obstáculos, peligros, objetos y diálogo usando las funciones generales ya existentes.
3. Añade su condición de cierre a `update_room_state()`.
4. Amplía el límite de transición y guardado, actualmente `5`, y agrega el índice al recorrido de `run_headless()`.
5. Ejecuta `cargo run -p seventeen -- --headless` y `cargo clippy -p seventeen --all-targets -- -D warnings`.

No hace falta modificar el render para una sala nueva salvo que introduzca una nueva clase visual de obstáculo, objeto o actor.

## Reemplazar primitivas por sprites

`FrameView` es el contrato estable de presentación. Sustituye las funciones `paint_enemy`, `world_diamond`, `world_circle` y los bloques de obstáculos en `src/render.rs` por búsquedas de atlas basadas en `kind`. No traslades vida, daño, IA o progresión al render: esos resultados deben seguir llegando desde VS3. Este límite permite cambiar toda la dirección artística sin reescribir el juego.
