# Velvet Anim — tools for motion & VFX

Crate: **`velvet-anim`**. Pure tooling + story host; renderers sample [`AnimPose`].

## Concepts

| Piece | Role |
|-------|------|
| `AnimPose` | `pos`, `scale`, `rotation`, `opacity` |
| `FloatTween` | One field animated with `velvet_math::Ease` |
| `EffectKind` | Presets: `deal`, `fade_in`, `fade_out`, `move`, `punch`, `shake`, `bounce`, `pop` |
| `AnimDirector` | Many named targets (`card0`, `hero`, …) |
| `.vanim` script | Compact author language |
| `AnimStoryHost` | Story `call anim.*` → director |

## Story (`.vstory`)

Register host: `StoryPlayer::start_with_host(prog, Arc::new(AnimStoryHost::new()))`, tick each frame with `host.tick(dt)`.

```text
scene start
narrator:
    Las cartas salen.

call anim.fx:
    target: card0
    effect: deal
    x: 200
    y: 360
    duration: 0.35

call anim.fx:
    target: card1
    effect: deal
    x: 320
    y: 360
    duration: 0.35
    delay: 0.08

call anim.move:
    target: banner
    x: 0
    y: 40
    duration: 0.4
    ease: back_out

end
```

Commands in `CommandRegistry::builtin()`: `anim.fx`, `anim.move`, `anim.stop`, `anim.script`.

## `.vanim` file / inline

```text
# deal three cards into hand slots
spawn card0 0 0
spawn card1 0 0
spawn card2 0 0
fx card0 deal 120 400 0.35
fx card1 deal 240 400 0.35 delay 0.08
fx card2 deal 360 400 0.35 delay 0.16
wait 0.4
fx card0 punch strength 0.2
```

Ops: `spawn`, `fx`, `move`, `stop`, `wait`.

Parse: `parse_anim_script` · run over time: `AnimScriptRunner`.

## Code (Rust)

```rust
use velvet_anim::{AnimDirector, EffectKind, EffectParams};
use velvet_math::Vec2;

let mut dir = AnimDirector::new();
dir.spawn_at("card0", Vec2::ZERO);
dir.play_effect(
    "card0",
    EffectKind::Deal,
    EffectParams {
        to: Vec2::new(200.0, 360.0),
        duration: 0.35,
        ..Default::default()
    },
);
dir.tick(1.0 / 60.0);
let pose = dir.pose("card0").unwrap(); // draw card at pose.pos, scale, opacity
```

## Notes

- Not a full Timeline editor GUI yet — tools first.
- UI crate has its own small tweens; `velvet-anim` is the **shared product spine** for games/cards/story.
