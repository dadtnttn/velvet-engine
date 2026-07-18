# FASE 3 — Proyectos y plantillas (parcial / en progreso)

## Objetivo

Crear proyecto desde plantilla y ejecutarlo (check/run) en pocas acciones.

## Hecho en esta sesión

| Ítem | Estado |
|------|--------|
| `velvet template list` | **OK** — 4 plantillas |
| `velvet template install` / `velvet new` | **OK** |
| Estructura velvet.project + scripts/main.vel | **OK** |
| `velvet script check` sobre proyecto creado | **OK** |
| main_menu.vel con regiones visual/advanced | **OK** (VN) |
| Pantalla GUI Studio “recientes” | **Pendiente** (CLI cubre flujo) |

## Flujo medible (≤5 acciones CLI)

```bash
velvet template list
velvet template install MyGame --template visual-novel --out .
velvet script check MyGame/scripts/main.vel
velvet document regions MyGame/scripts/main_menu.vel   # si se copió plantilla con menú
velvet script run MyGame/scripts/main.vel              # cuando haya main ejecutable
```

Evidencia: `{SCRATCH}/templates_run.log` — created demo_vn, check ok.

## Siguiente

- Copiar `main_menu.vel` en scaffold de CLI `cmd_new` para VN.
- Enriquecer las 4 plantillas (menús, mapas stub, assets placeholders).
- Studio home: create/open/recent (GUI o shell menu).
