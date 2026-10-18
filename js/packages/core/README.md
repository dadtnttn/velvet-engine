# @velvet/core

The core runtime for Velvet JS games. It has these parts:

- `GameLoop`: a fixed-step loop. `update(dt)` runs at 60 Hz by default, and `render(alpha)` runs once per frame. Five frame hooks (`loop.on`) run around each frame, with its timings (see "Frame hooks").
- `GameClock`: game time fed by the loop, with pause, speed multipliers (1, 2, 3 by default) and days. It sends `paused`, `resumed`, `speed` and `stepped` events, and `state()` gives a snapshot.
- `EventEmitter`: typed events with `on`, `once`, `off` and `emit`, and two tools for the whole bus: `tap` and `names`.
- `ActionMap`: keys, mouse buttons, the wheel and gamepads bound to named actions, read with `pressed`, `held` and `released`, and with analog values from `value`. It can be scoped to an element, it leaves a control's own keys to the control, `consume` hides an action for the rest of a frame, and its slots can be rebound at run time (see below).
- `createSaveStore`: versioned save slots with migrations. Saves go to the Velvet Desktop bridge, then localStorage, then memory.
- `createSettingsStore`: a few typed settings with defaults, for one namespace, saved through the bridge, then localStorage, then memory, with debounced writes and migrations (see "Settings").

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

## Frame hooks

`loop.on(phase, listener)` runs code at five points of each frame, without wrapping the loop's options. It returns the function that removes the listener.

| Phase | When it runs | Payload |
|---|---|---|
| `beforeFrame` | once, before the steps | `{ frame, timeMs, realDt }` |
| `beforeUpdate` | before each fixed step | `{ frame, step, dt }` |
| `afterUpdate` | after each fixed step | `{ frame, step, dt, durationMs }` |
| `afterRender` | after `render`, which also runs when there is no render function (then its duration is zero) | `{ frame, alpha, durationMs }` |
| `afterFrame` | last | `{ frame, timeMs, realDt, scaledDt, steps, updateMs, renderMs, frameMs }` |

The order is fixed: `beforeFrame`, then for each step `beforeUpdate`, `update`, `afterUpdate`, then `render`, `afterRender` and `afterFrame`. A frame that runs no step skips the step phases. The times are in milliseconds, and they come from the loop's `now` option (default `performance.now`), so a test can pass its own clock. `realDt` is the real time since the previous frame, zero for the first one. `scaledDt` is the game-step time that the frame ran: the number of steps times the step length.

**Input.** The loop never calls `input.endFrame()`: the game does, and the hooks see input as it stands at their point of the frame. Edges stay until the game clears them. In the example above, `endFrame()` is the last call of `update`, so `afterUpdate`, `afterRender` and `afterFrame` see the edges already cleared, while `beforeFrame` and `beforeUpdate` see the edges that the game has not yet cleared.

**Cost and errors.** While no hook listens, a frame reads no clock and builds no payload. A listener added during a frame runs from the next phase that the frame reaches, and a frame that began with no listener runs none of them. A `stop()` during a frame ends its remaining steps and its render, and `afterFrame` still runs once for that frame. An error thrown by a listener stops the frame, as an error thrown by `update` does; the next frame is already scheduled.

## Clock events

`clock.on`, `once` and `off` take these events:

- `paused` and `resumed`, sent when the pause changes, with the state after the change. Pausing a paused clock sends nothing.
- `speed`, with `{ previous, current }`, when the speed changes: by `setSpeed`, by `cycleSpeed`, or by a `restore` that changes it.
- `stepped`, with `{ dt, gameSeconds }`, after each `advance` of a running clock, once any day that began has been counted.

`restore` sends the changes that it makes, after it has set the clock, so a listener is right after a load. `clock.state()` gives what `snapshot()` keeps (elapsed, speed and paused) and also the `day`, the `timeOfDay`, the `dayLength` and the `speeds`, as a copy that changes nothing when it is altered.

## Event tools

