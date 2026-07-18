# Mapa de crates — Velvet Engine

## Grafo lógico

```text
velvet-math, velvet-time, velvet-events
        ↓
   velvet-core
        ↓
   velvet-app  ← plugins de:
        ├── velvet-assets
        ├── velvet-render (wgpu, winit via app window)
        ├── velvet-audio
        ├── velvet-input
        ├── velvet-ui / velvet-text
        ├── velvet-ecs / velvet-scene
        ├── velvet-story
        ├── velvet-play → velvet-rpg, velvet-action
        └── …

velvet-script-lexer → parser → ast
        → compiler → bytecode → vm
        → format, lsp (helpers)
        → story load (lower AST→IR)

velvet-project → validate modules
velvet-build → pack, loc, export
velvet-cli / velvet-editor (velvet-studio) / velvet-runtime
velvet-integration-tests / velvet-bench / velvet-test-utils
```

## Inventario

| Crate | Rol | Madurez |
|-------|-----|---------|
| velvet-math | Vectores, matrices, easing, curves | Alta |
| velvet-time | Time, fixed, timers, pause | Alta |
| velvet-events | Event channels | Alta |
| velvet-core | Config, plugins, services, hash | Alta |
| velvet-app | App shell | Alta |
| velvet-assets | Handles, load, hot reload | Media |
| velvet-render | 2D wgpu | Media |
| velvet-audio | Buses/mixer | Media (backend) |
| velvet-input | Actions | Alta |
| velvet-text | Markup/typewriter | Media |
| velvet-ui | Retained UI | Media |
| velvet-ecs | ECS custom | Media |
| velvet-scene | Scenes/prefabs | Media |
| velvet-script-* | Language pipeline | Media (HIR/types baja) |
| velvet-story | VN runtime | Alta-media |
| velvet-play | 2D play | Alta-media |
| velvet-rpg | RPG systems | Media |
| velvet-action | Action systems | Media |
| velvet-project | Project model | Media |
| velvet-build | Pack/export/loc | Media |
| velvet-cli | CLI | Media |
| velvet-editor | Studio MVP binary | Baja (GUI) |
| velvet-runtime | Host binary | Baja |
| velvet-integration-tests | Cross tests | Alta |
| velvet-bench | Microbenches | Media |
| velvet-test-utils | Helpers | Baja |

## Dependencias externas clave (justificadas)

| Dep | Uso |
|-----|-----|
| wgpu / winit | Render + ventana |
| serde / ron / json | Config y saves |
| clap | CLI |
| thiserror / anyhow | Errores |
| tracing | Logs |
| logos (si aplica) / lexer propio | Script |
| sha2 | Checksums |
| image | Texturas |
| walkdir | Tools |

No hay kira forzada en todas las builds: audio es capa propia con posible null backend.

## Binarios

| Binary | Package |
|--------|---------|
| `velvet` | velvet-cli |
| `velvet-studio` | velvet-editor |
| `velvet-script-lsp` | velvet-script-lsp |
| demos | examples/* |
| `velvet-bench` | velvet-bench |
