# VS3 Declarative App Runtime

`velvet-vs3-app` permite ejecutar aplicaciones 2D completamente descritas en Velvet Script 3. Rust actúa únicamente como host de capacidades: compila la VM, crea la ventana, recoge input físico, carga recursos autorizados e interpreta comandos gráficos genéricos.

## Ejecución

```powershell
velvet app check path/to/package
velvet app run path/to/package
velvet app run path/to/package --headless --frames 120
velvet app capture path/to/package output.png --frames 2
velvet app run path/to/package --save-dir ./partidas
velvet app profile path/to/package --action nueva --action avanzar --frames 60 --top 30
```

El runtime descubre automáticamente el único namespace que exporta `app_manifest`; no conoce el nombre del juego ni exige un módulo concreto.

## Contrato obligatorio

Un paquete debe exportar exactamente estas funciones:

```vel
export function app_manifest() { ... }
export function app_init() { ... }
export function app_update(dt: float, input: map) { ... }
export function app_action(action: string) { ... }
```

Exportaciones opcionales:

```vel
export function app_shutdown() { ... }
export function app_fonts(metrics: map) { ... }     // antes de app_init
export function app_storage(result: map) { ... }    // devuelve un frame
export function app_task(name: string, arg: any) { ... }  // trabajo largo
```

## Manifiesto

```vel
export function app_manifest() {
    return {
        "app": {
            "title": "Mi aplicación",
            "window_width": 1280,
            "window_height": 720,
            "minimum_width": 640,
            "minimum_height": 360,
            "canvas_width": 960,
            "canvas_height": 540,
            "resizable": true
        },
        "capabilities": ["window", "render2d", "input", "storage"],
        "limits": {
            "max_commands_per_frame": 4096,
            "max_assets": 512,
            "max_image_pixels": 67108864,
            "max_memory_units": 64000000,
            "max_instructions": 5000000,
            "task_slice_instructions": 2000000,
            "max_task_instructions": 4000000000
        },
        "root": "assets",
        "fonts": [
            {"id": "regular", "path": "system:segoeui.ttf"},
            {"id": "pixel", "path": "fonts/pixel.ttf"}
        ],
        "images": [
            {"id": "hero", "path": "sprites/hero.png", "mode": "preload"},
            {"id": "map", "path": "maps/map.png", "mode": "lazy"}
        ]
    }
}
```

`system:archivo.ttf` busca la fuente en las carpetas del sistema (Windows,
macOS y Linux) y si no existe usa una sans-serif común (Arial, Liberation Sans,
DejaVu Sans). Las rutas absolutas y cualquier ruta con `..` están prohibidas. Los IDs deben ser únicos. `app check` carga y valida todos los recursos, incluidos los declarados como `lazy`.

## Frame packet

`app_init`, `app_update` y `app_action` devuelven:

```vel
{
    "time": elapsed,
    "commands": [...],
    "events": [...]
}
```

El host rechaza frames con demasiados comandos, operaciones desconocidas, recursos inexistentes, fuentes no declaradas o stacks de clip/transformación/opacidad desbalanceados.

## Input físico

Rust no asigna acciones de juego. Entrega un paquete físico:

```vel
{
    "held": ["KeyW", "ArrowUp"],
    "pressed": ["Space"],
    "released": ["KeyE"],
    "pointer": vec2(412, 280),
    "mouse_held": false,
    "mouse_pressed": true,
    "mouse_released": false,
    "wheel": 0.0
}
```

El paquete VS3 decide si una tecla significa mover, atacar, abrir un menú o cualquier otra acción.

## Resolución y redibujado

El paquete maqueta en su lienzo lógico (`canvas_width`×`canvas_height` del
manifiesto) y el host lo dibuja a la resolución real de la ventana: en una
pantalla 4K, un lienzo de 1280×720 se dibuja 3 veces más grande, con texto,
formas y SVG rasterizados a ese tamaño (nada se estira). Más allá de 3× el
host estira la imagen el resto del camino. Las coordenadas del puntero
siguen en unidades lógicas.

Entre un frame y el siguiente el host solo redibuja las zonas cuyos comandos
cambiaron (o cuyo botón cambió de estado bajo el cursor), así que una
pantalla quieta con algo animado cuesta poco aunque la ventana sea enorme.

## Primitivas 2D

El intérprete software soporta:

- `clear`
- `rect`, `alpha_rect`, `rounded_rect`, `border`
- `circle`, `ellipse`, `alpha_ellipse`, `line`
- `sprite`, `sprite_frame`, `tile_fill`, `nine_slice`
- `text`, `text_wrap`
- `svg`
- `panel`, `bar`, `button`, `keycap`
- `clip_push`, `clip_pop`
- `translate_push`, `transform_pop`
- `opacity_push`, `opacity_pop`

