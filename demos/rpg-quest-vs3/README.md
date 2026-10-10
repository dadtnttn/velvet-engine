# RPG Quest VS3

Aplicación de referencia completamente declarada en Velvet Script 3.

## Autoridad

VS3 controla:

- estado, campaña y cinco mundos;
- mapas, props, colisiones, NPC y spawns;
- movimiento, combate, IA, recompensas y jefe;
- inventario, equipo, diálogos y misiones;
- entidades, menú, HUD, hotbar, controles y modales;
- bindings físicos de teclado y ratón;
- manifiesto de ventana, fuentes e imágenes;
- todos los comandos visuales y eventos de aplicación.

El ejecutable Rust del demo solo llama a `velvet-vs3-app::run_package`. El runtime oficial se encarga de VM, sandbox, ventana, input físico, recursos e interpretación genérica de primitivas 2D.

## Estructura

```text
data/
  app.vel       contrato app_manifest/app_init/app_update/app_action
  assets.vel    ventana, capacidades, límites, fuentes e imágenes
  bindings.vel  asignación de input físico
  core/         utilidades compartidas
  worlds/       contenido y colisiones por mundo
  game/         campaña, combate, inventario y diálogo
  render/       primitivas, mundos, entidades y UI
src/main.rs     wrapper mínimo del runtime oficial
tests/          aceptación del contrato declarativo
```

## Validación

```powershell
cargo run -p velvet-cli -- vs3 check demos\rpg-quest-vs3\data
cargo run -p velvet-cli -- app check demos\rpg-quest-vs3\data
cargo run -p velvet-cli -- app run demos\rpg-quest-vs3\data --headless --frames 120
cargo run -p velvet-cli -- app capture demos\rpg-quest-vs3\data output.png --frames 2
cargo test -p rpg-quest-vs3
```

## Ejecución

```powershell
cargo run -p velvet-cli -- app run demos\rpg-quest-vs3\data
```

También puede ejecutarse el wrapper mínimo:

```powershell
cargo run -p rpg-quest-vs3
```

El contrato completo está documentado en `docs/architecture/VS3_APP_RUNTIME.md`.
