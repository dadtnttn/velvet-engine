# FASE 1 — Estabilización del núcleo

## Objetivo

Workspace limpio, pruebas verdes, demos sin regresión; APIs consistentes; clippy/fmt gateables.

## Estado inicial

- Build/test/fmt verdes; clippy `-D warnings` rojo (~100 errores estilo/pedantic).

## Cambios realizados

- Limpieza clippy workspace-wide (derive Default, struct update syntax, `contains`, assign-ops, collapsible if, lifetimes, dead_code allows justificados).
- ECS: uso real de métodos storage; `is_empty` en EventQueue; elisión de lifetimes en queries.
- Text/typewriter: fix never-loop; TextEffect Default derived.
- Core/project/cli/runtime: struct-init configs.
- Math/bytecode/parser: fixes de auditoría previa.

## Criterio de salida

| Check | Resultado |
|-------|-----------|
| `cargo build --workspace --all-features` | OK |
| `cargo test --workspace --lib` | OK |
| `cargo test --workspace --all-features` | OK (Phase 0 evidence) |
| `cargo fmt --all --check` | OK |
| `cargo clippy … -D warnings` | OK |
| demos hybrid/hello/action/rpg | OK (Phase 0 demos.log) |

## Evidencia scratch

`C:\Users\jampi\AppData\Local\Temp\grok-goal-7a84fe77f221\implementer\`

- `clippy_exit.txt` → CLIPPY_EXIT=0  
- `gate_summary.txt`  
- `phase0_audit.md`  

## Siguiente fase

**Fase 2:** formato común + round-trip no destructivo (`@visual` / `@advanced` / CST preservable) con tests automatizados.
