```text
FASE COMPLETADA
Objetivo: Fase 0 definición/arquitectura + base Fase 1 (App/plugins/tiempo/eventos) + runner de ventana
Cambios:
  - Documentación de visión, arquitectura, dependencias, riesgos, aceptación, roadmap, convenciones
  - ADRs 0001–0006
  - Workspace Cargo multi-crate (~34 crates + hello-velvet)
  - Implementación real: velvet-math, velvet-time, velvet-events, velvet-core, velvet-app, velvet-project, velvet-cli
  - WindowRunner (winit) detrás de feature "window"
  - Inicio Fase 2: velvet-input (acciones/contextos) y velvet-assets (handles/cache/loaders)
  - Scaffolds honestos para render/script/story/play/rpg/action/studio
Crates afectados: todos los del workspace
Archivos creados: docs/**, crates/**, examples/hello-velvet, .github/workflows/ci.yml, configs raíz
Archivos modificados: n/a (repo nuevo)
Pruebas añadidas: unit tests math/time/events/core/app/project/input/assets/test-utils
Comandos ejecutados:
  cargo test --workspace --all-features
  cargo run -p velvet-cli -- doctor
  cargo run -p velvet-cli -- run --headless --frames 5
  cargo run -p hello-velvet
Resultados: workspace compila; tests OK; CLI doctor/run headless OK
Líneas de producción: ver medición tokei (en progreso; base ~pocos miles — lejos de 50k)
Líneas de pruebas: incluidas en crates
Problemas conocidos:
  - Ventana sin GPU (clear color Phase 2)
  - Crates de fases futuras son scaffolds explícitos
  - 50k LOC aún no alcanzado (esperado en fases posteriores)
Deuda técnica:
  - Schedules no paralelos
  - Asset hot-reload sin watcher notify aún
  - Input no cableado a eventos winit en WindowRunner
Siguiente fase: Fase 2 render/audio/assets hot-reload; cablear input a winit; Hello Velvet gráfico
```
