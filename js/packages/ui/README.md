# @velvet/ui

The 2D layer of a Velvet JS game: a stage that draws a fixed 16:9 surface in any window, and the
HUD primitives of the Persona look (slanted plates, keycaps, chips, meters, buttons, tooltips,
toasts, modals and menus). It is DOM, SVG and CSS, with no framework, and it sits over a 3D canvas.

It has six parts:

- **The stage** (`createStage`): every position, size and click is written in logical pixels, 1920 by 1080
  unless the game says otherwise. The stage scales the logical surface into the window with letterbox
  bars, never stretches it, and converts pointer positions back exactly.
- **The board view** (`createBoardView`): a pannable, zoomable world inside the stage, for card games and
  maps. Items sit at world positions, and a drag helper reports the pointer and the drop target.
- **The components**: small factories that return `{ el, update(props), destroy() }`. Modal, toast and
  the stage have a few extra methods (`open`, `show`, `setNative`); their docs say which.
- **The rebinding panel** (`createRebindPanel`): the game's actions, each with the control in each slot, a
  capture state for the next input, and reset buttons. It works through a port that `@velvet/core` builds.
- **The screen kit**: a screen stack (`createScreenStack`), the save list (`createSaveSlots`), notices with a history
  (`createNotices`), a visual-novel dialogue box (`createDialogueBox`), stat bars (`createBar`) and dirty marks that
  batch DOM writes per frame (`createDirtyMarks`). They are the screen-level pieces that every game rebuilds, and
  none of them knows a game's names or data.
- **The form controls and the settings panel**: a slider (`createSlider`), a toggle (`createToggle`), a choice between
  options (`createChoice`), and the settings screen that binds them to a game's settings store (`createSettingsPanel`).

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
| `createKeycap({ key, textCase })` | `key`, `textCase` | A key drawn as a keycap: `"KeyE"` shows `E`, `"Space"` shows `Space`. `"MouseLeft"`, `"MouseRight"`, `"MouseMiddle"` and `"MouseWheel"` draw a mouse with that part lit. `keyLabel(code)` gives the label. `textCase` is `"upper"` (the default: the stylesheet draws capitals, so `Esc` shows `ESC`) or `"as-is"`, which keeps the case as written, such as `Esc` or `Espacio`. |
| `createChip({ label, count })` | see left | A small label with an optional count badge. |
| `createMeter({ label, value, max, segments, tone })` | see left | `role="meter"`. `segments` draws cells instead of a fill. `tone` is `accent`, `gold` or `danger`. |
| `createTooltip({ stage })` | `anchor`, `text`, `placement` | Points at `anchor`, above it by default, and stays inside the stage. `anchor: null` hides it. |
| `createToast({ maxVisible, duration, fadeMs })` | see left | Adds `show(message, tone?)`. Notices queue, and fade after `duration` ms. |
| `createModal({ title, body, actions, closeOnEscape })` | see left | Adds `open()` (a promise of the chosen action id, or `null` on Escape) and `close(result?)`. Focus stays inside while it is open. |
| `createMenu({ items, label, onSelect })` | see left | Arrow keys, Home, End, Enter and Space. Hovering an item moves the focus to it. Adds `focus(index)` and the read-only `focusedIndex`. `focus` clamps an index past either end, and a disabled item gives the focus to the nearest enabled one. The menu follows the DOM focus, so an item that a click or `el.focus()` focuses becomes the focused item. Its keys call `preventDefault`, so a game's `ActionMap` ignores them. |

Each factory appends nothing by itself: put `el` in the stage's `root` (or in any element of the page).

## Focus

A hidden or removed element must never keep the keyboard focus, or a key the game listens for can stop working. Every
piece of the package that hides or destroys something with focus (the menus, buttons, modals, screens, save slots,
notices, the dialogue box, the rebinding panel, the settings panel, the controls, the board's items, the stage, and an
element that `setShown` hides) gives the focus up when it does. `releaseFocus` does the same for the game's own elements:

    import { releaseFocus } from "@velvet/ui";

    function closeInventory(): void {
      releaseFocus(inventoryEl, openerButton); // the focus moves to the button that opened it
      inventoryEl.hidden = true;
    }

