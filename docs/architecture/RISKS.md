# Risks and Mitigations

| ID | Risk | Impact | Likelihood | Mitigation |
|----|------|--------|------------|------------|
| R1 | Scope too large for single continuous effort | Incomplete modules | High | Strict phases; exit criteria; honest partial status |
| R2 | wgpu backend differences (esp. Windows DX12 vs GL) | Broken Hello Velvet | Medium | Headless render tests; doctor command; CI matrix |
| R3 | Custom ECS reinventing poorly | Performance/bugs | Medium | Keep API small; measure; document swap points |
| R4 | Velvet Script becomes unmaintainable | Tooling collapse | Medium | Lossless syntax tree; separate crates; golden tests |
| R5 | Line-count pressure causes bloat | Unmaintainable code | Medium | Quality gates; forbid empty stubs presented as done |
| R6 | Audio device failures on CI | Flaky tests | Medium | Mock backend in tests; real device only in integration |
| R7 | Save format churn breaks demos | User data loss | Medium | Versioned DTOs + migrations from day one |
| R8 | Editor (egui) diverges from in-game UI | Dual maintenance | Medium | Share layout concepts; Studio can use egui initially |
| R9 | Dependency license conflict | Distribution blocked | Low | cargo-deny; DEPENDENCIES.md review |
| R10 | Hot-reload races | Corruption in dev | Medium | Generation counters; reload barriers |
| R11 | Script infinite loops | Frozen games | Medium | Instruction budgets; cancel; sandbox |
| R12 | Insufficient test hardware diversity | False multiplatform claims | High | Document tested platforms only; CI where available |

## Open technical questions

1. Own physics vs Rapier behind façade (decide by Phase 6).
2. GC vs refcount+arena for script values (prototype in Phase 4).
3. Single-threaded vs multi-threaded ECS schedules (start single-threaded).
4. Asset package format (directory first; packed containers Phase 9+).
