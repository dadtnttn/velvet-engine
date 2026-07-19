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

## 3D image FX (pack open, flip, foil)

Velvet renders 2D, but **`Pose3D` + `project_image`** turn any image (card art,
pack PNG) into a **perspective quad** (yaw/pitch/roll). No full 3D mesh engine
required.

| API | Role |
|-----|------|
| `Pose3D` | pos, scale, yaw/pitch/roll, opacity, foil shimmer |
| `project_image` | → `ProjectedQuad` (4 corners + front face flag) |
| `PackOpenFx` | **Generator** for sealed pack → tear → lift → fan cards |
| `anim.pack_open` | Story command to start the generator |

### Story — open a pack

```text
call anim.pack_open:
    x: 480
    y: 270
    cards: 5
    duration: 2.0
```

Host: `AnimStoryHost` stores `PackOpenFx`. Each frame:

```rust
host.tick(dt);
for (id, quad) in host.pack_projected() {
    // draw textured quad with corners tl/tr/br/bl
    // if !quad.front { draw card back }
    // use quad.foil for holographic highlight UV
}
```

### `.vanim`

```text
pack_open 480 270 5 2.0
```

### Single card flip

```rust
use velvet_anim::{project_image, sample_card_flip, Fx3dCamera, Pose3D};
use velvet_math::{Ease, Vec2};

let mut pose = Pose3D::flat(Vec2::new(200.0, 300.0));
pose.yaw = sample_card_flip(t, Ease::CubicInOut);
let quad = project_image(&pose, 70.0, 100.0, &Fx3dCamera::default());
```

## Notes

- Not a full Timeline editor GUI yet — tools first.
- UI crate has its own small tweens; `velvet-anim` is the **shared product spine** for games/cards/story.
- 3D FX are **billboard projections**, not glTF/scene meshes (future work if needed).