`releaseFocus(container, fallback?)` does nothing when the focus is outside `container`. When it is inside, the focus
moves to `fallback` if that is given and can take the focus now (connected, enabled, and not in a hidden or inert part),
and otherwise the focused element is blurred. Call it before the element is hidden or removed.

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
| `createBoardView(stage, { worldBounds, minZoom, maxZoom, initialZoom, visibleArea })` | Adds the layer. `worldBounds` is `{ x, y, w, h }`. The zoom range defaults to 0.25 to 4, and the view starts at zoom 1 (or the nearest zoom in range) with the world centred in the visible area. `visibleArea` is the insets `{ top, right, bottom, left }` of the HUD panels that cover the stage (see below); a side left out is 0. |
| `el` | The world layer, inside the stage's root. It has the class `vu-board`, and its look is set inline. |
| `zoom` | Logical pixels per world unit. Settable: it zooms around the centre of the visible area, within the range. |
| `zoomAt(zoom, point)` | Zooms around a logical point, which keeps the world point under it in place unless the bounds clamp the pan. |
| `pan` | Where the world's origin sits, in logical pixels. Settable, and clamped. |
| `panBy(dx, dy)` | Pans by a distance in logical pixels, the same space as the stage. Use it from the keyboard. |
| `fit({ x, y, w, h })` | Shows a world rectangle centred in the visible area, as large as that area and the zoom range allow. |
| `setVisibleArea({ top, right, bottom, left })` | Sets the part of the stage that the view frames. The view is clamped to it again, and `onChange` runs whenever the area changes, even when the view stays put. Setting the same area again does nothing. |
| `visibleWorldRect()` | The world rectangle that the visible area shows now. A game whose cards are placed in world units can pass it as the `bounds` of `board.freeSpot` (`@velvet/cards`), so that a new stack lands where the player sees it. The cards board counts its own steps, so the game must map them to world units first. |
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

**The visible area.** A top bar or a side panel can cover part of the stage. Tell the board the insets, and it frames
the world in what is left: the first centring, `fit`, the zoom around the centre and the pan clamp all use that part.

    const board = createBoardView(stage, { worldBounds, visibleArea: { top: 96, right: 360 } });
    // when a panel opens or closes:
    board.setVisibleArea(panelOpen ? { top: 96, right: 360 } : { top: 96 });
    const spot = board.visibleWorldRect(); // the world rectangle the player can see now

## The rebinding panel

A panel that lets the player change the controls. It lists each slot of each action with its control, and a
row for a new binding when every slot is taken. Choosing a row starts a capture of the next input: a key, a mouse
button, a gamepad button or a stick push, as the devices allow. A line under the list says what happened: the
binding, a conflict with another action, a cancel.

    import { createRebindPort } from "@velvet/core";
    import { createRebindPanel } from "@velvet/ui";

    const panel = createRebindPanel({
      port: createRebindPort(input), // input: the game's ActionMap
      actions: [
        { id: "confirm", labelKey: "action.confirm" },
        { id: "move", labelKey: "action.move" },
      ],
      t: (key, args) => i18n.t(key, args), // the game's text lookup
      policy: "swap", // optional: "swap" (default), "refuse" or "allow"
      devices: ["keyboard", "gamepad"], // optional: the default is all three
    });
    stage.root.append(panel.el);

| Member | What it is |
|---|---|
| `createRebindPanel({ port, actions, t, policy?, devices?, timeoutMs? })` | Creates the panel. `timeoutMs` (default 8000) ends a capture that waits too long. |
| `el` | The panel's element. Put it in the stage's root or in the page. |
| `update({ actions?, t?, policy?, devices?, timeoutMs? })` | Changes what is given. A new `t` redraws every text, which is how a language change reaches the panel. |
| `refresh()` | Reads the slots from the port again. Call it after a change that the panel did not make. |
| `cancel()` | Ends the capture in progress. |
| `capturing` | True while a capture waits for an input. |
| `destroy()` | Ends a capture, removes the element and its listeners. |

**Keys.** Enter, Space or a click on a row starts a capture for that slot. Escape cancels it, and so does a
timeout. Delete clears the focused slot. The two buttons restore the action of the focused row, or every action.
While a capture waits, no other choice starts, and the keys belong to the capture. The arrows, Home and End move
the focus through the list, as they do in any menu.