### Texto que cabe

| Comando | Campos | Efecto |
|---|---|---|
| `text` | `max_width`, `overflow`, `align` | Si el texto supera `max_width`: `"ellipsis"` (por defecto) lo corta con `…`, `"shrink"` reduce el tamaño (mínimo 8 px) y luego corta, `"clip"` lo corta sin marca. `align` acepta `"left"`, `"center"` y `"right"` (x es el borde derecho). |
| `text` | `stroke`, `thickness` | Contorno: cada letra se rodea de `thickness` px (lógicos) del color `stroke`, dibujado detrás del texto. Queda continuo a cualquier resolución. |
| `text_wrap` | `w`, `max_lines` | Ajusta por palabras a `w`; las palabras más largas que la línea se parten por letras. Con `max_lines` la última línea termina en `…`. |
| `button` | `max_width`, `overflow` | La etiqueta nunca se sale del botón: por defecto se encoge y luego se corta. |

```vel
{"op": "text", "x": 20, "y": 40, "text": nombre, "font": "regular", "size": 18,
 "max_width": 160, "overflow": "shrink"}
{"op": "text_wrap", "x": 20, "y": 60, "w": 300, "max_lines": 3, "text": bio,
 "font": "regular", "size": 14, "line_height": 18}
```

### Medir texto desde el script

Si el paquete exporta `app_fonts(metrics)`, el host lo llama una vez antes de
`app_init` con una tabla por fuente:

```vel
{"regular": {"a": 55.62, "b": 55.62, ..., "default": 55.62, "line_height": 115.0}}
```

Son los avances de cada carácter (ASCII y letras latinas con acentos, `ñ`,
`¡`, `¿`, comillas, `…`) a tamaño 100. Con ellos el script puede maquetar:

```vel
state { FONTS: map = {} }
export function app_fonts(metrics: map) { FONTS = metrics }

function fits(text: string, size: int, w: int) {
    return str_measure(text, FONTS.regular, size) <= w
}
function card_height(text: string, w: int) {
    return len(str_wrap(text, FONTS.regular, 14, w)) * 18 + 24
}
```

`str_measure` y `str_wrap` dan el mismo resultado que el renderer (sin
kerning), así que las alturas calculadas coinciden con lo dibujado.

Todos los estilos, assets, fuentes, colores, tamaños, capas y acciones son campos declarativos del comando. El host no contiene IDs como nombres de personajes, mundos, botones o iconos.

### SVG en línea

`svg` dibuja un documento SVG generado por el script, ideal para personajes
personalizables, iconos o paneles con formas que las primitivas no cubren:

```vel
{"op": "svg", "x": 40, "y": 60, "w": 240, "h": 280, "action": "perfil",
 "text": "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 200 240'>...</svg>"}
```

- El documento se escala para llenar `w`×`h` (define un `viewBox` con la misma proporción).
- El host lo rasteriza con `resvg` una sola vez por combinación de texto y tamaño y
  guarda el resultado en una caché con límite de memoria; repetir el mismo texto
  cada frame es barato.
- `<text>` solo usa las fuentes declaradas en el manifiesto; `sans-serif` apunta a la primera.
- Las referencias externas (`<image href="archivo">`) se ignoran: el SVG no puede leer el disco.
- Límites: 256 KiB de texto y 4096 px por lado. Un SVG inválido no se dibuja.
- Acepta `action` como cualquier otra primitiva y `flip_x` para reflejarlo.

## Eventos del runtime

El frame puede emitir:

```vel
{"kind": "app.exit", "payload": {}}
{"kind": "window.set_title", "payload": {"title": "Nuevo título"}}
{"kind": "window.set_size", "payload": {"width": 1280, "height": 720}}
{"kind": "window.set_fullscreen", "payload": {"enabled": true}}
{"kind": "app.profile", "payload": {"enabled": true}}   // false imprime el informe
{"kind": "app.task", "payload": {"name": "years", "arg": 10}}  // ver Tareas largas
```

Cerrar la ventana desde el sistema sigue siendo una operación del host.

## Tareas largas (`app_task`)

`max_instructions` limita cada llamada (`app_update`, `app_action`…). Para un
trabajo que no cabe en una llamada (simular años, generar un mapa grande,
cargar datos, una IA que piensa) el frame emite:

```vel
{"kind": "app.task", "payload": {"name": "years", "arg": 10}}
```

y el host empieza `app_task(name, arg)` como **tarea cooperativa** sobre la misma
sesión (ve y cambia el mismo `state`). Desde el frame siguiente:

