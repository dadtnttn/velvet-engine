# Tutorial 02 — Visual Novel Basics

Build a short branching scene using Velvet Script and the story runtime concepts.

## Project setup

```bash
velvet new night_bus --template visual-novel --out .
cd night_bus
```

Open `scripts/main.vel` and replace it with:

```vel
character nora {
    name: "Nora"
    color: "#ff4f8b"
}

character driver {
    name: "Driver"
    color: "#aaaaaa"
}

state {
    ticket: bool = false
    patience: int = 2
}

scene bus_stop {
    background "assets/bg/stop.png"
    "The night bus is late. Neon puddles mirror the skyline."
    nora "If this one doesn't come, I'm walking."

    choice {
        "Wave the bus down hard" {
            patience -= 1
            jump boarding
        }
        "Check your bag for the ticket" {
            ticket = true
            nora "There — folded under the grocery list."
            jump boarding
        }
    }
}

scene boarding {
    show driver at left
    driver "Fare."
    if ticket {
        nora "Here."
        jump seat
    }
    nora "I… left it at the stop."
    if patience > 0 {
        driver "One warning."
        patience -= 1
        jump seat
    }
    driver "Next bus."
    jump ending_walk
}

scene seat {
    "The bus lurches forward. City lights smear into lines."
    nora "Maybe late is still on time."
    "Ending: Rolling Home"
}

scene ending_walk {
    "Rain finds the gaps in your jacket."
    "Ending: Long Walk"
}
```

## Check and analyze

```bash
velvet script check scripts/main.vel
velvet script lsp scripts/main.vel
```

`lsp` prints JSON symbols/diagnostics for tooling.

## Localization extract

```bash
velvet localization extract scripts/main.vel --out locale/source.json
velvet localization extract scripts/main.vel --out locale/source.po --format po
```

Translate keys, then:

```bash
velvet localization validate locale/source.json locale/es.json
```

## Story runtime (in code)

Workspace pattern (see `examples/visual-novel`):

1. `velvet_story::load_program_from_source(src, Some("main.vel"), "Night Bus")`
2. `StoryPlayer::start(program)`
3. Loop: present text → `advance()` / `choose(i)` → handle end

Studio hierarchy:

```bash
velvet-studio hierarchy .
velvet-studio check .
```

## Assets

Place files under `assets/` (paths in script are virtual until a loader maps them):

```text
assets/bg/stop.png
assets/characters/nora/neutral.png
assets/music/night.ogg
```

Pack a checksum manifest:

```bash
velvet pack --assets assets --out asset-pack.json
```

## Honest limits

- Rollback, gallery, and voice lip-sync exist as crate APIs but need game-shell wiring.
- Background/music commands are interpreted by the story runtime when the host applies them.
- This tutorial does not produce a shipped installer; use `velvet export --multi` for dry-run manifests only.

Next: [03_topdown.md](./03_topdown.md)
