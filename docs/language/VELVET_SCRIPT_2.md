# Velvet Script 2 — Typed, Rust-like author language (**ALPHA — superseded as official target**)

> **Official general language is now [VS3](./VELVET_SCRIPT_3.md)** (`// @edition 3`).  
> VS2 is **not** the product name we finish. Useful VS2 code may be absorbed into VS3; do not expand the “VS2 finished” promise.  
> Classic product `.vel` / `StoryProgram` remains supported.

> **Not Python. Not Ren’Py.** Same *authoring capabilities* class (dialogue, menus, layers, screens, i18n), different design: **static types**, explicit modules, no hidden globals, no `$ python` blocks.

| | |
|--|--|
| **Edition** | `// @edition 2` (default for new files once toolchain is stable) |
| **Extensions** | `.vel` |
| **Pipeline** | source → lexer → parser → AST → **HIR** → **types** → bytecode / story IR → VM |
| **Maturity** | **Alpha / partial pipeline** — **not finished**, not production-ready |
| **Honest LOC** | After cleanup (2026), ~**15k** lines of `velvet-script-*` sources (was ~42k with padding). Removed: `story_marker_*`, fake `alias_N`, `format_fixture_*`, `local_completions_N`, `Reserved72..399`, `DiagCode` E0001..E0500 stubs, `path_parse_N` / `lower_scene_N` / `catalog_key_N` / `kw_N` / `diag_eN` clone tests, inflated stdlib/corpus |

## Status (audit-aligned)

**Real and useful today:** lexer/parser/AST, validated declarative `screen` blueprints, HIR story items, `OpVs2` + `vs2_codegen` for dialogue/scenes/jumps/layers/arithmetic, VM + `Vs2Host`, resolve/stdlib (trimmed).

**Incomplete (do not treat as done):**

- Typechecker is partial; many HIR items still need real checks.
- Codegen no-ops or stubs for: `struct` / `enum` / `character` / `state` / `mod` / `use`, and field access (`player.health` ≈ base only). Declarative `screen` items compile to host-neutral blueprints, but do not emit v1 bytecode.
- Coroutines, full LSP semantic model, structured multi-span diagnostics on `Vs2Unit` are partial or thin.
- Generators under `scripts/gen_vs2_*` must **not** be re-run to reintroduce numbered padding (`story_marker_*`, `alias_N`, `format_fixture_N`, etc.).

Velvet Story (2.5) prefers **`StoryProgram`** product IR; OpVs2/host remains a debug fallback. VS2 is **not** a finished backbone — see [VELVET_2_5.md](./VELVET_2_5.md).

## Design goals

1. **Fast** — compile to bytecode; hot path is VM dispatch, not string eval.
2. **Translatable** — dialogue and menu labels use `MsgId` / `t!("key")`; logic never embeds user-facing prose by default in edition 2.
3. **Maintainable** — crates with real HIR/types; diagnostics with file:line:col.
4. **Game layers** — first-class `LayerId`, stacks, UI screens, story presentation.
5. **Rust-like** — `struct`, `enum`, `match`, `Result`/`Option`, `pub`/`use`/`mod`, handles instead of GC objects.

## Dual surface, one type system

| Surface | File role | Examples |
|---------|-----------|----------|
| **logic** | gameplay / systems | `fn`, `struct`, `enum`, modules |
| **story** | narrative sugar | `scene`, `say`, `menu`, `show`/`hide` |

Both lower to the same HIR and typechecker.

## Layers (games)

| Type / API | Meaning |
|------------|---------|
| `LayerId` | Stable id (`"settings"`, `"hud"`, `"dialogue"`) |
| `LayerKind` | Story \| Ui \| World \| Fx \| Audio |
| `push_layer` / `pop_layer` | Stack exclusive UI |
| `show_layer` / `hide_layer` | Overlay visibility |
| `set_layer_z` | Draw order |
| `screen Name { ... }` | Typed declarative UI (not Python Screen Language) |

Studio’s pantallas (`velvet.studio.json`) should use the same `LayerId` vocabulary.

## Declarative screens

Structure, copy, stable actions, classes, shortcuts, and enabled state can live in
VS2 while VCSS owns layout and presentation:

