# Velvet Script 3 — official general language (**target: usable**)

> **VS3 is the official line** for general game **logic**.  
> Classic story/product `.vel` stays. **VS2 is not the official product** — its useful code may feed VS3; the “finish VS2” goal is retired.

| | |
|--|--|
| **Edition** | `// @edition 3` |
| **Extension** | `.vel` (edition marker selects semantics) |
| **Role** | General **game logic** language (later: broader creation targets) |
| **Maturity** | **Usable MVP** — edition gate, pure logics, host natives (`sin`/`hash_sha256`), CLI `velvet vs3` |
| **Policy** | Tools & logics first — **no prefabricated games as the language API** |

## Why VS3 (not “finish VS2”)

- VS2 aimed high (typed, Rust-like, VM) but stayed **alpha** and was never the runtime of shipping demos.
- We need a **clear official name** and a **usable** bar: real run path, real diagnostics, real game logic — not silent no-ops.
- **Classic** remains the stable path for novels (`StoryProgram` / `VnSession`).
- **VS3** is where we invest for a **general logic language** (game first; web/other hosts later).

## Language map (honest)

| Language | Extension / marker | Official? | Use |
|----------|-------------------|-----------|-----|
| Classic Script / product story | `.vel` (no edition 3) | **Yes — stable narrative/product** | VN, dialogue, choices → `StoryProgram` |
| **VS3** | `.vel` + `// @edition 3` | **Yes — general logic (target)** | Game rules, systems, data, events |
| VS2 | `// @edition 2` | **No — superseded as product name** | Legacy alpha; absorb, don’t expand brand |
| Velvet Story | `.vstory` | Writer narrative | Lowers to product IR |
| Velvet Style | `.vcss` | Look / motion | Separate from logic |

## Design pillars

1. **Logics, not prefabs**  
   The language exposes **compositional logics** (when X, do Y; score; inventory rules; AI checks). It does **not** ship “the only way is this Balatro/RPG/Hotline template.” Demos may show logics; they are not the API.

2. **Game logic first**  
   Near term: state, events, pure functions, host-called tools (math, queries, cards, combat, story hooks).  
   Later: broader creation / multi-target (including web-oriented hosts). **Web3 is out of scope for the first usable cut.**

3. **One honest runtime**  
   Every claimed feature runs or fails with a **structured diagnostic**. No silent `Nop` for “done” features.

4. **Interop with classic**  
   Novels keep classic. Over time: classic scenes may **call** VS3 modules; VS3 may **signal** presentation hosts. Not required on day one.

5. **Same engineering discipline as the engine**  
   Tests on the real path, tools-first crates, no fake LOC.

## Near-term MVP (usable “logic language”)

Minimum for “we can write game logic in VS3 and run it”:

| # | Capability | Notes |
|---|------------|--------|
| 1 | Edition gate | `// @edition 3` recognized; classic unchanged |
| 2 | Values & functions | Numbers, bools, strings, structs-as-data; pure `fn` |
| 3 | Control flow | `if` / `match` / loops with clear bounds |
| 4 | Host surface | Register engine **tools** as natives (no genre kits) |
| 5 | Run | CLI or demo: compile + execute a logic unit with asserts |
| 6 | Diagnostics | file:line:col on errors |

**Explicitly later:** full borrow checker, full Studio IDE, web export, Web3, complete type system parity with Rust.

## Proven surface (matches tests — do not overclaim)

| Area | Syntax / tools |
|------|----------------|
| Edition | `// @edition 3` (required for VS3 API) |
| Functions | `function name(a, b) { … return … }` |
| Locals | `let x = expr` |
| Control | `if` / `else`, `while` |
| Values | ints, bools, floats, strings |
| Operators | `+ - * / %` `== != < <= > >=` `&& \|\| !` |
| Host tools | `abs` `min` `max` `clamp` `sin` `cos` `sqrt` `pow` `lerp` `hash_sha256` `len` `concat` `str` |
| Entry | `velvet vs3 check\|run`, `velvet_script_vs3::eval_call` |

**Not yet claimed:** typed params `x: int`, `->` return types, `struct`/`enum`, full typeck, Web3.

## What authors write (proven form)

```text
// @edition 3
// Logic only — presentation stays story/vcss/hosts

function can_play_card(hand_size, cost, energy) {
    return hand_size > 0 && energy >= cost
}

function apply_damage(hp, dmg) {
    if dmg >= hp {
        return 0
    } else {
        return hp - dmg
    }
}

function sum_to(n) {
    let i = 0
    let s = 0
    while i < n {
        i += 1
        s += i
    }
    return s
}
```

Hosts (Rust / play / story) **call** these logics; they don’t hide a prefabricated combat game inside the language.

Sample file: `samples/vs3-logic/game_rules.vel`.

## Pipeline (target)

```text
.vel (@edition 3)
  → lexer / parser (shared DNA with classic where safe)
  → HIR + types (strict enough to be useful)
  → bytecode / IR
  → VS3 runtime (evolve from VM work; rename as it stabilizes)
  → host bindings (engine tools)
```

Classic product path remains:

```text
.vel / .vstory → StoryProgram → VnSession   (unchanged support)
```

## Relationship to VS2 code

- **Keep** useful crates and tests.
- **Stop** marketing or roadmap items as “complete VS2”.
- **Move** new work under VS3 docs and edition 3.
- Breaking cleanups allowed if classic demos stay green.

## CLI

```bash
velvet vs3 check samples/vs3-logic/game_rules.vel
velvet vs3 run samples/vs3-logic/game_rules.vel --call can_play_card --arg 5 --arg 3 --arg 3
velvet vs3 run samples/vs3-logic/game_rules.vel --call apply_damage --arg i:10 --arg i:3
velvet vs3 run samples/vs3-logic/game_rules.vel --call fingerprint --arg s:hello
```

Library: `velvet_script_vs3::{compile, eval_call, detect_edition}`.

## Success criteria (first usable VS3)

1. A small **logic-only** sample compiles and runs with tests. ✅ `samples/vs3-logic/game_rules.vel`
2. At least **two engine tools** bound as natives (e.g. math + hash). ✅ `sin`, `hash_sha256`, `abs`
3. Docs state clearly: classic for story product, VS3 for general logic. ✅
4. No demo depends on unfinished VS3 features claimed as done. ✅

## Non-goals (first cut)

- Replacing `velvet-novella` classic story overnight.
- Web3 wallets, chains, or token APIs.
- Full Ren’Py/Unity-script parity.
- Genre frameworks as language builtins.

## See also

- ADR: `docs/adr/0008-velvet-script-3-official.md`
- Tools policy: `docs/architecture/TOOLS.md`
- Classic script: `docs/language/VELVET_SCRIPT.md`
- VS2 historical alpha: `docs/language/VELVET_SCRIPT_2.md`
