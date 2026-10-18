# @velvet/ui

The 2D layer of a Velvet JS game: a stage that draws a fixed 16:9 surface in any window, and the
HUD primitives of the Persona look (slanted plates, keycaps, chips, meters, buttons, tooltips,
toasts, modals and menus). It is DOM, SVG and CSS, with no framework, and it sits over a 3D canvas.

It has two parts:

- **The stage** (`createStage`): every position, size and click is written in logical pixels, 1920 by 1080
  unless the game says otherwise. The stage scales the logical surface into the window with letterbox
  bars, never stretches it, and converts pointer positions back exactly.
- **The components**: small factories that return `{ el, update(props), destroy() }`. Modal, toast and
  the stage have a few extra methods (`open`, `show`, `setNative`); their docs say which.

## Usage

Load the look once, and create the stage in the page's game element:

    import "@velvet/ui/velvet-ui.css";
    import { createButton, createKeycap, createMeter, createStage, createTooltip } from "@velvet/ui";

    const stage = createStage(document.getElementById("game")!, { width: 1920, height: 1080 });

    // A canvas at the screen's own pixels. Its logical size is the stage's, and k maps logical
    // units onto its backing store, so text and shapes stay sharp.
    const canvas = document.createElement("canvas");
    canvas.style.cssText = "position:absolute;left:0;top:0;width:1920px;height:1080px";
    stage.root.append(canvas);
    stage.onResize(() => {
      canvas.width = stage.pixelWidth;
      canvas.height = stage.pixelHeight;
      canvas.getContext("2d")?.setTransform(stage.k, 0, 0, stage.k, 0, 0);
      // ...draw the frame here, in logical units
    });

    // HUD: every component goes inside stage.root, so its positions are logical.
    const save = createButton({ label: "Guardar", hotkey: "F5", onPress: () => saveGame() });
    const health = createMeter({ label: "Salud", value: 70, segments: 10 });
    const tip = createTooltip({ stage });
    stage.root.append(save.el, health.el, tip.el);
    save.el.addEventListener("pointerenter", () => tip.update({ anchor: save.el, text: "Ranura 1" }));
    save.el.addEventListener("pointerleave", () => tip.update({ anchor: null }));

    // Game input: the stage converts client pixels; null means the pointer is over a bar.
    window.addEventListener("pointerdown", (event) => {
      const point = stage.toLogical(event.clientX, event.clientY);
      if (point !== null) clickAt(point.x, point.y);
    });

## The stage

| Member | What it is |
|---|---|
| `createStage(host, options)` | Fills `host` with the stage. Options: `width`, `height` (default 1920 and 1080), `native` (default `true`). |
| `root` | The logical-size element. Put the game's elements in it. |
| `pixelWidth`, `pixelHeight` | The backing-store size of a canvas that covers the presented 16:9 area. |
| `k` | Backing-store pixels per logical pixel. In native mode these are device pixels. With `native: false`, `k` is 1 and the compositor scales the canvas. |
| `scale`, `layout` | CSS pixels per logical pixel, and a snapshot of the whole presentation. |
| `toLogical(x, y)` | Client pixels to logical pixels, or `null` on the bars and outside the window. |
| `toLogicalClamped(x, y)` | Like `toLogical`, but never `null`: a point on the bars or outside the window is clamped to the nearest edge of the surface. Use it for a drag that may leave the surface. A window with no area gives the origin. |
| `isInside(x, y)` | `true` on the presented 16:9 area, `false` on the bars. The same answer as `toLogical(x, y) !== null`. |
| `toClient(x, y)` | The inverse of `toLogical`. |
| `rectOf(element)` | An element's box in logical pixels. |
| `onResize(listener)` | Runs after a change of the layout: a window resize, a new device pixel ratio, or `setNative`. It returns a function that removes the listener. |
| `setNative(on)` | Switches native drawing on or off, for a settings toggle. |
| `refresh()`, `destroy()` | Measures again now; removes the stage and its listeners. |

Rules that the stage keeps:

