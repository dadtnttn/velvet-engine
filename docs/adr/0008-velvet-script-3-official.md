# ADR 0008 — Velvet Script 3 as the official general language

## Status

**Accepted** (direction). Implementation is iterative; classic path stays supported.

## Context

We currently have overlapping surfaces:

| Surface | Role today |
|---------|------------|
| Classic `.vel` → `StoryProgram` / `VnSession` | **Usable** product VN / story path |
| VS2 (`// @edition 2`, HIR/types/OpVs2/VM) | **Alpha**, partial, not production runtime |
| `.vstory` | Writer-facing narrative → product IR |
| `.vcss` | Style / motion (stays separate) |

VS2 aimed at a typed, general language but never became the official usable path. We want:

1. **One official general language** for **game logic** (and later broader creation / web).
2. **Keep classic** story/product path for novels and existing demos.
3. **No prefabricated game modes** as the language surface — **logics and tools**, not recipes as the API.
4. Near term: **game logic first** (state, systems, events, data). Later: general creation (including web-oriented targets), not a Web3 product in v1 of VS3.

## Decision

### Editions

| Edition | Name | Fate |
|---------|------|------|
| **Classic** | Velvet Script / product `.vel` + StoryProgram | **Kept forever** as the stable narrative/product path |
| **VS2** | `// @edition 2` alpha | **Not the official line.** No new features branded “VS2”. Existing code may be **reused/absorbed** into VS3; the **name and promise of “VS2 finished” are discarded** |
| **VS3** | `// @edition 3` | **Official general language** going forward — usable, documented, the place we perfect the typed/general logic path |

### What VS3 is

- A **general-purpose game logic language** for Velvet Engine (and later other targets).
- Authors write **logics** (rules, state machines, systems, queries, events), not “use this prefab RPG mode”.
- Engine crates remain **tools**; demos remain **examples** (`TOOLS.md` policy unchanged).
- Same family as Script (lexer/parser DNA allowed), but **edition 3** is the product name for “the real language”.

### What VS3 is not (near term)

- Not a finished Web3 / blockchain platform.
- Not a replacement that deletes classic novels.
- Not a pile of game templates (`recipes` stay optional samples).
- Not “VS2 but we claim it’s done” without tests and real runtime.

### Near-term scope (game logic)

Priority order for VS3 usable MVP:

1. **Data & state** — clear types for game values, components-as-data, no hidden globals.
2. **Logic blocks** — pure/systems functions, conditions, events.
3. **Host bindings** — call into engine tools (math, cards, combat queries, story hooks) without shipping a whole game genre.
4. **Run path** — one honest runtime (evolve OpVs2/VM or a slim VS3 VM) with diagnostics, not silent no-ops.
5. **Interop** — classic story can call into VS3 logic modules (later); VS3 can emit or drive presentation via hosts.

### Relationship to existing code

- **Reuse** VS2 crates where solid (parser surface, bytecode ideas, VM dispatch).
- **Rebrand** unfinished VS2 surface as prehistory of VS3 in docs.
- **Classic** continues to load via `StoryProgram` / `VnSession` without requiring VS3.

## Consequences

- Docs: `VELVET_SCRIPT_3.md` is the north star; `VELVET_SCRIPT_2.md` marked superseded as *official* target.
- Roadmap: VS3 logic MVP is a near-term language priority.
- Demos: novella stays classic; new logic demos should prefer VS3 once runnable.
- Web / multi-target: design hooks only; no Web3 implementation in the first usable VS3 cut.

## Alternatives considered

1. **Finish VS2 as the name** — rejected: brand is tied to “alpha / incomplete”; clean official line is VS3.
2. **Only classic forever** — rejected: we still want a general logic language beyond VN sugar.
3. **Big-bang rewrite** — rejected: absorb working pieces; keep tree green.

## References

- `docs/language/VELVET_SCRIPT_3.md`
- `docs/architecture/TOOLS.md`
- `docs/language/VELVET_SCRIPT_2.md` (historical / alpha)
- `docs/adr/0004-velvet-script-pipeline.md`
