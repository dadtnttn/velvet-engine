# Matriz de características — auditoría 2026-07-17

Leyenda: **C** completo · **FL** funcional limitado · **P** parcial · **S** simulado/stub · **N** no implementado · **R** roto · **NP** no probado en esta auditoría

## Núcleo y plataforma

| Característica | Estado | Notas |
|----------------|--------|-------|
| Workspace multi-crate | C | 40+ members |
| Plugin trait + orden | FL | build/finish; sin hot plugins dinámicos |
| App loop fixed/variable | FL | Headless + Window |
| Resources / systems | FL | |
| Config + validate | FL | |
| Diagnostics / profiler spans | FL | CPU spans |
| Service hub | FL | velvet-core/services |

## Render / audio / input / assets / text / UI

| Característica | Estado | Notas |
|----------------|--------|-------|
| wgpu device + sprites | FL | |
| Cameras / letterbox / virtual res | FL | |
| Batching / atlas / anim | FL | |
| Particles (CPU) | FL | |
| Postprocess stack (params) | P | no full GPU graph |
| Luces 2D | P/N | no producto completo |
| Screenshot | P/N | |
| Audio buses / mixer / music / spatial | FL/S | demos sin device obligatorio |
| Streaming audio real | P | |
| Input actions / gamepad map | FL | |
| Asset load + hot reload poll | FL | |
| Rich text + typewriter | FL | |
| GPU font shaping CJK | P/N | measure placeholder-ish |
| UI tree widgets | FL | no binding Studio visual |

## Script

| Característica | Estado | Notas |
|----------------|--------|-------|
| Lexer / parser / AST | FL | |
| Compiler + bytecode + VM | FL | sandbox limits |
| Stdlib natives | FL | |
| Coroutines / debugger | FL | |
| Formatter | FL | |
| HIR | S | scaffold only |
| Type checker | S | scaffold only |
| LSP stdio (LSP spec) | FL | Content-Length JSON-RPC + protocol tests |
| tower-lsp | N | |
| VS Code extension | N | |

## Story / Play / RPG / Action

| Característica | Estado | Notas |
|----------------|--------|-------|
| Dialogue / choices / jumps | FL | |
| Saves versioned + checksum | FL | |
| Gallery / glossary / auto / skip | FL | |
| Rollback | FL | |
| Tilemap / physics / A* | FL | |
| Inventory / quests / leveling | FL | |
| Combat / perception / arena | FL | |

## Studio / CLI / export / templates

| Característica | Estado | Notas |
|----------------|--------|-------|
| velvet CLI doctor/run/script/pack/export | FL | |
| velvet check/test/build/clean/fmt/assets/inspect | FL | |
| Studio CLI open/new/hierarchy/check | P | no GUI |
| Modo simplificado | N | |
| Modo avanzado GUI | N | |
| UI designer drag-drop | N | |
| Narrative block editor | N | |
| Narrative graph | N | |
| 2D scene editor | N | |
| Round-trip visual↔script | FL | `velvet-document` + tests (Phase 2) |
| 4 templates with real content | P | scripts yes; full game kit no |
| Desktop export dry-run | FL | |
| Desktop export signed package | P/N | |
| Full template→export→run outside | P | |

## Demos

| Demo | Estado |
|------|--------|
| hello-velvet | FL |
| visual-novel | FL (CLI) |
| branching-story | FL |
| top-down-rpg | FL |
| action-arena | FL |
| hybrid-demo | FL |
