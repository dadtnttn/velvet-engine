# Velvet Anim — runtime tools (motion driven by `.vcss`)

Crate: **`velvet-anim`**.

**Author language for motion is `.vcss`** (`@keyframes` / `animation` in
[`VELVET_STYLE.md`](./VELVET_STYLE.md)). This crate is the **runtime toolbox**:
poses, tweens, timelines, 3D billboard projection.

## Prefer

```css
@keyframes deal {
  from { opacity: 0; scale: 0.65; yaw: 0.9; }
  to   { opacity: 1; scale: 1;    yaw: 0; }
}
.card.deal { animation: deal 0.35s cubic_out; }
```

```rust
let plan = velvet_style::plan_animation(&sheet, &query)?;
let tl = velvet_anim::timeline_from_plan(&plan);
```

## Legacy

Line-oriented `.vanim` scripts still parse for compatibility; convert with
`velvet_style::vanim_to_vcss` or load via `style.play` / `anim.script` (auto-detect).

## Runtime tools (not a second style language)

| API | Role |
|-----|------|
| `Pose3D` / `project_image` | Billboard projection |
| `Timeline` / `ChannelTrack` | Keyframe runner |
| `AnimDirector` | Multi-target 2D tweens |
| `timeline_from_plan` | Bridge from `.vcss` |
