# Velvet Script Language Guide

Velvet Script is the narrative- and gameplay-oriented scripting language for **Velvet Engine**.

**Extensions:** `.vel`, `.velscene`, `.velui`, `.velproject`, `.velprefab`

## Design goals

- Readable by writers for dialogue and choices
- Predictable for programmers (explicit control flow, no hidden globals)
- Compiles to bytecode for the sandboxed VM
- Tooling: format, diagnostics, symbols (Studio / CLI / LSP analysis)

## Pipeline

```text
Source (.vel)
  → lexer (velvet-script-lexer)
  → parser + recovery (velvet-script-parser)
  → AST (velvet-script-ast)
  → HIR / types (velvet-script-hir, velvet-script-types)
  → compiler (velvet-script-compiler)
  → bytecode (velvet-script-bytecode)
  → VM (velvet-script-vm)
```

Story modules can also be **lowered** into `StoryProgram` IR (`velvet-story`) without executing arbitrary bytecode for dialogue lines.

## Top-level items

| Construct | Purpose |
|-----------|---------|
| `character` | Named speaker with display name, color, portrait |
| `state` | Typed story variables with defaults |
| `scene` | Ordered narrative / dialogue block |
| `function` | Callable procedure (gameplay + utility) |
| `component` / `system` | Planned for deeper Play binding (partial) |

## Narrative example

```velvet
character aria {
    name: "Aria"
    color: "#ff4f8b"
    portrait: "characters/aria/neutral.png"
}

state {
    aria_trust: int = 0
    found_key: bool = false
}

scene apartment_night {
    background "backgrounds/apartment_night.png"
    music "music/night_city.ogg" fade_in 1.5

    show aria.neutral at right

    aria "I thought you wouldn't come."

    choice {
        "Apologize" {
            aria_trust += 1
            aria "At least you admit it."
            jump conversation
        }
        "Stay silent" {
            aria_trust -= 1
            aria "As always."
            jump hallway
        }
    }
}
```

## Gameplay example

```velvet
function damage(current, amount) {
    let next = current - amount
    if next < 0 {
        return 0
    }
    return next
}

function main() {
    print(damage(100, 30))
    return 0
}
```

## Rich text (UI / dialogue display)

Markup is handled by **velvet-text**, not the script grammar:

```text
"This is {color=#ff5577}important{/color}."
"Wait{pause=0.5}... not yet."
"{shake intensity=3}Get away!{/shake}"
```

## CLI tools

```bash
velvet script check path/to/file.vel
velvet script run path/to/file.vel
velvet script fmt path/to/file.vel
velvet script lsp path/to/file.vel
```

## VM sandbox limits

| Limit | Default role |
|-------|----------------|
| Instructions / run | Prevent infinite loops |
| Memory units | Cap value graph size |
| Recursion depth | Cap call stack |
| Sandbox flag | Restrict host side effects |

## Status (honest)

| Area | Status |
|------|--------|
| Lexer / parser (core) | Implemented |
| Story lowering | Implemented |
| Bytecode + VM | Implemented (core ops) |
| Formatter | Implemented (basic) |
| LSP analysis | Implemented (diagnostics, symbols, goto, completions) |
| Full type system | Partial |
| Coroutines | Partial / planned |
| tower-lsp JSON-RPC server | Stub / analyze API |

See ADRs under `docs/adr/` for architectural decisions.
