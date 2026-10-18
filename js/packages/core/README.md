# @velvet/core

The core runtime for Velvet JS games. It has five parts:

- `GameLoop`: a fixed-step loop. `update(dt)` runs at 60 Hz by default, and `render(alpha)` runs once per frame.
- `GameClock`: game time fed by the loop, with pause, speed multipliers (1, 2, 3 by default) and days.
- `EventEmitter`: typed events with `on`, `once`, `off` and `emit`.
- `ActionMap`: keys, mouse buttons and the wheel bound to named actions, read with `pressed`, `held` and `released`. It can be scoped to an element, it leaves a control's own keys to the control, and `consume` hides an action for the rest of a frame (see below).
- `createSaveStore`: versioned save slots with migrations. Saves go to the Velvet Desktop bridge, then localStorage, then memory.

The bridge contract (`VelvetBridge` and `getBridge`) lives in `src/bridge.ts`.

## Usage

    import { ActionMap, GameClock, GameLoop, createSaveStore } from "@velvet/core";

    type Action = "confirm" | "move";

    const input = new ActionMap<Action>([
      { action: "confirm", control: { type: "key", key: "Enter" } },
      { action: "confirm", control: { type: "mouse", button: "left" } },
      { action: "move", control: { type: "key", key: "KeyW" }, axisComponent: "y" },
      { action: "move", control: { type: "key", key: "KeyS" }, axisComponent: "y", scale: -1 },
    ]);
    input.attach(window);

    const clock = new GameClock({ onDay: (day) => console.log("day", day) });
    const saves = createSaveStore({ game: "my-game", version: 1 });

    const loop = new GameLoop({
      update(dt) {
        clock.advance(dt);
        if (input.pressed("confirm")) openMenu(); // openMenu, moveHero and draw are game code
        const { x, y } = input.axis2("move");
        moveHero(x, y, dt);
        input.endFrame(); // once per frame: clears pressed and released, and the wheel and pointer deltas
      },
      render(alpha) {
        draw(alpha);
      },
    });
    loop.start();

    await saves.save("slot-1", { gold: 42 });
    const loaded = await saves.load("slot-1"); // null when the slot is empty

## Keys, controls and handled events

`attach(target)` listens on `window`, or on an element, which then sees only the events inside it. A press
(a keydown, a pointerdown or a wheel tick) does not count when one of these holds:

- its target is a control: an `input`, `textarea`, `select`, `button`, `[contenteditable]` element or
  `[role="menuitem"]`, or something inside one. The control keeps its own keys: Enter on a focused button
  clicks it, and a text field receives its letters. Pass `captureControls: true` in the options to count
  them anyway, for keys that must work while a control has the focus;
- its `defaultPrevented` flag is already set when the map sees it. That happens when a listener on the
  element the event went to called `preventDefault`, because that listener runs before the window's. A
  listener that runs after the map does not count, since the map has read the event by then;
- `markHandled(event)` was called for it. Use it when the handler should take the key without calling
  `preventDefault`.

Releases (keyup, pointerup, pointercancel) and blur always count, so a press that counted is always
released. A map scoped to an element also releases its keys when focus leaves that element, because the
key up would go elsewhere. Pointer moves always update `pointer()`, whatever their target. The UI
components in `@velvet/ui` call `preventDefault` for the keys they use, so a game's map ignores them.

`consume(action)` hides an action for the rest of the frame: `pressed`, `held`, `released` and the axis
values read as idle until `endFrame`. The key state is kept, so a key that is still down is held in the
next frame.

## Tests

From `js/`, run `pnpm test` and `pnpm typecheck`. The tests use a fake frame scheduler,
synthetic input events and in-memory storage, so they need no browser.