```velvet
// @edition 2
screen title_menu {
    class: "title-menu"
    title: "VELVET ARCANA"
    subtitle: "NIGHTFALL CASINO"
    eyebrow: "THE VELVET TABLE"
    footer: "ARROWS NAVIGATE   ENTER CONFIRMS"

    button start {
        label: "NEW RUN"
        description: "Challenge the first blind."
        action: "start"
        icon: "play"
        class: "primary"
        hotkey: "ENTER"
        badge: "READY"
        enabled: true
    }
}
```

Rust hosts use `velvet_script_layers::parse_screen_source` to obtain validated
`ScreenBlueprint` and `ScreenButtonSpec` values. Screen names, button ids, and
properties must be unique; button `label` and `action` are required. Parser,
formatter, compiler acceptance, LSP symbols/completions, and source diagnostics
all understand the syntax. Host rendering and action routing remain explicit.

## Story capabilities (Ren’Py-class, not clone)

| Capability | VS2 form |
|------------|----------|
| Labels / flow | `scene`, `jump`, `call`, `return` with `SceneId` |
| Dialogue | `say speaker, t!("key")` |
| Choices | `menu { t!("a") => { ... } }` |
| Presentation | `show` / `hide` / `at` / `with` + `Transform` / `Transition` |
| State | `state { name: Type = default }` — explicit, typed |
| Characters | `character` items with portraits/colors |
| Screens | `screen` + typed `Action` |
| i18n | `t!("key")`, extract to `tl/<lang>/` |
| Transforms | `transform` blocks → animation IR / bytecode |
| **No** | Python blocks, `$` lines, monkey-patch store |

## Logic example

```velvet
// @edition 2
mod game;

use story::layer::{LayerId, push_layer};

pub enum Channel {
    Bgm,
    Sfx,
    Voice,
}

pub fn open_settings() -> Result<(), ScriptError> {
    push_layer(LayerId::new("settings"))?;
    Ok(())
}
```

## Story example

```velvet
// @edition 2
character aria {
    name: t!("char.aria.name"),
    color: "#ff4f8b",
}

state {
    trust: i32 = 0,
}

scene intro {
    background "bg/night.png";
    show aria at right;
    say aria, t!("intro.1");
    menu {
        t!("intro.choice.ok") => {
            trust += 1;
            jump conversation;
        }
        t!("intro.choice.no") => {
            jump hallway;
        }
    }
}
```

## Explicit non-goals

- Python interop or Ren’Py `.rpy` execution.
- Full Rust borrow checker in script (handles + typeck only).
- Unlimited generics / macros.

## Tooling

```bash
velvet script check path.vel
velvet script run path.vel
velvet script fmt path.vel
velvet script extract path.vel --out tl/en/messages.json
velvet script dump-hir path.vel
```

## Pipeline crates (alpha)

| Crate | Role |
|-------|------|
| `velvet-script-syntax` / lexer / parser / ast | Front-end surface |
| `velvet-script-hir` | High-level IR (story + logic) |
| `velvet-script-types` | Typechecker |
| `velvet-script-resolve` | Name resolution, import graph, prelude |
| `velvet-script-layers` | Game layers (`LayerId`, stacks, screens) |
| `velvet-script-i18n` | `MsgId` / `t!` extract & catalogs |
| `velvet-script-stdlib` | Typed prelude signatures |
| `velvet-script-bytecode` | `Op` + `OpVs2` catalog |
| `velvet-script-compiler` | AST compile + **`vs2_codegen`** (HIR → OpVs2) |
| `velvet-script-vm` | VM + **`Vs2Host` / `Vs2MiniVm`** story/layer host |
| `velvet-script-format` | Formatter + VS2 brace rules (rejects Python style) |
| `velvet-script-lsp` | LSP + VS2 completions/hover |
| `velvet-script-corpus` | Sample corpus tests |

**Honest maturity:** ALPHA / **partial** — not Ren’Py, not Python; not a finished language. Author capabilities class (dialogue, menus, layers, i18n) with rust-like *intent*; typeck/codegen still incomplete for several HIR items. Core engine beta is separate.

See also: [VELVET_SCRIPT.md](./VELVET_SCRIPT.md) (v1 surface), Studio alpha docs.
