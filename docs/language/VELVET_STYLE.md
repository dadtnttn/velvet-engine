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
- Fallback: `var(--name, fallback)` — nested fallbacks are supported
- Missing vars remain as unresolved `StyleValue::Var`
- `:root` exports custom properties only; ordinary declarations there do not leak into every element

### Inheritance (`resolve_inherited`)

With ancestor context (root-to-parent scopes on `StyleQuery`), inheritable
properties fall back to the nearest ancestor's computed value: `color`,
`font-family`, `font-size`, `font-weight`, `letter-spacing`, `text-align`,
and every `--*` custom property (nearest real ancestor beats `:root` for
`var()` lookup, so subtree themes work). Anything else (`opacity`, `margin`,
…) never flows implicitly. Whole-value keywords: `inherit` forces the parent
value for any property (absent without a parent), `initial` removes the
declaration (host default), `unset` inherits when inheritable else removes.
Queries without ancestors resolve exactly like `resolve`. The `velvet-ui`
bridge (`vcss_style*`) resolves inherited, so menu color/fonts reach child
widgets; `velvet style explain` shows the winning ancestor as an
`<inherited from …>` step.

### Selectors and context

Simple compounds (`button.primary:hover`, `#start.button`), descendant
selectors (`.menu .panel .button`) and the child combinator
(`.menu > .button`, direct parent only) are supported. Context selectors
require explicit ancestry on the query, ordered from root to direct parent:

```rust
let query = StyleQuery::class("button")
    .with_element("button")
    .with_ancestor_class("menu")
    .with_ancestor(StyleScope::class("panel"));
```

Other combinators and attribute selectors do not partially
match. Specificity follows CSS: `:not()` adds its argument, `:is()`
takes the max alternative, combinators add nothing.

### `@media` (game conditions)

```css
.banner { opacity: 1; }
@media (prefers-reduced-motion) {
  .banner { opacity: 0.5; }
}
@media (min-width: 600px) and (orientation: landscape) {
  .banner { opacity: 0.25; }
}
```

Conditions: `prefers-reduced-motion` (`reduce`/`no-preference`/bare),
`prefers-contrast` (`more`/`less`/`no-preference`/bare = more),
`orientation: portrait|landscape`, `min-width`/`max-width` in px,
`and` groups, `not` prefix and comma OR. Unknown features never match.
`MediaQuery::parse` also accepts a bare feature as shorthand.
Hosts pass facts via `StyleQuery::{reduced_motion, high_contrast,
orientation, viewport_width}` (or `UiTree::vcss_style_auto`, which derives
orientation/width from layout). Only plain rules nest inside.

### Math and color functions in values

```css
.panel {
  padding: calc(12px + 4px);
  width: clamp(120px, 40%, 320px);
  gap: max(4px, 2px * 3);
  background: color-mix(in srgb, var(--gold) 70%, #501e78);
  border-color: alpha(#ffffff, 0.5);
  color: rgb(255 0 0 / 0.9);
}
```

### Gradients and shadows

```css
.hero {
  background: linear-gradient(to bottom, #501e78, #0a0c16 70%);
  box-shadow: 0 2px 8px #000000;
}
```

- `linear-gradient(<to side|Ndeg>, stops)` and `radial-gradient([circle at X Y,] stops)`;
  stop positions as `%` or 0..=1 (missing ones spread evenly).
  Rust: `Gradient::sample(t)` + `fallback_color()`; UI bridge fills
  `UiStyle::background_gradient`.
- `box-shadow: [inset] x y [blur] [spread] [color]` (comma list, either
  order) → `parse_box_shadow` → `UiStyle::box_shadows`. Malformed layers
  keep the raw keyword, never half-parse.

- `calc()`/`min()`/`max()`/`clamp()` over `px`, numbers, `s`/`ms`
  (mixed units stay unevaluated for hosts; `%` resolves against a layout
  base via `eval_calc(expr, Some(base))`, else stays as math).
- `color-mix(in srgb, A [pct], B [pct])` (weights default 50/50, normalize),
  `alpha(color, amount)`, and `rgb(... / alpha)` incl. `%` channels.
- `var()` works inside function arguments: `calc(var(--pad) + 4px)`
  (fallback: `var(--pad, 4px)`), `color-mix(in srgb, var(--gold) 70%, …)`,
  `alpha(var(--brand), 0.5)`, gradient stops, and `box-shadow` colors.
  Values stay deferred (`ColorMix`/`Alpha`/var-bearing `Calc`/`Gradient`/
  `Shadow`) until resolve folds them; undefined vars keep the deferred
  node and hosts skip it. Gradient stops need ALL stops concrete or the
  whole gradient is skipped; shadow layers drop individually.
  `box-shadow` only parses structured when a `var(` is present (bare
  multi-length values like `margin: 0 0` keep the keyword path).