**A pad.** The panel follows the menu rules, so a gamepad drives it when the game forwards the pad as keys with
`startPadMenuKeys` from `@velvet/core`. Start it while the panel is open and stop it when the panel closes.

**The port.** `RebindPort` is the set of operations that the panel uses: `slots(action)`, `capture(action, slot,
request)`, `clear`, `resetAction` and `resetAll`. The panel imports nothing from core, so the types are declared
here, and `createRebindPort` in `@velvet/core` provides them.

**Texts.** Every text comes from `t`. When the lookup has no text for a key, it returns the key itself. The panel
then shows a plain form of the row (the action's id, the device's name and the control's code), so the screen still
works while a catalog is incomplete.

| Key | Arguments | Used for |
|---|---|---|
| `input.rebind.title` | | The panel's title. |
| `input.rebind.list` | | The name of the list, for screen readers. |
| `input.rebind.hint` | | The line shown when the panel opens. |
| `input.rebind.capture` | | The prompt while a capture waits. |
| `input.rebind.row` | `action`, `device`, `control` | A row with a bound slot. |
| `input.rebind.row.empty` | `action` | A row with an empty slot. |
| `input.rebind.row.capturing` | `action` | The row whose capture is waiting. |
| `input.rebind.inverted` | `control` | A control that is an inverted axis. |
| `input.rebind.bound` | `control` | A capture that was bound. |
| `input.rebind.swapped` | `control`, `action` | A swap with another action. |
| `input.rebind.shared` | `control`, `action` | A conflict that was allowed. |
| `input.rebind.refused` | `control`, `action` | A conflict that was refused. |
| `input.rebind.cancelled` | | A capture that was cancelled or timed out. |
| `input.rebind.failed` | | A capture that failed. |
| `input.rebind.cleared` | | After Delete. |
| `input.rebind.reset.action` | | The button that resets the focused action. |
| `input.rebind.reset.all` | | The button that resets every action. |
| `input.rebind.reset.done` | | After a reset. |
| `input.device.keyboard`, `input.device.mouse`, `input.device.gamepad` | | The device names. |

The control names are the display keys of `@velvet/core` (`input.key.Space`, `input.pad.a` and so on); the list is in
`docs/formats/INPUT_BINDINGS.md`.

The panel sets no hover or focus geometry. A row and a button change colour only, from the menu and button styles.
The panel's own width is set inline, and its list scrolls when it is taller than 640 logical pixels.

## Screens

`createScreenStack(host, options)` keeps the game's screens in one order: the title, the game, the pause menu, the
save list and the dialogues are pushed, popped and replaced on it. The top screen is the one the player uses.

    import { createScreenStack, type Screen } from "@velvet/ui";

    const screens = createScreenStack(stage.root);
    const game: Screen = {
      id: "game",
      el: gameEl,
      onBack: () => {
        screens.push(pause); // Escape or a pad's B opens the pause menu over the game
        return false; // the game stays on the stack
      },
    };
    let pausedBefore = false;
    const pause: Screen = {
      id: "pause",
      el: pauseEl,
      kind: "overlay",
      onEnter: () => {
        pausedBefore = clock.paused; // remember whether the player had paused the game first
        clock.pause();
      },
      onLeave: () => {
        if (!pausedBefore) clock.resume();
      },
      onBack: () => {
        screens.pop(); // Escape closes the menu
        return false;
      },
    };

    screens.push(game);

A screen has an `id` (no two screens on the stack share one), an `el` and optional hooks. Its `kind` is `"screen"`
(the default), which covers the screens below it so that they are hidden, or `"overlay"`, which sits over the screen
below under a scrim and leaves it visible. The screens under the top one are inert, so neither the pointer nor the
keyboard reaches them.

| Member | What it is |
|---|---|
| `createScreenStack(host, { transitionMs? })` | Makes the stack inside `host`. A static host is made relative, and `destroy` puts it back. `transitionMs` defaults to 180, and is zero under reduced motion. |
| `push(screen)`, `pop()`, `replace(screen)` | Put a screen on top, take the top one off (and return it), or swap the top one. `replace` leaves the screens below as they are. |
| `popTo(id)` | Takes every screen above `id` off, in one step. Returns false when no such screen is on the stack. |
| `back()` | The back action. The stack calls it itself for Escape and a pad's B. Returns true when the key was taken. |
| `top`, `depth`, `has(id)`, `ids()` | What is on the stack, from the bottom up. |
| `onChange(listener)` | Runs after each change, with the top screen (or null). Returns the function that removes it. |
| `el`, `destroy()` | The layer that holds the screens. `destroy` removes it and the listeners, and runs no hooks. |

The hooks, in the order they run:

- `onEnter` when the screen comes on the stack, or replaces another.
- `onPause` when another screen is pushed over it, and `onResume` when that screen leaves.
- `onLeave` when the screen leaves the stack, or is replaced.
- `onBack` on Escape or a pad's B, while the screen is on top. Return `false` to keep the screen (for example after it
  pushed another one). Any other result pops it. A screen without `onBack` is popped too, unless it is the only one.
