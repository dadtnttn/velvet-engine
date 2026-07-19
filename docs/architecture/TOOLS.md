# Velvet Engine — tools vs demos vs recipes

Policy for authors and crate design:

| Kind | What it is | Examples |
|------|------------|----------|
| **Tool** | Composable API you drive yourself (data + pure functions + hosts you wire) | `velvet-cards` catalog/zones, `velvet-anim` Timeline/Pose3D/`project_image`, aim/hitscan/loadout |
| **Recipe** | Optional sample composition of tools — copy/adapt, **not** the product spine | `velvet_anim::recipes`, `velvet_action::recipes` |
| **Demo** | Playable illustration under `demos/` / `examples/` | `card-duel`, `velvet-stakes`, `hotline-rush` |

## Rules

1. **Crates ship tools first.** No “the only way is one premade game mode.”
2. **Demos may look like games** but must call tools; they do not define the API.
3. **Recipes are optional** and documented as such.
4. Story/CLI expose **generic** commands (`anim.track`, `cards validate`) before branded cutscenes.

## Crate map (tools)

| Crate | Tools |
|-------|--------|
| `velvet-cards` | Catalog, deck list, validation, shuffle, zones |
| `velvet-anim` | Tweens, director, Pose3D, project_image, Timeline tracks, story host |
| `velvet-style` | `.vcss` = CSS + JS-lite (look, `@keyframes`, `@script` play/animate); `style.load` / `style.call` |
| `velvet-action` | Weapons, combat queries, aim, fragility, loadout, pickup; arena/dash as systems |
| `velvet-story` + `velvet-story-lang` | StoryProgram/player, boot, command registry |
| Script crates | Language pipeline (not a finished game) |

## Demos (not tools)

- `demos/card-duel` — menus + simple duel using cards tools  
- `demos/velvet-stakes` — Balatro-like loop using poker eval + cards zones  
- `examples/hotline-rush` — uses action tools + optional room recipe  