- `tap(listener)` receives every event of an emitter as `{ type, payload }`, before the listeners of that event. It is meant for tools such as an event log; a game uses `on`.
- `names()` lists the events that have a listener now, or that were emitted since the emitter was made, in the order they first appeared.
- `listenerCount(type)` tells how many listeners an event has, so that a hot emitter can skip a payload that nobody reads.

All three are opt-in: without a tap, an emit does no extra work for it.

## Settings

    import { createSettingsStore } from "@velvet/core";

    interface Prefs {
      volume: number;
      language: string | null;
    }

    const prefs = createSettingsStore<Prefs>({
      namespace: "my-game",                        // one namespace per package or game area
      version: 1,
      defaults: { volume: 1, language: null },
      // migrate: { 1: (values) => ({ ...values, volume: 1 }) }   // keyed by the version each step starts from
    });
    await prefs.load();                 // the saved values, or the defaults; never rejects
    prefs.get("volume");                // 1 until a value is saved
    const stop = prefs.subscribe("volume", (volume) => applyVolume(volume));
    prefs.set("volume", 0.5);           // subscribers hear at once; the values are saved after 250 ms of quiet
    prefs.reset("volume");              // back to the default; reset() with no key resets every setting
    await prefs.save();                 // writes now: resolves true, or false when the storage refused

**Where the values go.** In Velvet Desktop, the bridge's settings, under the namespace as the key. In a browser, localStorage, under `velvet-settings:` and the namespace. Otherwise memory. Pass `bridge`, `storage` or `backend` to choose, or `null` to skip one. Two stores on one namespace share the saved values, and the last write wins, so each package takes its own namespace.

**What is read.** The saved form is `{ "format": "velvet-settings", "version": N, "values": { ... } }`. A saved value is used only when its type matches its default: a number for a number, an array for an array, an object for an object. A default of `null` takes any value. A value of the wrong type is ignored, so a damaged setting never stops the game from starting. Values of an older version are migrated one step at a time, as save slots are. Values of a newer build are not read. A plain object saved without the envelope counts as version 1.

**What is written.** `set` changes a value at once and saves every value after the debounce, so a burst of changes is one write (`debounceMs` changes the wait). `save` writes at once. A write waits for `load`, so it never replaces stored values that were not read yet. A value that was set before the load finished keeps its value, and the other saved values still load. A storage that refuses a write never throws into the game: `save` resolves false.

`chooseSettingsBackend({ prefix, explicit?, bridge?, storage? })` is the choice that the settings store and the bindings store share: the backend passed in, else the bridge's settings, else localStorage under the prefix, else memory. The bindings file keeps its own keys (`input.bindings` in the bridge, `velvet-input:input.bindings` in localStorage), so saved bindings still load.

## Desktop calls: pictures and links

Two helpers ask Velvet Desktop for things that a browser page cannot do itself. Outside Velvet Desktop they reject with a `BridgeUnavailableError`, whose `code` is `"no-desktop"`. A host from before the call rejects with `"unsupported"`.

    import { captureScreenshot, openExternalLink } from "@velvet/core";

    const png = await captureScreenshot();           // "data:image/png;base64,..." of the game's page
    photo.src = png;
    await openExternalLink("https://example.com");   // https, http or mailto only

- `captureScreenshot()` gives a PNG of the page, for photo modes and bug reports. A picture over 8 MiB is refused with the code `too_large`.
- `openExternalLink(url)` opens the link in the player's browser or mail program. The host refuses other schemes, spaces, quotes and user information before the host name, with the code `invalid_url`.

A failure inside the host keeps the host's own `code`. Both calls work on Windows for now; elsewhere the host answers `unsupported`.

## Tests

From `js/`, run `pnpm test` and `pnpm typecheck`. The tests use a fake frame scheduler, synthetic input events and
in-memory storage, so they need no browser. Gamepads are synthetic too: a stubbed `navigator.getGamepads` returns pads
that a test changes from frame to frame (`src/input-test-support.ts`). The capture and menu-key tests run on happy-dom,
so their events go through the page's own capture and bubbling.