- `focusTarget()` gives the element that takes the focus when the screen comes on top. By default it is the first
  focusable element of the screen.

**Pausing.** A menu stops the clock when it opens (`onEnter`) and starts it again when it leaves (`onLeave`). Remember
whether the player had paused the game before the menu, so that closing the menu does not restart a clock that the
player stopped; the example above does that. A screen that is covered by another one gets `onPause` and `onResume`
instead, for the things that must stop while it is not on top.

**Focus.** Each screen gets back the element that had the focus when another screen covered it. While a screen is on
the stack, the focus stays in it: a click on empty space that takes the focus away brings it back to the top screen.

**Keys.** Escape reaches the stack on the window, after the controls have had it. A control that already took the key
(a menu, a modal, a list) prevents the default, so the stack leaves it alone. A pad's B is an Escape key from
`startPadMenuKeys` in `@velvet/core`, so it works the same way.

**Changes.** Do not push or pop from inside a hook: the change would run inside the one that made it. `onBack` is the
exception: it may push a screen, if it returns `false`. From a hook that must change the stack, use an event or a
microtask.

**Transitions.** A screen fades and moves a little with CSS opacity and transform. The stack sets the fade length on
its layer as `--vu-screen-ms`, so the stylesheet and the timer agree.

## Save slots

`createSaveSlots(options)` is the list of a game's save slots: an autosave row and a set of manual rows (three by
default). Each row shows the save's title, its play time, when it was written, and a thumbnail when there is one. In
`"load"` mode the player reads a saved slot. In `"save"` mode the player writes a manual slot, and the autosave is
read-only.

    import { createSaveStore } from "@velvet/core";
    import { createSaveSlots } from "@velvet/ui";

    const store = createSaveStore({ game: "my-game", version: 1 });
    const saves = createSaveSlots({
      store,
      mode: "save",
      t: (key, args) => i18n.t(key, args), // the game's text lookup
      collect: () => ({
        title: "Town square",
        playTimeMs: clock.elapsed * 1000,
        thumbnail: snapshotDataUrl(), // a PNG, JPEG or WebP data URL, or null
        state: game.toJSON(), // the game's data, as JSON
      }),
      onLoad: (record) => game.fromJSON(record.state),
      onSaved: () => notices.notify({ message: "Game saved." }),
    });
    screens.push({ id: "saves", el: saves.el, kind: "overlay", focusTarget: () => saves.focusTarget() });

The store is any object with `list`, `load`, `save` and `remove`, and the store of `@velvet/core` fits. A slot holds a
record `{ meta, state }`: the `meta` (title, play time and thumbnail) is what the list shows, and the `state` is the
game's data as `collect` gave it. The list writes this wrapper, so the store's version covers it.

