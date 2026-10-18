# @velvet/devtools

Developer tools for a Velvet JS game, for development builds only. They draw an overlay over the game
(F3 shows and hides it) with six parts:

- **Performance**: a graph of the last frames, FPS, median, 95th percentile and maximum frame times, the
  count of long frames (over 16.7 ms and over 33 ms), the update and render split of the loop, the number of
  DOM elements and the JavaScript heap where the browser reports it.
- **Time**: pause, resume, step one frame, the speeds that the game clock allows, and slow motion.
- **Events**: the events of a `@velvet/core` event bus, filtered by name, pausable and capped.
- **State**: values that the game registers with `inspect`, shown as trees that open on demand, with refresh,
  auto-refresh and copy as JSON.
- **Console**: typed commands with history, Tab completion and built-in commands (`help`, `pause`, `resume`,
  `step`, `speed`, `clear`, `fps`).
- **Watches**: numbers pinned to a small HUD in the corner, which stays on screen while the panel is hidden.

The tools are plain DOM and CSS. Their stylesheet, scoped to `vdt-` classes, is added to the page once. The
overlay is fixed to the window, so it never changes the layout of the game's host element. Its only
dependencies are `@velvet/core` (the loop, the clock, the event bus and the input helpers) and the `Stage`
type from `@velvet/ui`.

## Wiring into a game's main

    import { EventEmitter, GameClock, GameLoop } from "@velvet/core";
    import { createStage } from "@velvet/ui";
    import type { Devtools } from "@velvet/devtools";

    interface GameEvents {
      day: { day: number };
      save: void;
    }

    const host = document.getElementById("game")!;
    const stage = createStage(host);
    const events = new EventEmitter<GameEvents>();

    // The speeds include x0.25 and x4, so the time controls cover the whole range.
    const clock = new GameClock({
      speeds: [0.25, 0.5, 1, 2, 4],
      onDay: (day) => events.emit("day", { day }),
    });

    // Dev builds only: see the next section.
    let devtools: Devtools | undefined;
    if (import.meta.env.DEV) {
      const { createDevtools } = await import("@velvet/devtools");
      devtools = createDevtools({
        host,
        enabled: true,
        clock,
        events,
        eventNames: ["day", "save"],
        stage,
      });
      devtools.watch("fps", () => devtools?.stats().fps ?? 0);
      devtools.inspect("game", () => ({ day: clock.day, elapsed: clock.elapsed }));
      devtools.command("say", {
        args: [{ name: "text", type: "string" }],
        help: "prints a line of text",
        run: ({ text }) => String(text),
      });
    }

    const options = {
      update(dt: number) {
        clock.advance(dt);
        // ...game logic; read input with the ActionMap and call input.endFrame() here
      },
      render(alpha: number) {
        // ...draw
      },
    };
    // wrapLoop is the only change to the loop: without the tools, the options go in unchanged.
    const loop = new GameLoop(devtools ? devtools.wrapLoop(options) : options);
    loop.start();

`import.meta.env.DEV` is typed by Vite's client types (`/// <reference types="vite/client" />`). The game lists
`"@velvet/devtools": "workspace:*"` among its dependencies, like the other packages. The tools are created
before the loop, because `wrapLoop` is how the loop gets the update and render split and the pause hold.

## Dev builds only

Build the tools only when `import.meta.env.DEV` is true, as above. Vite replaces that flag with `false` in a
production build, so the branch and the dynamic import are removed, and the package is not in the bundle.
`enabled: false` is the second guard: every method does nothing, `wrapLoop` returns the options unchanged,
and no element, style or listener is created. The package is marked `sideEffects: false`, so importing it
alone adds nothing either.

## Keys and focus

- **F3** (or the `toggleKey` option, as a `KeyboardEvent.key` or `.code` value) shows and hides the panel. The
  tools take that key: the game does not see it, and the browser's own F3 action is cancelled.
- **The console and the panel keep their keys to themselves.** Every key pressed in the panel, every
  pointer press and every wheel turn on it is marked with core's `markHandled`, so no `ActionMap` counts
  it, even a map made with `captureControls: true`. `markHandled` does not call `preventDefault`, so typing,
  clicking and scrolling still work in the panel. Pointer moves still update `ActionMap.pointer()`.
- Escape leaves the console. Hiding the panel moves the focus out of it.
- The overlay lets clicks through everywhere except on its panel.

