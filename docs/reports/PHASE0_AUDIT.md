# FASE 0 — Auditoría completa de Velvet Engine

**Fecha:** 2026-07-17  
**Workspace:** `C:\Hijosdelsol\VelvetEngine`  
**Versión reportada:** 0.1.0  

## Comandos ejecutados

| Comando | Resultado |
|---------|-----------|
| `cargo build --workspace` | **OK** |
| `cargo test --workspace --lib` | **OK** |
| `tokei crates examples` | Rust **Code ≈ 52 073** (275 files) |
| `cargo run -p hybrid-demo` | **OK** (ending peaceful) |
| `cargo run -p hello-velvet` | **OK** (30 updates) |
| `cargo run -p action-arena` | **OK** |
| `cargo run -p top-down-rpg` | **OK** |
| `cargo run -p visual-novel` | **Funcional** (interactivo CLI; requiere input) |
| `cargo run -p velvet-cli -- doctor` | **OK** (all checks passed) |
| `cargo run -p velvet-editor -- --help` | **OK** (binario `velvet-studio`) |
| `cargo fmt --all --check` | No ejecutado en esta pasada (deuda Fase 1) |
| `cargo clippy … -D warnings` | No ejecutado limpio aún (deuda Fase 1; hay warnings dead_code) |

## Resumen ejecutivo

El repositorio es un **motor 2D/narrativo real multi-crate**, compilable y con cientos de tests unitarios/integración. Las demos de gameplay y story **funcionan** en headless/simulación.  

**No** es aún un producto de producción para creadores:

- Velvet Studio = **CLI/shell de herramientas**, no editor visual.
- LSP de producto = **analizador JSON de línea**, no tower-lsp/stdio LSP real.
- HIR/types de script = **scaffolds vacíos**.
- Plantillas = **scripts + velvet.project**, sin assets binarios ni menú jugable completo empaquetado.
- Flujo plantilla → export → ejecutable fuera del repo = **parcial**.

## Clasificación por sistema

| Sistema | Estado | Evidencia |
|---------|--------|-----------|
| Velvet Core (config, plugins, servicios, hash) | **Funcional pero limitado** | `velvet-core`; registry plugins; services; profiling |
| Velvet App (loop, schedules, headless/window) | **Funcional pero limitado** | `App::run`, HeadlessRunner, WindowRunner feature |
| Events / Time / Math | **Funcional** | crates con tests amplios |
| ECS | **Funcional pero limitado** | custom ECS; no hierarchy-in-ECS completa como Bevy |
| Scene / prefabs | **Funcional pero limitado** | manager, prefab, transition graph |
| Render (wgpu) | **Funcional pero limitado** | GPU path, batch, letterbox; luces 2D/post GPU incompletos |
| Audio | **Parcial / simulado en CI** | buses/mixer/music CPU; backend real no forzado en demos |
| Input | **Funcional** | actions, bindings, winit map, replay |
| Assets | **Funcional pero limitado** | handles, hot reload poll, pack; async real limitado |
| Text | **Funcional pero limitado** | markup/typewriter/layout; no glyphon/GPU text full |
| UI retained | **Funcional pero limitado** | tree/layout/widgets; no egui Studio |
| Script lexer/parser/compiler/VM | **Funcional pero limitado** | pipeline usable; HIR/types scaffold |
| Script format | **Funcional** | format_source |
| Script LSP | **Stub / simulado** | analyze API; main = NDJSON custom, no LSP protocol |
| Story | **Funcional** | dialogue, choices, saves, gallery, auto, skip |
| Play | **Funcional** | tilemap, physics, pathfinding, triggers |
| RPG | **Funcional** | inventory, quests, leveling |
| Action | **Funcional** | combat, perception, arena |
| CLI | **Funcional** | doctor, run, script, pack, export, check/test/build… |
| Studio | **Parcial (MVP tooling)** | open/new/hierarchy/check; sin UI gráfica |
| Templates | **Parcial** | 4 dirs con main.vel + project; assets vacíos/mínimos |
| Export | **Parcial** | dry-run multiplataforma; archive no implementado |
| Runtime | **Parcial** | binario host mínimo |
| Round-trip visual/código | **No implementado** | no CST preservable para UI |
| Modo simplificado / avanzado Studio | **No implementado** | |

## Conteo de líneas (tokei)

```text
Rust Code (crates+examples): ~52 073
Rust total lines:            ~58 633
```

Desglose propio previo (informe tokei_report): producción ≥35k, tests+tools ≥15k, total ≥50k.

## Búsqueda de incompleto

- `todo!()` / `unimplemented!()`: **no** como APIs terminadas principales (grep limpio de panics de producto).
- Scaffolds explícitos: `velvet-script-hir`, `velvet-script-types`.
- Comentarios “not yet”: export archive, velvet run --path load game, LSP product.
- Studio genera “scene stub” con texto TODO en el **contenido generado**, no en el motor.

## Demos verificadas

| Demo | Resultado |
|------|-----------|
| hello-velvet | OK headless |
| hybrid-demo | OK |
| action-arena | OK |
| top-down-rpg | OK |
| visual-novel | Jugable por CLI interactivo |
| branching-story | Presente (no re-ejecutada esta pasada) |

## Riesgos principales (ver RISKS)

1. Editor visual es el gap #1 respecto a la visión de producto.
2. LSP custom ≠ LSP stdio → riesgo de reescritura.
3. Round-trip sin CST romperá scripts si se edita solo como AST “pretty print”.
4. Audio/render en demos a menudo sin ventana/GPU real en CI.
5. Clippy -D warnings no está verde (deuda Fase 1).

## Conclusión Fase 0

**Base técnica sólida y verificada.** El trabajo de “terminar el motor” debe concentrarse en:

1. estabilizar calidad (Fase 1);
2. formato común preservable (Fase 2);
3. Studio + plantillas + flujo crear→jugar→exportar (Fases 3–4+).

No se reinicia la arquitectura. Se construye encima de los crates existentes.