### Shorthands (`border` / `background` / `font`)

Parsed inline into longhands (source order kept, so later longhands win
both ways):

- `border: <width> <style> <color>` in any order, 1–3 parts
  (`2px solid red`, `solid thick var(--c)`; `thin/medium/thick` = 1/3/5px)
  → `border-width` / `border-style` / `border-color`.
- `background: <color> [<image>]` (either order) → `background` +
  `background-image`; a single `url()`/`svg()` moves to `background-image`
  (the bridge reads images there). Single colors/gradients/`var()` stay on
  `background`; multi-layers (`a, b`) and gradient-over-color stay single
  keywords (one painted layer).
- `font: [weight] size[/line-height] family…` (`bold 16px/1.5 "Tahoma",
  sans-serif`) → `font-weight` / `font-size` / `line-height` / `font-family`.
  `font-style` is not a VCSS property — shorthands naming it stay single.
- `inherit`/`initial`/`unset` fan out to every longhand of the three.

One-way: `fmt` prints longhands back, never re-collapses. Anything
unparseable keeps the single shorthand key (which then warns as unknown).

### Minify (`style fmt --minify`)

`format_stylesheet_min` squeezes structure to one line (values reuse the
canonical display; `@script` drops comments and glues non-word tokens).
Re-parses to the same AST (`min(min(x)) == min(x)`, and pretty-printing the
min output gives the canonical pretty form).

### Property groups (game UI)

| Group | Properties |
|-------|------------|
| Color / chrome | `background`, `background-image`, `nine-slice`, `nine-slice-width`, `color`, `border-color`, `border-width`, `border-radius`, `glow`, `glow-strength`, `opacity`, `box-shadow` |
| Box | `width`, `height`, `margin` (+ sides / `*-x`), `padding` (+ sides / `padding-x`), `gap` |
| Type | `font-size`, `font-family`, `font-weight`, `letter-spacing`, `text-align` |
| Motion static | `x`, `y`, `scale`, `rotate` / `yaw` |
| Animation | `animation`, `animation-*`, `transition*` |
| Game | `icon`, `icon-size`, `gold`, `neon`, `foil` |

List for tooling: `KNOWN_PROPERTIES` in `velvet-style`. Hosts may ignore props they do not paint yet.

### Raster UI chrome and nine-slice

Use `background-image` with `url("...")` for a raster plate or `svg(name)` for
an inline `@svg`. `nine-slice` gives fixed source-image margins; the optional
`nine-slice-width` gives their rendered thickness. Both accept CSS-style one to
four-value shorthand in `top right bottom left` order, plus `-top`, `-right`,
`-bottom`, and `-left` overrides.

```css
.menu button {
  background-image: url("ui/button_gothic.png");
  nine-slice: 220 330 190 330;
  nine-slice-width: 14 32 14 32;
}
```

Keeping source and destination margins separate lets high-resolution artwork
retain ornate corners while the button is rendered at normal UI dimensions.
Asset paths remain host-resolved.

### Animation properties

| Property | Meaning |
|----------|---------|
| `animation` | shorthand: `name duration ease delay` |
| `animation-name` | keyframes name |
| `animation-duration` | time (`s` or `ms`, normalized to seconds) |
| `animation-delay` | time (`s` or `ms`, normalized to seconds) |
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
| `len` / `min` / `max` / `abs` / `floor` / `ceil` / `round` / `sqrt` / `pow` | helpers |
| `sin` / `cos` / `clamp` / `lerp` | motion math |
| `upper` / `lower` / `trim` / `split` / `join` | string helpers |
| `replace` / `includes` / `startsWith` / `endsWith` / `repeat` | more strings |
| `range(n)` | `[0 … n)` numbers for stagger loops |
| `push(arr, v)` (new array) / `keys(obj)` / `typeof(v)` | collections |
| `slice` / `charAt` / `padStart` / `padEnd` / `indexOf` | strings + arrays |
| `sort` (numeric-aware) / `reverse` | arrays (`map`/`filter` need lambdas — not yet) |
| `entries` / `values` / `has` / `merge` | objects |
| `atan2` / `hypot` / `sign` / `time()` | more math + runtime clock |
| `once("event", fn)` | one-shot handler (`run_event_drain` consumes it) |
| `rand` / `pick` | seeded randomness |
| `set` / `query`(`get`) | runtime `StyleRuntime` bridge |
| `print(...)` | captured to `ScriptRun.logs` (hosts route it) |