| Option or prop | What it is |
|---|---|
| `store` | The store. `list()`, `load(slot)`, `save(slot, data)` and `remove(slot)`. |
| `mode` | `"load"` (the default) or `"save"`. |
| `manualSlots` | How many manual slots, from 1 to 99. Default 3. Their names are `slot-1`, `slot-2` and so on; `slotPrefix` changes the start. |
| `autosave` | The autosave's slot name. Default `"auto"`. `null` gives no autosave row. |
| `collect(slot)` | Gives the draft to write: `title`, `playTimeMs`, `thumbnail` and `state`. The state must be JSON data and must not be undefined. The thumbnail must be a PNG, JPEG or WebP data URL, or null, and another value fails the save. `collect` is needed to save, and for `autosave()`. |
| `onLoad(record)`, `onSaved(record)`, `onDeleted(slot)` | Run after the player's pick, with the record or the name of the slot. |
| `onError(error, slot)` | Runs when a read, write or delete fails. `slot` is null when the list of slots itself could not be read. |
| `t`, `locale` | The text lookup, and the language tag for the dates (null uses the page's language). |

| Member | What it is |
|---|---|
| `el` | The list. It is a component, so `update({ mode, t, locale })` redraws it. |
| `refresh()` | Reads every slot again and draws the rows. |
| `autosave()` | Writes the autosave now, without a question, from `collect`. It rejects when the write fails, so the game can react. |
| `focusTarget()` | The first row that can be picked, or null. |
| `destroy()` | Removes the list. |

**Keys.** The arrows, Home and End move between the rows, skipping the ones that cannot be picked. Enter or Space
picks the focused row. Delete asks to delete it. A pad drives the list through the same keys. A write over a save asks
first, and so does a delete. A thumbnail from any other source than a PNG, JPEG or WebP data URL is not shown.

**Helpers.** `formatPlayTime(ms)` gives `"1:02:03"`. `formatSaveTime(iso, locale)` gives a short date and time.
`isSafeThumbnail(value)` tells a thumbnail that the list will show.

**Texts.** The keys are `save.autosave`, `save.slot` (with `number`), `save.empty`, `save.damaged`, `save.untitled`,
`save.delete`, `save.confirm.overwrite.title` and `.body`, `save.confirm.delete.title` and `.body` (both with `name`),
`save.confirm.yes`, `save.confirm.no`, and `save.status.saved`, `.loaded`, `.deleted`, `.empty` and `.failed`. Each
falls back to English when the lookup has no text for it.

## Notices

`createNotices(options)` is a column of short messages in one corner of the surface. The notices queue by priority: the
most important shows first, and at most `maxVisible` show at once (three by default). A notice stays for its duration,
then fades. A notice can carry buttons. A sticky one stays until it is dismissed.

    import { createNotices } from "@velvet/ui";

    const notices = createNotices({
      host: stage.root,
      anchor: "bottom-right",
      avoid: () => (dialogue.isOpen ? stage.rectOf(dialogue.el) : null), // the column never covers this
    });
    notices.notify({ message: "Sold for 3" });
    notices.notify({ message: "Ana is hungry", tone: "warn", key: "hunger-ana" }); // a repeat is counted
    notices.notify({
      message: "Accept the offer?",
      priority: 10,
      durationMs: null,
      actions: [
        { id: "yes", label: "Yes" },
        { id: "no", label: "No" },
      ],
      onAction: (id) => offer(id === "yes"),
    });

| Member | What it is |
|---|---|
| `createNotices({ host, anchor, avoid, maxVisible, t, width, height, margin, durationMs, fadeMs, historyLimit })` | Makes the column and the history drawer inside `host`. The surface is 1920 by 1080 by default. `durationMs` is the default time on screen (4000). `historyLimit` is 50 by default. |
| `notify(spec)` | Raises a notice, and returns its id. `spec`: `message` (plain text), `tone` (`"info"`, `"warn"` or `"danger"`), `priority` (default 0; a higher one shows first and can take the place of a lower one), `durationMs` (`null` is sticky), `key` (while a notice with the same key is on screen, the count goes up instead), `actions` with `onAction(id)`. |
| `dismiss(id)`, `clear()` | Take a notice off, with its fade, or every notice off at once. The history stays. |
| `visible()`, `history()` | The ids on screen, and the history newest first, each with its `count`. |
| `openHistory()`, `closeHistory()`, `toggleHistory()`, `historyOpen` | The drawer of past notices. Escape closes it, and the focus goes back to where it was. |
| `relayout()`, `update({ anchor, avoid, maxVisible, t })` | Place the column again. `avoid` is a rectangle in logical pixels, or a function that gives one or null. Call `relayout` when the rectangle moves. |
| `el`, `destroy()` | The column. `destroy` removes the column and the drawer. |

**Focus.** A notice never takes the focus. Its buttons are reached with Tab, or the game moves the focus to one of them.

**Avoiding a rectangle.** The column never covers `avoid`. When it would, the column moves above or below the rectangle,
whichever side has more room. The notices that do not fit wait in the queue, and come back when the rectangle moves.
Make the notices after the screens, so they are drawn above them.

**Texts.** `notice.history.title`, `notice.history.close` and `notice.history.empty` (the drawer). The counts are
written as `×2`.

## Dialogue box

`createDialogueBox(options)` plays a conversation. It shows the speaker's name on the plate, a portrait, the text
written out letter by letter, the choices under the text, and a backlog. It plays a guion conversation as it is, and
its types match `@velvet/guion`'s `Shown` and `Conversation`, so the UI package does not import guion.

    import { GuionLibrary } from "@velvet/guion";
    import { createDialogueBox } from "@velvet/ui";

    const dialogue = createDialogueBox({
      speed: 40, // letters a second; 0 writes each line at once
      portrait: (line) => portraitOf(line.who, line.mood), // the caller resolves the art
      onLine: (line) => voice.play(line.who),
      onEnd: (end) => {
        if (end.type === "vn") game.startScene(end.to);
      },
    });
    stage.root.append(dialogue.el);

    const library = await GuionLibrary.create();
    dialogue.play(library.start("intro.arrival"));

    // once per update of the game's loop, so that the text stops when the game is paused:
    dialogue.tick(seconds);

| Option or prop | What it is |
|---|---|
| `speed` | Letters a second. Default 40. Zero writes each line at once. |
| `fastForwardFactor` | How much faster the text runs while fast-forward is on. Default 4. |
| `autoAdvance`, `autoDelaySeconds` | Go on by themselves, a while (1.5 seconds by default) after a line is written. |
| `portrait(line)` | Gives the element for the portrait, or null for none. |
| `t` | The text lookup for the buttons and the backlog. |
| `onLine(line)`, `onLetter(letter)`, `onChoice(choice)` | Hooks for voice and sound. `onLetter` runs for each letter that is written, not for spaces, so throttle it if it is loud. |
| `onEnd(shown)` | Runs once, when the conversation ends or hands over to a visual-novel scene. The box hides itself first. |

| Member | What it is |
|---|---|
| `play(session)` | Plays a conversation, which replaces one that is playing. A session has `current()`, `next()` and `choose(index)`. |
| `tick(seconds)` | Moves the text and the auto-advance on by `seconds` of game time. The box has no timers of its own. |
| `advance()` | Finishes a line that is being written, or goes on to the next box. |
| `choose(index)` | Takes an option of the choice on screen. An option that is closed is ignored. |
| `setFastForward(on)`, `setSkipping(on)` | Fast-forward runs the text faster. Skip writes each line at once and goes on until a choice or the end. |
| `openBacklog()`, `closeBacklog()`, `backlog` | The lines and the choices so far, oldest first. |
| `stop()` | Hides the box without calling `onEnd`. |
| `isOpen`, `shown`, `fastForward`, `skipping` | What the box is doing now. `shown` keeps the end or the hand-over after the conversation ends. |
| `update(props)`, `el`, `destroy()` | Change the props, the element, and the cleanup. |

**Keys.** The first line of a conversation takes the focus into the box, and so does a line that comes on while the
focus is in the box or nowhere. The Next button then has the focus, so Enter or Space goes on, and so does a pad's A
(through `startPadMenuKeys`). A click on the text goes on too. A choice takes the arrows and Enter. Auto and Skip are
toggles, Log opens the backlog, and Escape closes it.

**Texts.** `dialogue.next`, `dialogue.auto`, `dialogue.skip`, `dialogue.log`, `dialogue.log.title`, `dialogue.log.close`
and `dialogue.choices`.

The box never decides a rule. The conversation says what it shows, and the box hands back the player's choices.

**With a screen stack.** Put the box in an overlay screen when a stack runs the game. The stack keeps the focus on
its top screen, so the box's buttons need to be inside that screen to take the focus and to answer Enter.

## Stat bars and dirty marks

`createBar({ label, value, max, tone, text })` is a thin bar with role `meter`. Its fill is drawn with a transform, so a
change of value moves no layout. `tone` is `"accent"`, `"gold"` or `"danger"`, and `text` is drawn over the bar (such as
`"70 / 100"`) or hidden with null.

`createDirtyMarks({ schedule? })` batches DOM writes. Each element registers the function that draws it. `mark(name)`
asks for a redraw, and every mark made in a frame is drawn once, in one flush. The flush is scheduled on the next
animation frame, or the game can call `flush()` itself with `schedule: null`. The write helpers write nothing when the
value is already there: `setText(node, text)`, `setShown(element, shown)` and `setActive(element, active)`.

    const marks = createDirtyMarks();
    const hunger = createBar({ label: "Hunger", value: 70, max: 100, text: "70 / 100" });
    stage.root.append(hunger.el);
    marks.register("gold", () => setText(goldEl, String(run.gold)));
    marks.mark("gold"); // drawn at the next frame, once however many marks came in

## Form controls

`createSlider`, `createToggle` and `createChoice` are the controls of a settings screen. Each one returns
`{ el, value, setValue(value, { silent? }), onChange, setDisabled(disabled), destroy() }`. A silent `setValue` moves
the control without calling `onChange`, which is how a change of the store reaches the control without being written
back. The pad drives them through the keys that `startPadMenuKeys` from `@velvet/core` sends: the arrows, Enter (A)
and Escape (B).

    import { createChoice, createSlider, createToggle } from "@velvet/ui";

    const music = createSlider({
      label: "Música",
      min: 0,
      max: 10,
      format: (value) => `${value * 10} %`,
      value: 7,
      onChange: (value) => audio.setMusic(value / 10),
    });
    const subtitles = createToggle({ label: "Subtítulos", value: true, onChange: (on) => settings.set("subtitles", on) });
    const language = createChoice({
      label: "Idioma",
      options: [{ value: "es", label: "Español" }, { value: "en", label: "English" }],
      wrap: false,
    });
    stage.root.append(music.el, subtitles.el, language.el);

| Factory | Options | Keys and pointer |
|---|---|---|
| `createSlider({ min, max, step?, value?, format?, label?, onChange?, disabled? })` | `step` defaults to 1. `format(value)` gives the text beside the track. | Left and Down lower it, Right and Up raise it, Home and End go to the ends. A press or a drag on the track sets it. |
| `createToggle({ label?, value?, onChange?, disabled? })` | `value` defaults to false. | Enter and Space turn it over, and so does a click. |
| `createChoice({ label?, options, value?, wrap?, onChange?, disabled? })` | `options` are `{ value, label }`, at least one. `wrap` defaults to true: the arrows go round the ends. | Left and Up give the previous option, Right and Down the next, Home and End the first and the last. The two arrows on the sides do the same for the mouse. |

The slider moves in steps from `min`, and a value is snapped to the nearest step and kept in range. Its largest value
is the last step that is not above `max`, which is also `aria-valuemax`. The slider has role `slider`, the toggle
`switch` with `aria-checked`, and the choice `listbox`, whose one visible option is its active descendant. A disabled
control leaves the Tab order and gets `aria-disabled`. Hover and focus change colour only.

The toggle is a `div` on purpose: a pad's A reaches it as an Enter key, and a focused button would turn twice on that.

## Settings panel

`createSettingsPanel({ store, sections, t?, onReset? })` builds a settings screen from sections of rows. Each row is a
label and one control, bound to one setting of a store. The store is the one that `@velvet/core` makes with
`createSettingsStore`. The panel declares only the methods it uses, so this package does not import core.

    import { createSettingsStore } from "@velvet/core";
    import { createSettingsPanel } from "@velvet/ui";

    const settings = createSettingsStore({
      namespace: "my-game",
      version: 1,
      defaults: { volume: 7, subtitles: true, language: "es" },
    });
    await settings.load();

    const panel = createSettingsPanel({
      store: settings,
      t: (key, args) => i18n.t(key, args), // the game's text lookup
      sections: [
        {
          title: "settings.sound",
          items: [{ kind: "slider", key: "volume", label: "settings.volume", min: 0, max: 10, format: (v) => `${v * 10} %` }],
        },
        {
          title: "settings.screen",
          items: [
            { kind: "toggle", key: "subtitles", label: "settings.subtitles" },
            {
              kind: "choice",
              key: "language",
              label: "settings.language",
              options: [{ value: "es", label: "Español" }, { value: "en", label: "English" }],
            },
          ],
        },
      ],
    });
    screens.push({ id: "settings", el: panel.el, kind: "overlay" });

| Member | What it is |
|---|---|
| `createSettingsPanel({ store, sections, t?, onReset? })` | Builds the panel with a row for each item, at the store's values. |
| `store` | `get(key)`, `set(key, value)` and `subscribe(key, listener)` are required. `reset()` is optional: when the store has it, the panel shows a "Reset to defaults" button that calls it, and then `onReset`. The settings store of `@velvet/core` has no `reset` yet, so the button stays hidden with it. |
| `sections` | `{ title, items }`. An item is `{ kind: "slider" \| "toggle" \| "choice", key, label, ...options }`, with the options of its control. A key may be in one row only. |
| `t` | The text lookup. The titles, the labels, the option labels and the reset text (`settings.reset`) go through it. A lookup with no text returns the key, which is then shown as it is. |
| `el`, `destroy()` | The panel. `destroy` removes it and stops following the store; the store keeps its values. |

A change of a control writes the store at once. A change that the store makes (another part of the game) moves the
control silently, so nothing is written back. Up and Down move the focus through the rows and the reset button, and
wrap at the ends; Left and Right change the focused control, and Enter or Space turn a toggle. A key that the store
does not have makes the panel throw a RangeError when it is created, and a stored value of the wrong type shows the
control's own default. The panel is built once: to show the texts in another language, make it again.

## Error overlay

    import { createErrorOverlay } from "@velvet/ui";
    import { formatReport } from "@velvet/core";   // the overlay does not import core: it takes the text

    const overlay = createErrorOverlay({
      host: document.body,
      report: { id: report.id, message: report.message, text: formatReport(report) },
      onCopy: (copied) => log.info("report copied", { copied }),
      onContinue: () => resumeGame(),
      onReload: () => location.reload(),
    });
    // overlay.close() hides it and gives the focus back; overlay.destroy() removes it. Make a new one for a new report.

A panel over the whole window, in the modal's look, titled "Something went wrong". The report text is shown in a box
that scrolls, as text and never as markup. The report is a string, or an object with its `text` and, optionally, its
`id` (shown as "Report {id}") and its `message` (shown as the headline). The overlay opens at once, with the focus on
Continue.

- **Copy** copies the text through the clipboard. When the page has no clipboard, or it refuses, the text is selected
  so that Ctrl+C copies it, and the copy command is tried. `onCopy(copied)` says which way it went.
- **Continue**, and Escape, close the overlay, and then call `onContinue`. `close()` closes it without the callback.
- **Reload** calls `onReload`, which defaults to reloading the page. The overlay stays open, so the game can decide.
- **Keys and pad.** Tab and Shift+Tab stay inside it. The left and right arrows move between the three buttons and wrap,
  so a pad's d-pad works. The arrows scroll the report box when it has the focus. Enter presses the focused button;
  a pad's A does too once the game maps the pad to the keys (`startPadMenuKeys` in @velvet/core).
- **Labels.** `t(key, args)` gives every label, with the same shape as the rebinding panel's translator. The keys are
  `error.title`, `error.body`, `error.reportText`, `error.reportId` (with `{id}`), `error.copy`, `error.copied`,
  `error.copyFailed`, `error.continue` and `error.reload`. Without `t`, the English labels are used.
- When the overlay closes, the focus goes back to the element that had it before it opened, or is released when that
  element is gone. The look is in `velvet-ui.css`, under "Error overlay".

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
the pointer round trip, the bars, the focus trap, the toast timing, the board's transforms, zoom, clamp, hit testing and drags, the
stylesheet, and the rebinding panel's rows, captures, resets, keys, cleanup and language change. The screen kit has its own suites:
the screen stack (layers, hooks, focus, Escape and transitions), the save list (rows, writes, confirmations, keys and
damaged slots), the notices (queue, expiry, repeats, history and the avoided rectangle), the dialogue box (writing,
choices, skip, auto-advance, backlog), the stat bar and the dirty marks, and the stylesheet's rules for them. The focus
suite (`focus.test.ts`) drives each surface to the state where it holds the focus, hides or destroys it, and checks
that the focus is outside it. The form controls, the settings panel and their stylesheet rules have suites of their own
(`components/slider.test.ts`, `toggle.test.ts`, `choice.test.ts`, `settings-panel.test.ts`, `form-css.test.ts`).

`demo/index.html` and `demo/main.ts` show the stage and every component. They load the stylesheet from
`../src/velvet-ui.css`. Vite is not added yet, so open the page with Vite (or another server that runs
`.ts` modules) when you want to see it.
