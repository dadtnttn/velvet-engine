# Development Roadmap

Phases are sequential. Keep the tree compiling after every phase.

| Phase | Name | Exit criterion |
|-------|------|----------------|
| **0** | Definition & architecture | Workspace builds; docs & ADRs; CI skeleton |
| **1** | App & core | Plugins, schedules, empty window, tested loop |
| **2** | Render, input, audio, assets | Hello Velvet |
| **3** | ECS & scenes | Load/update/unload scenes; prefab tests |
| **4** | Velvet Script basic | Compile + VM + diagnostics |
| **5** | Velvet Story | Visual Novel demo end-to-end |
| **6** | Velvet Play | Controllable player, map, collisions |
| **7** | Velvet RPG | Playable RPG loop demo |
| **8** | Velvet Action | Action Arena loop |
| **9** | Velvet Studio (MVP) | Project/scene/inspector/run |
| **10** | Script tooling | Format, LSP basics, richer language |
| **11** | Saves, localization, export | Desktop export + i18n commands |
| **12** | Polish, hybrid demo, reports | 50k LOC, docs, security/perf reports |

## Near-term priorities (post Phase 12 track)

| Priority | Item | Status | Notes |
|----------|------|--------|-------|
| **Now** | **Card authoring tools** (`velvet-cards`) | **active** | Catalog, deck lists, validation, seeded shuffle, draw/discard **zones** — **tools for authors**, not a finished card *game* (no AI, no match UI, no TCG rules parity). CLI: `velvet cards …`. |
| Demo | **Balatro-like** (`demos/velvet-stakes`) | **pre-alpha** | Poker chips×mult, blinds, hand select — fan demo; jokers/shop later. |
| Later | Hotline Miami–like (top-down action shooter) | **deferred / future** | Pre-alpha spine exists (`velvet-action::hotline`, `examples/hotline-rush`). **Not current priority** — leave on roadmap until card tooling matures; further Hotline (windowed input, levels, masks) is future work. |
| Later | Full CCG/TCG rules engine / match loop | **out of scope near-term** | Optional consumers may use zones later; success bar is tools, not “win a match”. |

## Parallelizable work (within a phase)

- Tests and documentation can trail implementation by a few commits but must land before phase close.
- Benchmarks land when the subsystem exists (not before).

## Definition of Done (per phase)

1. Code compiles (`cargo check --workspace`).
2. Relevant tests pass.
3. Progress report (template in project brief) written under `docs/reports/`.
4. No silent placeholders for claimed features.
5. CHANGELOG updated.
