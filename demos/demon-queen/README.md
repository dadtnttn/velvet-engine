# Aquella Vez que un Infarto me Reencarnó como la Reina Demonio más Temida del Continente Oscuro

Aventura narrativa 18+ para Velvet Engine. El prólogo funciona como novela
visual; al comenzar el Acto 1 cambia a exploración isométrica del castillo sin
abandonar los diálogos dirigidos por datos.

El juego compone interfaz y CG en un lienzo lógico nativo de `1920×1080`. La
ventana puede redimensionarse: Velvet Render presenta el lienzo mediante
letterbox bilineal, conserva `16:9` y transforma el mouse a coordenadas lógicas.
A `1920×1080` usa copia directa.

La historia autoritativa es el JSON versionado
[`story/prologue.json`](story/prologue.json). El host Rust recibe de allí el
cuarto, fondo, hablante, texto, ficha de imagen, decisiones y estado; no contiene
una segunda copia de la historia.

Desarrollado por **Velvet Weaver**. *Woven with Velvet Engine.*

> Hi F95zone staff! I'm **Velvet Weaver**, the developer of this game
> ([thread 312490](https://f95zone.to/threads/312490/)). This is its source code, built on my own
> engine. I added this note on 2026-10-15 as proof for my Game Developer badge ticket.

## Código público y recursos locales

Git incluye código Rust, editor, herramientas de Peli/ComfyUI, guiones,
locales, mundos y datos de juego. Los recursos públicos revisados son el
icono, marcos de interfaz, objetos de tienda, mapas, escenas pixel art de carga
y el pack de esqueletos
KayKit CC0 con su licencia. La lista exacta vive en
[`../../tools/repository-policy.json`](../../tools/repository-policy.json).

CG, ilustraciones de personajes, referencias de producción, vídeos, música,
modelos restantes, capturas y
salidas de generación permanecen locales e ignorados. No se eliminan del
disco. Un clon del código permite compilar, pero la experiencia completa
requiere restaurar el pack privado en sus rutas originales bajo `data/`.

```powershell
# Compilar código sin el pack privado
cargo check -p demon-queen --all-targets

# Validar guiones sin imágenes ni ComfyUI (desde esta carpeta)
node tools/peli/peli.mjs validar --sin-recursos data/pelis/prologo/p01_ultima_ronda.peli.md
node --test tools/peli/tests/*.test.mjs
```

La opción `portable` incrusta arte privado y requiere el pack completo.
Sus pruebas visuales y las pruebas del dungeon que cargan modelos locales
se ejecutan aparte de CI del motor. Para QA de imágenes puede configurarse
`PELI_PYTHON` con un intérprete que tenga las dependencias de `revisar.py`.

Los prototipos `demon-queen-*-threejs` son herramientas locales de autoría
y exportación; sus carpetas no se publican con el workspace.

El flujo reproducible para crear bases desnudas, fichas de 4 vistas y
referencias vestidas está documentado en
[`docs/comfyui/WORKFLOW_REFERENCIAS_PERSONAJE.md`](docs/comfyui/WORKFLOW_REFERENCIAS_PERSONAJE.md).

## Prólogo conectado

El recorrido actual adapta `PROLOGUE_DRAFT.md`: muerte de Gorou, Departamento de
Almas, despertar en el dormitorio real, examen de Nyxara, espejo y recuerdo de la
copa manipulada por Pip. Sus ramas cambian variables y diálogos, pero vuelven al
mismo hilo para mantener viable la producción. `prologue_end` abre
`worlds/act1_castle.json`, restaura la última posición y conserva la partida.
El último diálogo se muestra dentro del VN. Al avanzar, un fundido entrega el
control al mapa WGPU 3D dentro del mismo proceso, ventana y dispositivo gráfico.
El mapa carga en segundo plano; se conservan tamaño, posición, foco y título de
ventana. No se lanza un segundo EXE ni se abre CMD. `Esc` vuelve al menú del mismo
juego y `Continuar` recupera la fase 3D y su posición.

Cada diálogo incluye en el JSON la descripción, encuadre, personajes y estado de
la imagen correspondiente;
[`PROLOGUE_STORYBOARD.md`](PROLOGUE_STORYBOARD.md) presenta esas fichas junto al
guion. Las CG ya disponibles se cargan por su clave y las pendientes conservan el
fondo de su cuarto hasta que se agregue el recurso final.

## Acto 1 explorable

El modo normal usa el mapa 3D de `worlds/queen-room.world.json`, modelos GLB,
colisiones, puertas automáticas, cámara isométrica y minimapa. `WASD`/flechas
mueven a la Reina, `Shift` corre, `Q/E` ajusta zoom y `M` amplía el mapa.

El explorador alternativo `--2d-explore` usa `velvet-play` para movimiento y colisiones, mientras el host
aplica una proyección isométrica con cámara suave. Mapa, tiles, regiones,
spawns, actores y conversaciones viven en `worlds/`; los personajes usan SVG
temporales que después pueden sustituirse por sprites sin cambiar la lógica.
Durante la exploración se usan plantillas de conversación con retrato. Las CG
quedan reservadas para escenas que realmente necesitan una composición única.

La arquitectura de NPCs del castillo (sirvientas, patrulla sin cruzar puertas,
colisiones, diálogos y burbujas legibles) está documentada en
[`NPC_SYSTEM.md`](NPC_SYSTEM.md). Léela antes de cambiar `src/exploration3d.rs`
o `src/bin/map_overlay.rs`.

La Reina ya usa el modelo chibi v2 riggeado (`data/assets/3d/characters/`):
Blender hornea `Queen_Walk` en ocho PNG isométricos transparentes de 128×192,
12 fps, definidos en `worlds/act1_castle.json`. El renderer mantiene el
auto-sombreado del modelo y dibuja una sombra de contacto suave sobre el tile.
Para regenerarlos tras cambiar el `.blend`:

```powershell
& "C:\Program Files\Blender Foundation\Blender 5.2\blender.exe" --background `
  --python tools\render_queen_game_sprites.py -- `
  --source-blend data\assets\3d\characters\queen_chibi_v2_final.blend `
  --output-dir data\assets\exploration\characters\queen_walk
```

## Ejecutar

```powershell
cargo run -p demon-queen --release
```

### EXE portable de distribución

```powershell
powershell -ExecutionPolicy Bypass -File demos\demon-queen\tools\build_portable.ps1
```

El resultado queda en
`demos/demon-queen/dist/DemonQueen-Prologue-Portable.exe`, acompañado por
`data/assets/3d/` y `worlds/queen-room.world.json`. Es un único ejecutable para
VN y exploración; no necesita `demon-queen-3d.exe`. La variante
principal incrusta el prólogo, los mundos explorables, los once idiomas, el
VCSS, los SVG temporales y los PNG activos de Krea2/RedCraft. Los modelos de la
Reina, dungeon, bosque y pueblo se distribuyen en la carpeta de datos.
Los guardados, galería y preferencia de idioma permanecen en
`%APPDATA%\VelvetEngine\DemonQueen`, para que actualizar o mover el EXE no
destruya el progreso.

Controles:

- Clic sobre el cuadro de diálogo, `Enter` o `Espacio`: mostrar/avanzar texto.
- Ratón o `↑`/`↓` + `Enter`: navegar y decidir.
- Rueda arriba o `Page Up`: retroceder a la escena anterior.
- Rueda abajo o `Page Down`: avanzar cuando el diálogo ofrece continuación directa.
- Mantener `Alt`: ocultar temporalmente toda la interfaz, también en los menús, para ver el fondo.
- `R`: cerrar y relanzar el juego; recarga estilos, historia y arte sin borrar el autoguardado.
- `F5` / `F9`: guardar / recargar.
- `Esc`: volver.

En el castillo:

- `WASD` o flechas: moverse en la proyección isométrica.
- Clic en un suelo caminable: desplazarse allí.
- `E`, `Enter`, `Espacio` o clic en el personaje: hablar/continuar.
- `Esc`: cerrar la conversación; pulsarlo otra vez vuelve al título.

El progreso se guarda automáticamente después de cada diálogo o decisión. El
autosave conserva también la pila de retroceso. `PLAY` abre la selección de
partida; `Continuar` queda visible pero deshabilitado hasta existir un guardado
válido.

El runtime 3D escribe cada segundo y al cerrar
`%APPDATA%\VelvetEngine\DemonQueen\saves\exploration-3d.json`. Al pulsar
`Continuar`, el host VN valida ese sidecar contra el `world_entry`, lo mezcla
con `continue.json` y vuelve al castillo en la última posición y
orientación. Así el movimiento WASD/flechas del 3D y el guardado narrativo
comparten checkpoint sin perder progreso. Para depurar sin abrir WGPU se puede
usar `--2d-explore`.

`OPTIONS` permite cambiar el idioma al instante entre once traducciones. El
inglés es el idioma de una instalación nueva y cada opción se presenta con su
autónimo (`Español`, `Deutsch`, `简体中文`, `繁體中文`, `हिन्दी`, `Русский`,
`日本語`, `한국어`, `Latine`, `Esperanto`). La preferencia elegida queda
guardada y el render usa shaping y fuentes con cobertura específica para
devanagari, CJK y hangul.

La Galería de Recuerdos, accesible desde Opciones y desde el final del prólogo,
registra las ilustraciones vistas, elimina duplicados y conserva los desbloqueos
entre partidas. `Alt` también oculta su interfaz para mostrar la CG nativa.

El menú usa cuatro fondos nativos: `PLAY`, `OPTIONS` y `EXIT` cambian entre la
Reina del jardín, la versión pop y la versión glitch al pasar el cursor; la
pantalla de continuar usa el retrato dorado.

## Interfaz procedural VCSS

El menú principal conserva su arte raster nativo. El cuadro de diálogo, la placa
del hablante y todas las decisiones se construyen con gráficos procedurales del
motor y tokens VCSS: degradado, bisel, doble borde, rieles, gemas, brillo,
tipografía y estados. La guía para crear o retocar pieles está en
[`UI_CHROME_VCSS.md`](UI_CHROME_VCSS.md).

## Importar historias JSON

```powershell
cargo run -p demon-queen --release --features console -- --validate-story
cargo run -p demon-queen --release --features console -- --validate-story C:\historias\capitulo-1.json
cargo run -p demon-queen --release --features console -- --story C:\historias\capitulo-1.json
cargo run -p demon-queen --release --features console -- --image-plan C:\historias\capitulo-1.json
cargo run -p demon-queen --release --features console -- --export-storyboard demos\demon-queen\PROLOGUE_STORYBOARD.md
cargo run -p demon-queen --release --features console -- --capture-explore act1-castle.png
```

La build pública usa el subsistema GUI de Windows y no abre `conhost.exe`.
La feature `console` queda reservada para estos comandos de desarrollo, donde
sí se necesita ver la salida de diagnóstico.

La especificación, ejemplos y reglas están en
[`story/README.md`](story/README.md); `story/schema.json` añade validación y
autocompletado en editores compatibles.

## Resolución

El framebuffer lógico de ambos modos es fijo a **1920×1080**. Los assets se
pintan 1:1; un único compositor GPU presenta VN y 3D en la ventana redimensionable,
con letterbox 16:9, muestreo 1:1 a tamaño nativo y bilineal al cambiar el tamaño. Los recursos
finales viven en `data/assets/` y las copias fullscreen en
`data/assets/native_1080p/`.

## Verificación

```powershell
cargo test -p demon-queen
cargo clippy -p demon-queen --all-targets -- -D warnings
cargo run -p demon-queen --release --features console -- --headless 0
cargo run -p demon-queen --release --features console -- --headless 1
cargo run -p demon-queen --release --features console -- --validate-story
```

Prueba de integración con ventana real (guardados y galería aislados):

```powershell
cargo run -p demon-queen --bin demon-queen -- --smoke-unified demos/demon-queen/.local-tests/unified/report.json
```

Comprueba inicio VN, último diálogo, fundido, identidad de ventana/proceso,
movimiento 3D, cambio de tamaño y recuperación desde disco con `Continuar`.
Escribe un informe JSON y capturas nativas. La guía técnica está en
[`VN_3D_INTEGRATION.md`](VN_3D_INTEGRATION.md).

El antiguo `story/demon_logic.vel` queda solo como referencia histórica; ya no se
carga en el juego.
