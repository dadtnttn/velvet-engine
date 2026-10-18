# @velvet/core

The core runtime for Velvet JS games. It has six parts:

- `GameLoop`: a fixed-step loop. `update(dt)` runs at 60 Hz by default, and `render(alpha)` runs once per frame.
- `GameClock`: game time fed by the loop, with pause, speed multipliers (1, 2, 3 by default) and days.
- `EventEmitter`: typed events with `on`, `once`, `off` and `emit`.
- `ActionMap`: keys, mouse buttons, the wheel and gamepads bound to named actions, read with `pressed`, `held` and `released`, and with analog values from `value`. It can be scoped to an element, it leaves a control's own keys to the control, `consume` hides an action for the rest of a frame, and its slots can be rebound at run time (see below).
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

## Gamepads

Gamepads use the standard mapping, so buttons are named `a`, `b`, `x`, `y`, `leftShoulder`, `rightShoulder`,
`leftTrigger`, `rightTrigger`, `select`, `start`, `leftStick`, `rightStick` and `dpadUp` to `dpadRight`. A pad
control is one of three types:

    { type: "pad-button", button: "a" }                      // a button, or a trigger with an analog value
    { type: "pad-axis", axis: "leftX" }                      // an axis, from -1 to 1, with a deadzone
    { type: "pad-stick", stick: "left", direction: "up" }    // a stick pushed one way, read as a button

Each takes an optional `pad` (the index of one pad; by default any connected pad) and `threshold` (the level at
which the control is active; default 0.5). An axis and a stick take `deadzone` too (default 0.2). An axis bound
with `axisComponent` feeds its component at every level. Other bindings are active while their level is at or
above the threshold.

- The pads are read once per frame, in `endFrame()`, so a change shows up in the next frame, as a key event
  does. A map reads the gamepads even when none of its bindings uses one, so device tracking and hot-plug work.
- `value(action)` gives the analog strength of an action from -1 to 1, whether or not the action is active: a
  trigger pulled 40% gives 0.4, and an inverted binding gives a negative value.
- `connectedGamepads()` and `onGamepadChange(listener)` report connections and disconnections. A pad that goes
  away releases its buttons.
- `lastDevice()` is `"keyboard"`, `"mouse"` or `"gamepad"`: the device used last, keyboard at first.
  `onDeviceChange(listener)` reports a change, so a UI can switch its prompts.
- `rumble({ pad, durationMs, strong, weak })` plays a dual-rumble effect. It resolves to `false` when the pad
  cannot vibrate, so a game does not need to check first.

The standard names and the defaults are exported as `PAD_BUTTON_NAMES`, `PAD_AXIS_NAMES`, `DEFAULT_DEADZONE` and
`DEFAULT_PAD_THRESHOLD`. A pad that does not report the standard mapping is read with the standard indexes anyway.

## Rebinding

A binding has a **slot**: a number that names its place in the action's list. Slots keep their numbers when one is
rebound or cleared. `slotsOf(action)` lists them, with `null` for an empty slot.

    const outcome = map.rebind("confirm", 0, { action: "confirm", control: { type: "key", key: "KeyZ" } });
    // outcome.status is "bound", "swapped", "allowed" or "refused"; outcome.conflicts lists the slots in conflict
    map.clearSlot("confirm", 1);
    map.resetAction("confirm");   // back to the bindings that the map was made with
    map.resetAll();

Two bindings **conflict** when they are different actions, in the same `context` (default `"default"`), on the same
control. An axis conflicts only with the same direction. The policy decides what `rebind` does (default `"swap"`):

- `"swap"`: the other action takes the control that the slot held, or loses its control when the slot was empty.
  The other binding keeps its own scale and axis.
- `"refuse"`: nothing changes, and the status is `"refused"`.
- `"allow"`: both actions keep the control.

`conflicts()` lists the groups that still conflict. Contexts keep, for example, the menu keys and the game keys
apart. `resetAction` and `resetAll` restore the defaults without resolving the conflicts they bring back, and
`conflicts()` shows them.

**Capturing** the next input, for a rebinding screen:

    const input = await map.captureNext({ devices: ["keyboard", "gamepad"], timeoutMs: 8000 });
    // { device, control, sign? } for the input, or null when it was cancelled, timed out or aborted

A capture takes a key before the page and the game see it, so the key being bound does not also act. Escape cancels
by default (`cancelKeys` changes that). It takes a mouse button, and it takes a pad button pressed from released, or
a stick pushed from rest, with `sign` giving the direction of the push. A control that is held when the capture
starts counts only after it is released. `cancelCapture()` ends the capture in progress, and a new capture ends the
last one.

**Display data**: `describeBinding(binding)` gives `{ device, code, labelKey }`, and `negative` for an inverted axis.
The `labelKey` is a stable id such as `input.key.Space` or `input.pad.a`. The game supplies their texts; see
`docs/formats/INPUT_BINDINGS.md`.

**For a UI**: `createRebindPort(map, { scheduler?, target? })` gives the operations that `createRebindPanel` in
`@velvet/ui` takes: the slots of an action, a capture into a slot, clearing, and the resets.

**The bindings file**: `createBindingsStore({ key?, settings?, version?, migrations? })` saves a map and loads it back.
It writes to the bridge's settings when the desktop host is present, otherwise to localStorage, otherwise to memory.
`save(map)`, `load(map, knownActions)` (which gives null when nothing is saved) and `clear()` do the work. The file is
versioned like a save: an older one is migrated with `migrations`, and a newer one is refused. The format is in
`docs/formats/INPUT_BINDINGS.md`. `serializeBindings`, `parseBindings` and `applyBindings` are the same steps without
a store.

## Menu keys from a pad

`startPadMenuKeys()` makes the d-pad, the left stick, A and B press the arrows, Enter and Escape of the focused
element. A menu that follows the keyboard focus rules (`@velvet/ui`) then works with a pad too. Start it while a menu
is open, and call the function it returns to stop. The keys are marked handled, so the game's map ignores them.

## Tests

From `js/`, run `pnpm test` and `pnpm typecheck`. The tests use a fake frame scheduler, synthetic input events and
in-memory storage, so they need no browser. Gamepads are synthetic too: a stubbed `navigator.getGamepads` returns pads
that a test changes from frame to frame (`src/input-test-support.ts`). The capture and menu-key tests run on happy-dom,
so their events go through the page's own capture and bubbling.
