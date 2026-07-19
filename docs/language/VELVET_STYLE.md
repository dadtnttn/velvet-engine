# Velvet Style (`.vcss`) — CSS-like tools

Crate: **`velvet-style`**. Author stylesheets, resolve classes/states, paint UI.

## Syntax (subset)

```css
/* comments */
.button {
  background: #0a0c16;
  border-color: #b9964b;
  border-width: 1;
  color: #d2af64;
  height: 52;
  padding-x: 14;
  icon-size: 34;
}

.button:selected {
  background: #501e78;
  border-color: #ffdc96;
  glow: #dc50dc;
  color: #ffe496;
  glow-strength: 0.85;
}

#start {
  icon: star;
}
```

### Selectors

| Form | Meaning |
|------|---------|
| `.class` | class |
| `.class:state` | class + pseudo (`selected`, `hover`, …) |
| `#id` | element id |

### Values

- Colors: `#rgb`, `#rrggbb`, `rgb()`, `rgba()`, names (`gold`, `neon`, …)
- Numbers / lengths: `52`, `1.5`, `12px`
- Keywords: `star`, `none`, …

## Rust API

```rust
use velvet_style::{parse_stylesheet, resolve, StyleQuery, StyleRegistry};

let mut reg = StyleRegistry::new();
reg.load_str("casino", include_str!("casino.vcss"))?;

let style = reg.resolve(
    &StyleQuery::class("button").with_state("selected").with_id("start")
);
let bg = style.background().rgb_tuple();
let color = style.color_text().rgb_tuple();
```

## Story language (invoke)

```text
call style.load:
    name: casino
    path: styles/casino.vcss

call style.use:
    name: casino
```

(Host wires `StyleRegistry` like `AnimStoryHost`.)

## Design rules

- **Tools**: parse + resolve only; your renderer applies properties.
- Demos may ship a default `.vcss`; authors override freely.
