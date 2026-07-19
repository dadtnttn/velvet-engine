# Velvet Anim — runtime tools (motion driven by `.vcss`)

Crate: **`velvet-anim`**.

**Author language for motion is `.vcss`** — CSS `@keyframes` / `animation` plus
JS-lite `@script` (`play` / `animate`). See [`VELVET_STYLE.md`](./VELVET_STYLE.md).

This crate is the **runtime toolbox**: poses, tweens, timelines, 3D billboard
projection. It is **not** a second style language.

## Prefer (from `.vcss`)

```css
@keyframes deal {
  from { opacity: 0; scale: 0.65; yaw: 0.9; }
  to   { opacity: 1; scale: 1;    yaw: 0; }
}
@script {
  fn dealOne(id) {
    play("deal", { target: id, duration: 0.32, ease: "cubic_out" });
  }
}
```

```rust
let sheet = velvet_style::parse_stylesheet(src)?;
let run = velvet_style::call_style_fn(&sheet, "dealOne", &[velvet_style::JsValue::str("card0")])?;
for plan in &run.timelines {
    let tl = velvet_anim::timeline_from_plan(plan);
    // sample / drive your draw loop
}
```

Or pure CSS class resolve:

```rust
let plan = velvet_style::plan_animation(&sheet, &query)?;
let tl = velvet_anim::timeline_from_plan(&plan);
```

## Legacy

Line-oriented `.vanim` still parses for compatibility; convert with
`velvet_style::vanim_to_vcss` or load via `style.play` / `anim.script` (auto-detect).

## Runtime tools (not a second style language)

| API | Role |
|-----|------|
| `Pose3D` / `project_image` | Billboard projection |
| `Timeline` / `ChannelTrack` | Keyframe runner |
| `AnimDirector` | Multi-target 2D tweens |
| `timeline_from_plan` | Bridge from `.vcss` plans |