Supports: `let`, `fn`/`function`, `for`, `while`, `switch`/`case`/`default`,
`if`/`else`, `return`, numbers (optional `s` unit: `0.35s`), strings,
backtick templates `` `hi ${name}, ${n + 1}!` `` (holes nest `${…}`,
objects and inner templates; `` \` `` / `` \${ `` stay literal),
ternary (`c ? a : b`, right-associative), nullish (`a ?? b` — only
null/undefined trigger, short-circuits), arrays, objects, `+ - * / %`,
comparisons, `&&` `||` `!`, string concat with `+`.

`switch` compares with `==` semantics, first match wins, no fallthrough
(there is no `break`; use `if`/`else` for ranges; at most one `default:`).
`while` shares the 100000-iteration guard with `for` (infinite loops are
runtime errors, never hangs). `case`/`default` at a `switch` label position
are keywords — avoid starting a statement with those identifiers there.

---

## Rust API

```rust
use velvet_style::{
    parse_stylesheet, plan_animation, call_style_fn, emit_style_event,
    StyleQuery, StyleScope, JsValue,
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
velvet style check templates/visual-novel/styles/ui.vcss
velvet style dump templates/visual-novel/styles/ui.vcss --class button --state selected
velvet style run templates/visual-novel/styles/ui.vcss dealHand --arg 5
velvet style eval templates/visual-novel/styles/ui.vcss "clamp(10, 0, 5)"
velvet style fmt templates/visual-novel/styles/ui.vcss
velvet style fmt templates/visual-novel/styles/ui.vcss --minify
velvet style snapshot templates/visual-novel/styles/ui.vcss --class button --state selected --write --expect themes/button-selected.json
velvet style explain templates/visual-novel/styles/ui.vcss .button.primary:hover color --ancestor .menu
```

`check` also prints `warn:` lines: unknown properties (with did-you-mean
hints, e.g. ``backgroud`` → ``background``), `animation-name` without
`@keyframes`, `var(--x)` without fallback or definition (including `var()`
inside `calc()`/`color-mix()`/gradient/shadow arguments), `transition-property`
naming unknown properties, and custom properties defined but never used via
`var()` (typo catchers — `--custom` properties are always accepted).
Base rules and `@media` blocks are scanned; `@script` bodies are not.
Colors accept CSS names, `#rgb`/`#rrggbb`/`#rrggbbaa`, `rgb()`/`rgba()`
(comma or space/`/` forms), `hsl()`/`hsla()`, `oklch(L C H [/ A])`
(`L` bare/`%`, `C` bare/`%` with 100% = 0.4, `H` bare/`deg`/`turn`;
out-of-gamut clips per channel), `color-mix()` and `alpha()`.
`auto-contrast(color[, dark][, light])` picks dark/light by WCAG luminance
(defaults black/white; `var()` arguments fold in resolve).
`currentcolor` as a whole value resolves against the element's own `color`
(e.g. `border-color: currentcolor`); `color: currentcolor` itself and
`currentcolor` nested inside functions stay unresolved.

`explain` traces one property through the cascade: every matched rule in
cascade order (specificity, then source) with the value it declares, the
winner flagged with `*` (including `@media` origin), plus the final
resolved value. One compound per query (`button.primary:hover`); move
ancestor context to repeatable `--ancestor` (root-to-parent) and enable
reduced-motion matching with `--reduced-motion`. Rust:
`explain_cascade(&sheet, &query, "color") -> (Vec<ExplainStep>, Option<StyleValue>)`.

### Transitions

```css
.button { transition: opacity 0.2s cubic_out; }
.button:selected { opacity: 1; }
```

Rust: `plan_transition(&from, &to, Some("id"))` → `TimelinePlan`.

### `velvet-ui` bridge

Retained UI nodes can resolve VCSS directly. `UiNode::name` is the selector
id, `UiNode::classes` supplies classes, widget kind supplies the element name,
and parent nodes provide descendant-selector context. Focus, hover, active, and
disabled states become live pseudo states:

```rust
let style = tree.vcss_style(button_id, &sheet)?;
```

The adapter maps colors, background image references, nine-slice parameters,
border, radius, opacity, typography, size constraints, margin, padding, flex
growth, and `gap` into `UiStyle`. Row, column, and grid layouts consume the
resolved gap. Resolution is non-mutating so transient pseudo states do not
become sticky.

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
