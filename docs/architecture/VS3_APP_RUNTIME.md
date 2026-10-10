# VS3 Declarative App Runtime

`velvet-vs3-app` permite ejecutar aplicaciones 2D completamente descritas en Velvet Script 3. Rust actúa únicamente como host de capacidades: compila la VM, crea la ventana, recoge input físico, carga recursos autorizados e interpreta comandos gráficos genéricos.

## Ejecución

```powershell
velvet app check path/to/package
velvet app run path/to/package
velvet app run path/to/package --headless --frames 120
velvet app capture path/to/package output.png --frames 2
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

`app_shutdown()` es opcional.

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
        "capabilities": ["window", "render2d", "input"],
        "limits": {
            "max_commands_per_frame": 4096,
            "max_assets": 512,
            "max_image_pixels": 67108864,
            "max_memory_units": 64000000,
            "max_instructions": 5000000
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

Las rutas absolutas y cualquier ruta con `..` están prohibidas. Los IDs deben ser únicos. `app check` carga y valida todos los recursos, incluidos los declarados como `lazy`.

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
```

Cerrar la ventana desde el sistema sigue siendo una operación del host.

## Seguridad y límites

- La VM conserva sus límites de instrucciones, memoria, stack y recursión.
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
