# Velvet Anim — **tools**, not premade cutscenes

Crate: **`velvet-anim`**. Building blocks so *you* invent flips, pack reveals, shop spins, etc.

Premade pack-open sequences are **not** the API. Optional samples: `velvet_anim::recipes`.

## Tool stack

| Tool | Role |
|------|------|
| `AnimPose` / `FloatTween` / `AnimDirector` | 2D targets by id |
| `Pose3D` + `Pose3DChannel` | yaw, pitch, roll, foil, depth, … **you set** |
| `project_image` / `ImageBillboard` | perspective quad from **your** image size |
| `ChannelTrack` + `Timeline` | keyframes **you** author |
| Story: `anim.billboard`, `anim.pose3d`, `anim.track` | drive tools from `.vstory` |
| `recipes::*` | optional examples — copy/adapt, not required |

## Compose a card flip yourself (Rust)

```rust
use velvet_anim::{
    ChannelTrack, Timeline, Pose3D, Pose3DChannel, project_image, Fx3dCamera,
};
use velvet_math::{Ease, Vec2};

let mut tl = Timeline::new().with_channel(
    ChannelTrack::new(Pose3DChannel::Yaw)
        .key(0.0, 0.0, Ease::Linear)
        .key(0.4, std::f32::consts::PI, Ease::CubicInOut),
);
// each frame:
tl.tick(dt);
let pose = tl.sample_pose(Pose3D::flat(Vec2::new(200.0, 300.0)));
let quad = project_image(&pose, 70.0, 100.0, &Fx3dCamera::default());
// draw YOUR texture on quad.tl/tr/br/bl; use pose.foil for holofoil UVs
```

## Story language (generic tools)

```text
call anim.billboard:
    target: card0
    x: 200
    y: 300
    half_w: 70
    half_h: 100
    front: "art/strike"
    back: "art/card_back"

call anim.track:
    target: card0
    channel: yaw
    from: 0
    to: 3.14159
    duration: 0.45
    ease: cubic_in_out

call anim.pose3d:
    target: card0
    foil: 0.6
    pitch: -0.1
```

Host: `AnimStoryHost` — `tick(dt)` then `project_all()` for quads.

## Optional recipes

```rust
use velvet_anim::recipes::{recipe_card_flip, recipe_card_emerge};
// These return Timeline tools you can edit or ignore.
```

## What this is *not*

- Not a fixed “pack open game mode”
- Not glTF / full 3D meshes (billboard projection tools only)
- Not “one command does the whole TCG shop” — you compose channels
