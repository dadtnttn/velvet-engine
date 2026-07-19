# Velvet Style (`.vcss`) — **one** language for look + motion

Crate: **`velvet-style`**.

Visual style **and** animation share this format. The old separate **`.vanim`**
mini-language is folded in: use `@keyframes` + `animation`, or convert legacy
vanim with `vanim_to_vcss`.

## Style (UI)

```css
.button {
  background: #0a0c16;
  border-color: #b9964b;
  color: #d2af64;
  height: 52;
}
.button:selected {
  background: #501e78;
  border-color: #ffdc96;
  glow: #dc50dc;
  glow-strength: 0.85;
  color: #ffe496;
}
#start { icon: star; }
```

## Motion (was `.vanim`)

```css
@keyframes deal {
  from { opacity: 0; y: -80; scale: 0.65; yaw: 0.9; }
  to   { opacity: 1; y: 0;   scale: 1;    yaw: 0; }
}

.card.deal {
  animation: deal 0.35s cubic_out;
  animation-delay: 0.08s;
}
```

### Animation properties

| Property | Meaning |
|----------|---------|
| `animation` | shorthand: `name duration ease delay` |
| `animation-name` | keyframes name |
| `animation-duration` | seconds |
| `animation-delay` | seconds |
| `animation-timing-function` / `animation-ease` | e.g. `cubic_out` |
| `animation-target` | optional id |

### Animatable channels in keyframes

`opacity`, `x`, `y`, `scale`, `yaw`, `pitch`, `roll`, `foil`, `depth`, …

## Rust

```rust
use velvet_style::{parse_stylesheet, plan_animation, StyleQuery};
use velvet_anim::timeline_from_plan;

let sheet = parse_stylesheet(src)?;
let plan = plan_animation(&sheet, &StyleQuery::class("card").with_class("deal"))?;
let timeline = timeline_from_plan(&plan);
```

## Legacy `.vanim`

```rust
let vcss = velvet_style::vanim_to_vcss("fx card0 deal 200 360 0.3\n")?;
```

Story: `style.load`, `style.use`, `style.play` (also accepts old vanim body).

## Language count

Styles + motion = **one** author language (`.vcss`), not two.
