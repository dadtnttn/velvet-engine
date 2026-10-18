# @velvet/ui

The 2D layer of a Velvet JS game: a stage that draws a fixed 16:9 surface in any window, and the
HUD primitives of the Persona look (slanted plates, keycaps, chips, meters, buttons, tooltips,
toasts, modals and menus). It is DOM, SVG and CSS, with no framework, and it sits over a 3D canvas.

It has three parts:

- **The stage** (`createStage`): every position, size and click is written in logical pixels, 1920 by 1080
  unless the game says otherwise. The stage scales the logical surface into the window with letterbox
  bars, never stretches it, and converts pointer positions back exactly.
- **The board view** (`createBoardView`): a pannable, zoomable world inside the stage, for card games and
  maps. Items sit at world positions, and a drag helper reports the pointer and the drop target.
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

## The board view

`createBoardView(stage, options)` adds a world layer to the stage's root. The world is a plane in
**world units**; the view draws it at a zoom (logical pixels per world unit, so 1 means one world unit is one
logical pixel) and a pan (where the world's origin sits, in logical pixels). Items are elements in the layer,
placed at world positions with CSS transforms, so moving one changes only its transform, and a pan or a zoom
changes only the layer's transform. Neither causes a layout.

    import { createBoardView } from "@velvet/ui";

    const board = createBoardView(stage, { worldBounds: { x: 0, y: 0, w: 4000, h: 2000 } });
    const card = document.createElement("div");
    board.addItem(card, { x: 400, y: 300, w: 112, h: 150, z: 1 });
    const stop = board.enableDrag(card, {
      onMove: ({ el, world, start, origin }) =>
        board.moveItem(el, { x: origin.x + (world.x - start.x), y: origin.y + (world.y - start.y) }),
      onDrop: ({ target }) => {
        if (target !== null) console.log("dropped on", target.dataset["id"]);
      },
    });
    // From the keyboard, once per frame, with the ActionMap's direction: board.panBy(dx, dy).

| Member | What it is |
|---|---|
| `createBoardView(stage, { worldBounds, minZoom, maxZoom, initialZoom })` | Adds the layer. `worldBounds` is `{ x, y, w, h }`. The zoom range defaults to 0.25 to 4, and the view starts at zoom 1 (or the nearest zoom in range) with the world centred. |
| `el` | The world layer, inside the stage's root. It has the class `vu-board`, and its look is set inline. |
| `zoom` | Logical pixels per world unit. Settable: it zooms around the centre of the surface, within the range. |
| `zoomAt(zoom, point)` | Zooms around a logical point, which keeps the world point under it in place unless the bounds clamp the pan. |
| `pan` | Where the world's origin sits, in logical pixels. Settable, and clamped. |
| `panBy(dx, dy)` | Pans by a distance in logical pixels, the same space as the stage. Use it from the keyboard. |
| `fit({ x, y, w, h })` | Shows a world rectangle centred, as large as the zoom range allows. |
| `worldToLogical(x, y)`, `logicalToWorld(x, y)` | Convert between world units and the stage's logical pixels. |
| `onChange(listener)` | Runs after a pan or a zoom, with `{ zoom, pan }`. It returns the function that removes the listener. It does not run for item moves. |
| `addItem(el, { x, y, w, h, z })` | Adds an element as an item, at a box in world units. `z` is an integer and defaults to 0. Throws for an element that is already an item. |
| `moveItem(el, { x, y, z })` | Moves an item, and re-stacks it when `z` is given. A field that is left out keeps its value. |
| `removeItem(el)` | Takes an item off the board. The element is not destroyed, but its drag helpers are stopped. |
| `itemAt(x, y)` | The topmost item at a world point, or null. Higher `z` is on top; for equal `z`, the item added later is on top. A point on the left or top edge is inside, and one on the right or bottom edge is not. |
| `enableDrag(el, callbacks, { threshold })` | Reports a press and drag of an item. The callbacks `onStart`, `onMove`, `onDrop` and `onCancel` each get `{ el, world, start, origin, target }`: the pointer and where the press started, in world units, the item's place when the press started, and the item under the pointer (other than the dragged one), or null. A press that does not pass the threshold (6 logical pixels by default) is a click. It returns the function that stops the helper. |
| `destroy()` | Removes the layer, every listener and every drag helper. |

Rules that the board keeps:

- A press on empty space (the root itself, or the layer outside every item) and a drag pans the view. The pan
  follows the pointer in logical pixels at any zoom. A press on an item, or on an element of the game that
  sits beside the layer (a HUD button), does not pan. The wheel over the board zooms around the pointer, and
  prevents the page from scrolling.
- The view stays on the world. A world wider or taller than the surface always covers it, so the pan cannot
  show empty space past an edge. A smaller world is centred and does not pan. The zoom stays in its range.
- A press starts only on the surface, and a bar press does nothing. A drag that has started keeps going when
  the pointer moves over the bars or leaves the window: the pressed element captures the pointer, and the
  position is clamped to the nearest edge (`toLogicalClamped`).
- The board sets no hover or focus style, so a game styles its items with colour and opacity, as the other
  pieces do, and never with a change of size or position.
- An element in the layer that is not an item (a background plate that the game draws, for example) counts as
  empty space: a press on it pans the view. Make an element interactive by adding it as an item.

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
the pointer round trip, the bars, the focus trap, the toast timing, the board's transforms, zoom, clamp, hit testing and drags, and the
stylesheet.

`demo/index.html` and `demo/main.ts` show the stage and every component. They load the stylesheet from
`../src/velvet-ui.css`. Vite is not added yet, so open the page with Vite (or another server that runs
`.ts` modules) when you want to see it.