## Time controls and the clock

- Pause, resume, step and the speeds act on the core `GameClock`. While the clock is paused, game time stands
  still: logic that reads `clock.elapsed` stops with it.
- `devtools.wrapLoop(options)` also holds the game's `update` while the tools have paused the game, so logic
  that does not read the clock stops too. A step runs `update` once, with the clock running for that call.
  While the update is held, input edges still build up and are delivered on resume or on the next step,
  because the game's `input.endFrame()` runs in that update.
- Without `wrapLoop`, a pause only stops the clock, and a step moves the clock on by one loop step.
- The clock allows only the speeds it was created with, so create it with `speeds: [0.25, 0.5, 1, 2, 4]` for
  the full range. Slow motion runs the clock at `slowSpeed` (default 0.25, which the clock must allow) and
  gives back the previous speed when turned off. Setting a speed by hand turns slow motion off.
- `destroy()` lets go of a clock that the tools paused, and undoes slow motion.

## Performance numbers

- Frame times come from the tools' own frame callback, one per animation frame. The graph and the numbers
  refresh about every 100 ms, and only while the Performance tab is open. The DOM count and the heap are read
  once a second.
- The median and the 95th percentile use the nearest-rank method over the last 240 frames. The FPS is the
  mean interval of the last 60 frames.
- A gap of more than a second, such as a hidden tab, is not counted as a frame.
- The update and render split exists only when the loop goes through `wrapLoop`, because the core loop does
  not report its timing. The Performance tab says so when the split is missing.
- `devtools.stats()` returns the same numbers, and `devtools.resetCounters()` clears the long-frame counts.

## Event log

Core's `EventEmitter` does not list its event names, so the game names the events it wants logged, with
`events` and `eventNames`. The log keeps the newest 200 events, whether or not the panel is open. Its payload
is shown on one line. Pausing the log counts the events that arrive meanwhile as dropped.

## State inspector

    const stop = devtools.inspect("inventory", () => inventory, { refreshMs: 250 });
    stop(); // removes the view

- `read` runs when the view is refreshed: by its Refresh button, or every `refreshMs` (default 500) when its
  Auto switch is on. A read that throws keeps the last value and shows the error.
- A tree opens one row at a time. The properties of a closed object are not read, so its getters are never
  called. A getter in an open object shows as "(getter, not read)".
- Arrays and collections show 100 items at a time, with a button for the next 100. Cycles show as
  `[Circular]`. The depth stops at 8. A value that cannot be read becomes an error row, and never breaks the
  game.
- Copy JSON writes the last value read, with cycles, functions and unreadable properties marked.

## Console

    const stop = devtools.command("gold", {
      args: [{ name: "amount", type: "number" }, { name: "note", type: "string", optional: true }],
      help: "adds gold",
      run: ({ amount, note }) => {
        gold += Number(amount);
        return `gold ${gold}${note === undefined ? "" : ` (${note})`}`;
      },
    });

- Words are split on spaces; double quotes keep several words together (`note "bought a sword"`).
- Argument types are `string`, `number` and `boolean` (`on`, `off`, `yes`, `no`, `true`, `false`, `1` and `0`).
  A word of the wrong type, a missing required argument or an extra word prints an error, and the command
  does not run. A string argument with `choices` completes with Tab, and only those choices are accepted.
- `run` may return text, which is printed. Throwing prints the message. A name that is already taken is
  replaced. The name must start with a letter and use only letters, digits, `-` and `_`.
- Built-in commands: `help [name]`, `pause`, `resume`, `step`, `speed [value]` (shows the speed when no value
  is given), `clear` and `fps`.
- In the prompt: Enter runs the line, Up and Down browse the history (the line being typed comes back after
  the newest), Tab completes a command name or a choice, and Escape leaves the prompt.

## Watches

    const unpin = devtools.watch("enemies", () => world.enemies.length);

A watch shows its number on the HUD at the top-left, and the HUD stays on while the panel is hidden. The
value is read about every 100 ms. A read that throws shows its error in place of the number. The HUD is not
interactive, so it never takes clicks from the game.

## Tests

From `js/`, `pnpm test` runs every package's tests, this package's included, with Vitest and happy-dom. The
tests of this package drive their frames through a scheduler of their own, so they need no animation frames.