- Cada frame la tarea corre hasta `task_slice_instructions` instrucciones
  (2 millones por defecto). Si se acaban a mitad de trabajo, se pausa sola y
  sigue en el próximo frame, sin perder nada. La ventana nunca se congela.
- `yield(frame)` muestra ese frame y espera al siguiente. El valor de `yield`
  es `{"dt": segundos, "input": paquete físico, "action": acción clicada o null}`.
- Mientras la tarea corre, el host no llama a `app_update` ni a `app_action`:
  los clics llegan como `action` en la respuesta de `yield`.
- Al terminar, la tarea devuelve un frame (o `null`) y la app sigue normal.
- `max_task_instructions` (4.000 millones por defecto) corta una tarea
  descontrolada; el error muestra la pila del script.
- Las respuestas de `storage` pedidas durante la tarea se entregan a
  `app_storage` cuando termina.

```vel
export function app_task(name: string, arg: any) {
    match name {
        "years" => {
            let i = 0
            while i < arg {
                simulate_year()
                i += 1
                let ev = yield(progress_frame(i, arg))   // un año por frame
                if ev.action == "cancel" { break }
            }
        }
    }
    return frame()
}
```

`velvet app profile` deja terminar las tareas que inicie cada `--action`.

## Guardado de partida (`storage`)

Con la capacidad `storage` el script guarda y carga datos por *slots* sin ver
rutas ni el sistema de archivos:

```vel
{"kind": "storage.save",   "payload": {"slot": "auto", "data": partida, "tag": "autosave"}}
{"kind": "storage.load",   "payload": {"slot": "auto"}}
{"kind": "storage.delete", "payload": {"slot": "auto"}}
{"kind": "storage.list",   "payload": {}}
```

Después de procesar el frame, el host responde cada petición llamando a
`app_storage(result)`, que devuelve un frame como `app_action`:

```vel
export function app_storage(r: map) {
    match r.op {
        "load" => { if r.ok { restore(r.data) } }
        "list" => { SLOTS = r.slots }   // [{"slot", "bytes", "modified"}]
        "save" => { if !r.ok { toast("No se pudo guardar: " + r.error) } }
    }
    return frame()
}
```

`result` contiene `op`, `slot`, `ok`, `error`, `data` (solo en `load`) y el
`tag` que se haya enviado. Reglas:

- `data` puede ser cualquier valor serializable (null, bool, números, texto,
  listas, mapas, vectores); se guarda como JSON. Máximo 8 MiB por slot y 64
  slots.
- Los nombres de slot son 1-48 caracteres `[A-Za-z0-9_-]`; nunca forman rutas.
- Cada slot se escribe en un archivo temporal y se renombra, así un cierre
  inesperado no deja una partida a medias.
- Carpeta: `--save-dir`, si no `VELVET_SAVE_DIR`, si no
  `%APPDATA%\Velvet\<título>\saves` (Windows),
  `~/Library/Application Support/Velvet/<título>/saves` (macOS) o
  `$XDG_DATA_HOME/velvet/<título>/saves` (Linux).
- `velvet app capture` y `velvet app profile` usan slots en memoria: nunca
  tocan el disco.
- Sin la capacidad `storage` cada petición responde `ok: false`.

## Perfilado

`velvet app profile` ejecuta el paquete sin ventana (`app_init`, las acciones
`--action` en orden y `--frames` actualizaciones) y lista, por función, las
instrucciones propias, el porcentaje del total y las llamadas. Los natives
aparecen como `native:<nombre>` y las lambdas como `fn#<n>`. Si una llamada
supera `max_instructions`, el error muestra la pila del script en ese momento.

## Seguridad y límites

- La VM conserva sus límites de instrucciones (por llamada, por porción de tarea y por tarea completa), memoria, stack y recursión.
- El guardado solo escribe en la carpeta de slots de la app.
- Los recursos quedan confinados al `root` declarado.
- Se limita el número de recursos, píxeles por imagen y comandos por frame.
- Las capacidades deben declararse explícitamente.
- VS3 no recibe acceso arbitrario al sistema de archivos ni a APIs de ventana.

## Arquitectura

```text
Paquete VS3
  ├─ app_manifest
  ├─ app_init / app_update / app_action
  ├─ recursos declarativos
  └─ frame packets
          ↓
velvet-vs3-app
  ├─ VM persistente y sandbox
  ├─ validación de contrato
  ├─ loader genérico de recursos
  ├─ eventos físicos
  ├─ ventana winit/softbuffer
  └─ intérprete de primitivas 2D
```

El demo `demos/rpg-quest-vs3` es una aplicación de referencia. Su ejecutable Rust solo llama a `run_package`; toda su lógica, contenido, UI, bindings, recursos y presentación viven en `.vel`.
