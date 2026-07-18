# FASE COMPLETADA: 0 (+ base de Fase 1)

## Objetivo

Documentar visión/arquitectura y establecer un workspace Cargo compilable con núcleo de aplicación (plugins, schedules, tiempo, eventos).

## Cambios

- Documentación de visión, arquitectura, dependencias, riesgos, criterios, roadmap y convenciones.
- ADRs 0001–0006.
- Workspace multi-crate con ~34 crates + ejemplo hello-velvet.
- Implementación real: `velvet-math`, `velvet-time`, `velvet-events`, `velvet-core`, `velvet-app`, `velvet-project`, CLI básica.
- Scaffolds marcados para módulos de fases posteriores.
- CI GitHub Actions, licencias, rustfmt/clippy/deny configs.

## Crates afectados

Todos los del workspace; implementación sustancial en math/time/events/core/app/project/cli.

## Estado por área

| Área | Estado |
|------|--------|
| Docs arquitectura | implementado |
| Workspace | implementado |
| App/plugins/schedules | implementado (headless) |
| Ventana vacía | planificado (Fase 2 / cierre Fase 1 con winit) |
| Render/audio/input | scaffold / planificado |
| Velvet Script | scaffold / planificado |
| Story/Play/RPG/Action | scaffold / planificado |
| Studio | stub binario |

## Pruebas añadidas

- Unit tests en math, time, events, core, app (plugins, ciclo, fixed update), project, test-utils.

## Comandos

```text
cargo test --workspace
cargo run -p velvet-cli -- doctor
cargo run -p velvet-cli -- run --frames 30
cargo run -p hello-velvet
```

## Problemas conocidos

- `velvet run` es headless; ventana real pendiente de winit/wgpu.
- Crates de fases futuras son shells deliberados, no mocks disfrazados de features completas.
- `rust-toolchain.toml` usa `stable` (host actual 1.94.x).

## Deuda técnica

- `App::run_schedule` mueve sistemas fuera del stage (simple, no paralelo).
- Plugin enable-filter por config aún no cubre dependencias transitivas.
- Sin `cargo deny` en CI aún (config presente).

## Siguiente fase

Completar Fase 1 con runner de ventana (winit) y continuar Fase 2 (render, input, audio, assets).