- The picture keeps 16:9 (the logical aspect) and is never stretched. The rest of the window is bars.
- The bars are not interactive: clicks and moves on them stop at the viewport, so the game's window
  listeners never see them. `toLogical` returns `null` for them. A drag that may leave the surface uses
  `toLogicalClamped`, which keeps following the pointer at the nearest edge.
- The root's size and position are inline styles, so the stage works without the stylesheet.
- Canvases should be sized to `pixelWidth` by `pixelHeight` and drawn with `k`. When the backing store
  already matches the area's device pixels, the compositor copies it directly.

## Components

Every factory returns `{ el, update(props), destroy() }`. `update` merges the props it is given: a prop
that is left out keeps its value. To clear a text or an optional value, pass `null` or `""`.

| Factory | Props | Notes |
|---|---|---|
| `createPlate({ title })` | `title` | A slanted ink panel with a title tab. Content goes in `body`. |
| `createButton({ label, hotkey, pressed, disabled, onPress })` | see left | A native `<button>`. `pressed` makes it a toggle (`aria-pressed`); the game owns the state. |
| `createKeycap({ key })` | `key` | A key drawn as a keycap: `"KeyE"` shows `E`, `"Space"` shows `Space`. `"MouseLeft"`, `"MouseRight"`, `"MouseMiddle"` and `"MouseWheel"` draw a mouse with that part lit. `keyLabel(code)` gives the label. |
| `createChip({ label, count })` | see left | A small label with an optional count badge. |
| `createMeter({ label, value, max, segments, tone })` | see left | `role="meter"`. `segments` draws cells instead of a fill. `tone` is `accent`, `gold` or `danger`. |
| `createTooltip({ stage })` | `anchor`, `text`, `placement` | Points at `anchor`, above it by default, and stays inside the stage. `anchor: null` hides it. |
| `createToast({ maxVisible, duration, fadeMs })` | see left | Adds `show(message, tone?)`. Notices queue, and fade after `duration` ms. |
| `createModal({ title, body, actions, closeOnEscape })` | see left | Adds `open()` (a promise of the chosen action id, or `null` on Escape) and `close(result?)`. Focus stays inside while it is open. |
| `createMenu({ items, label, onSelect })` | see left | Arrow keys, Home, End, Enter and Space. Hovering an item moves the focus to it. Adds `focus(index)` and the read-only `focusedIndex`. `focus` clamps an index past either end, and a disabled item gives the focus to the nearest enabled one. The menu follows the DOM focus, so an item that a click or `el.focus()` focuses becomes the focused item. Its keys call `preventDefault`, so a game's `ActionMap` ignores them. |

Each factory appends nothing by itself: put `el` in the stage's `root` (or in any element of the page).

## Theme

Colours, fonts and sizes are custom properties on `.vu-root`: `--vu-ink`, `--vu-paper`, `--vu-accent`,
`--vu-accent-ink`, `--vu-gold`, `--vu-gold-hi`, `--vu-danger`, `--vu-muted`, `--vu-scrim`, `--vu-font`,
`--vu-size`, `--vu-size-small`, `--vu-size-title`, `--vu-slant`, `--vu-pad-x`, `--vu-pad-y`, `--vu-gap`,
`--vu-meter-height` and `--vu-duration`. The bars are `--vu-bars`, on `.vu-viewport`. The defaults are the
Persona palette of the Demon Queen menus. Set a property on the root, or on any element above it, to restyle.

Hover, focus and press change colour, brightness or opacity only. They never change a size, a position or
a padding. The stylesheet test checks this rule. With `prefers-reduced-motion`, transitions and fades stop.

## Tests and demo

From `js/`, run `pnpm test` and `pnpm typecheck`. The tests use happy-dom with a fake browser geometry
(`src/test-support.ts`), so they check the letterbox maths, the pixel sizes at pixel ratios 1, 1.5 and 2,
the pointer round trip, the bars, the focus trap, the toast timing and the stylesheet.

`demo/index.html` and `demo/main.ts` show the stage and every component. They load the stylesheet from
`../src/velvet-ui.css`. Vite is not added yet, so open the page with Vite (or another server that runs
`.ts` modules) when you want to see it.
