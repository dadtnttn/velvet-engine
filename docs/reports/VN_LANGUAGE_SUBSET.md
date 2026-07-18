# Velvet Script — official VN language subset

This is the **author-facing** subset for visual novels (mental equivalent of daily `.rpy` usage).  
Full language grammar is larger; ship and teach **this** first.

## File layout

```text
my_game/
  velvet.project
  scripts/
    main.vel          # entry_scene
    main_menu.vel     # optional UI with @visual / @advanced regions
  assets/
    bg/ …
    music/ …
```

## Characters & state

```vel
character hero { name: "Hero" color: "#ff4f8b" }
character friend { name: "Mira" color: "#4fc3ff" }

state {
    trust: int = 0
    met_mira: bool = false
}
```

## Scenes

```vel
scene main {
    background "assets/bg/room.png"
    music "assets/music/soft.ogg" fade_in 1.0
    show friend at right
    hero "I said I'd be here."
    friend "You always say that."
    choice {
        "I keep promises." {
            trust += 1
            jump ending_warm
        }
        "Don't make this heavy." {
            trust -= 1
            jump ending_cool
        }
    }
}

scene ending_warm {
    hide friend
    "Ending: Warm Lights"
}

scene ending_cool {
    "Ending: Cool Air"
}
```

## Statements (product path)

| Construct | Effect |
|-----------|--------|
| `background "path"` | Set BG; product session queues dissolve |
| `music "path" fade_in N` | BGM intent with fade-in |
| `show id.at? at place?` | Show sprite; z-order by place (left/center/right) |
| `hide id` | Remove sprite |
| `speaker "text"` / `"narration"` | Dialogue line → Say screen |
| `choice { "label" { … } … }` | Choice screen |
| `jump scene` | Jump |
| `call scene` | Call (return supported in IR) |
| `name = value` / `+=` etc. | Variables |
| `end "id"` | Explicit ending id |

## Product player controls (runtime)

Via `VnSession` / `velvet play`:

- **Advance** — click-to-continue / reveal typewriter  
- **Choose** — menu index  
- **Save / Load** — versioned slots (`.velsave.json`)  
- **Prefs** — text speed, auto, skip, master/music/sfx volumes, fullscreen flag  
- **History** — recent lines  
- **Confirm** — quit / overwrite  
- **Rollback / skip / auto** — reading controls  

## CLI

```bash
velvet script check scripts/main.vel     # errors: file:line:column
velvet recheck-replay . --choice 0       # check then product play
velvet play . --choice 0 --windowed      # product session; windowed best-effort
velvet document patch scripts/main_menu.vel button.start text "Play"
velvet export --binary hello-velvet --out dist --build --release
```

## Out of subset (later)

- Full Python-like scripting blocks  
- ATL animation language  
- `translate` / `tl/` i18n pipeline (S6)  
- Live2D  

## Types / HIR

`velvet-script-hir` and `velvet-script-types` are **scaffolds**. Authoring does **not** require them for the VN product path. Semantic checks today: parse + compile diagnostics with locations.
