# Velvet Style (`.vcss`) — CSS + JS for look & motion

Crate: **`velvet-style`**.

**One author language.** `.vcss` mixes:

| Layer | Role | Syntax |
|-------|------|--------|
| **CSS** | Look, cascade, `@keyframes` | selectors, properties, animation |
| **JS-lite** | Orchestration | `@script { let, fn, for, play, animate, on }` |

Runtime playback is **velvet-anim** (`timeline_from_plan`) — not a second style language.

Legacy **`.vanim`** line scripts convert with `vanim_to_vcss`.

---

## CSS side

```css
:root {
  --gold: #ebc878;
  --btn-height: 52;
}

.button {
  background: #0a0c16;
  border-color: #b9964b;
  color: var(--gold);
  height: var(--btn-height);
  padding-x: 14;
  gap: 12;
  border-radius: 4;
}
.button:selected {
  background: #501e78;
  border-color: #ffdc96;
  glow: #dc50dc;
  glow-strength: 0.85;
  color: #ffe496;
}
#start { icon: star; }

@keyframes deal {
  from { opacity: 0; y: -80; scale: 0.65; yaw: 0.9; }
  to   { opacity: 1; y: 0;   scale: 1;    yaw: 0; }
}

.card.deal {
  animation: deal 0.35s cubic_out;
  animation-delay: 0.08s;
}
```

### Custom properties

- On `:root` (or any matched rule): `--name: value;`
- Use: `var(--name)` — substituted when resolving cascade
- Missing vars remain as unresolved `StyleValue::Var`

### Property groups (game UI)

| Group | Properties |
|-------|------------|
| Color / chrome | `background`, `color`, `border-color`, `border-width`, `border-radius`, `glow`, `glow-strength`, `opacity` |
| Box | `width`, `height`, `margin` (+ sides / `*-x`), `padding` (+ sides / `padding-x`), `gap` |
| Type | `font-size`, `font-family`, `font-weight`, `letter-spacing`, `text-align` |
| Motion static | `x`, `y`, `scale`, `rotate` / `yaw` |
| Animation | `animation`, `animation-*`, `transition*` |
| Game | `icon`, `icon-size`, `gold`, `neon`, `foil` |

List for tooling: `KNOWN_PROPERTIES` in `velvet-style`. Hosts may ignore props they do not paint yet.

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

---

## JS-lite side (`@script`)

```css
@script {
  let stagger = 0.08;

  fn dealHand(count) {
    for (let i = 0; i < count; i = i + 1) {
      play("deal", {
        target: "card" + i,
        delay: i * stagger,
        duration: 0.32,
        ease: "cubic_out"
      });
    }
  }

  fn punchCard(id) {
    play("punch", { target: id, duration: 0.22 });
  }

  // tween without named keyframes
  fn logoIn() {
    animate("#logo", { opacity: [0, 1], y: [-24, 0] }, 0.45, "cubic_out");
  }

  on("menu.open", fn () {
    logoIn();
  });

  on("hand.deal", dealHand);
}
```

### Built-ins

| Call | Effect |
|------|--------|
| `play(name, { target, delay, duration, ease })` | Run `@keyframes name` → `StyleAction::Play` + timeline |
| `animate(target, { prop: [from,to], … }, dur, ease?)` | Imperative channels |
| `wait(secs)` | Sequencing hint for hosts |
| `emit(event, payload?)` | Host-visible signal |
| `len` / `min` / `max` / `abs` / `floor` / `ceil` | helpers |

Supports: `let`, `fn`/`function`, `for`, `if`/`else`, `return`, numbers (optional `s` unit: `0.35s`), strings, arrays, objects, `+ - * / %`, comparisons, `&&` `||` `!`, string concat with `+`.

---

## Rust API

```rust
use velvet_style::{
    parse_stylesheet, plan_animation, call_style_fn, emit_style_event,
    StyleQuery, JsValue,
};
use velvet_anim::timeline_from_plan;

let sheet = parse_stylesheet(src)?;

// CSS path
let plan = plan_animation(&sheet, &StyleQuery::class("card").with_class("deal"))?;
let timeline = timeline_from_plan(&plan);

// JS path
let run = call_style_fn(&sheet, "dealHand", &[JsValue::num(5)])?;
for tl in &run.timelines {
    let _ = timeline_from_plan(tl);
}

// Event handlers from @script on(...)
let _ = emit_style_event(&sheet, "menu.open", &[])?;
```

Story commands: `style.load`, `style.use`, `style.resolve`, `style.play`,
`style.call`, `style.emit`, `style.set`, `style.dump`.

CLI:

```bash
velvet style check demos/velvet-stakes/data/styles/casino.vcss
velvet style dump demos/velvet-stakes/data/styles/casino.vcss --class button --state selected
```

### Transitions

```css
.button { transition: opacity 0.2s cubic_out; }
.button:selected { opacity: 1; }
```

Rust: `plan_transition(&from, &to, Some("id"))` → `TimelinePlan`.

### StyleRuntime

```rust
let mut rt = StyleRuntime::new();
call_style_fn_rt(&sheet, "go", &[], &mut rt)?;
// set/query in @script touch rt
```

### @import

```css
@import "base.vcss";
```

Use `parse_stylesheet_with_imports(src, base_dir)`.

---

## Language count

| Author language | Extension | Role |
|-----------------|-----------|------|
| Velvet Story | `.vstory` | narrative |
| Velvet Script | `.vel` | general scripting |
| **Velvet Style** | **`.vcss`** | **UI + motion (CSS+JS)** |

Styles + motion + orchestration = **one** style language, not CSS + vanim + extra.
