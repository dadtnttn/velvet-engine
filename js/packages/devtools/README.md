# @velvet/devtools

Developer tools for a Velvet JS game, for development builds only. They draw an overlay over the game
(F3 shows and hides it) with seven parts:

- **Performance**: a graph of the last frames, FPS, median, 95th percentile and maximum frame times, the
  count of long frames (over 16.7 ms and over 33 ms), the update and render split of the loop, the number of
  DOM elements and the JavaScript heap where the browser reports it.
- **Time**: pause, resume, step one frame, the speeds that the game clock allows, and slow motion.
- **Events**: the events of a `@velvet/core` event bus, filtered by name, pausable and capped.
- **Log**: the entries of the game's `@velvet/core` logger, newest first, with a level filter and a text filter.
  The error reports of the reporter come first, each opening to its full text, with a button that copies it.
  See "Log" below.
- **State**: values that the game registers with `inspect`, shown as trees that open on demand, with refresh,
  auto-refresh and copy as JSON.
- **Console**: typed commands with history, Tab completion and built-in commands (`help`, `pause`, `resume`,
  `step`, `speed`, `clear`, `fps`, `log.level`).
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

    const loop = new GameLoop({
      update(dt: number) {
        clock.advance(dt);
        // ...game logic; read input with the ActionMap and call input.endFrame() here
      },
      render(alpha: number) {
        // ...draw
      },
    });

    // Dev builds only: see the next section. The loop exists first, so that the tools can read its frame hooks.
    let devtools: Devtools | undefined;
    if (import.meta.env.DEV) {
      const { createDevtools } = await import("@velvet/devtools");
      devtools = createDevtools({
        host,
        enabled: true,
        clock,
        events,
        loop,
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

    loop.start();

    // The game's events reach the log by themselves: every event of `events` is logged. The loop needs no wrapper.
    const loop = new GameLoop(devtools ? devtools.wrapLoop(options) : options);
    loop.start();

`import.meta.env.DEV` is typed by Vite's client types (`/// <reference types="vite/client" />`). The game lists
`"@velvet/devtools": "workspace:*"` among its dependencies, like the other packages. The loop is created before
the tools, and passed to them as `loop`: its frame hooks give the update and render split. The loop's options
go in unchanged; `wrapLoop` is no longer needed for that (see "Deprecated" below for the one thing it still does).

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
- `devtools.wrapLoop(options)` (deprecated, see below) also holds the game's `update` while the tools have
  paused the game, so logic that does not read the clock stops too. A step runs `update` once, with the clock
  running for that call. While the update is held, input edges still build up and are delivered on resume or
  on the next step, because the game's `input.endFrame()` runs in that update.
- Without `wrapLoop`, a pause only stops the clock, and a step moves the clock on by one loop step. The frame
  hooks cannot hold an update, so this is the one difference that the loop option does not remove.
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
- The update and render split comes from the loop's `afterFrame` hook when the tools are given the `loop`. It
  also comes from `wrapLoop` when the tools were not given the loop. Without either, the Performance tab says
  the split is not measured.
- `devtools.stats()` returns the same numbers, and `devtools.resetCounters()` clears the long-frame counts.

## Event log

The tools tap the `events` bus, so every event that the game emits is logged, with its name and payload on one
line. The log keeps the newest 200 events, whether or not the panel is open. Pausing the log counts the events
that arrive meanwhile as dropped. The log can be narrowed with `eventNames` (deprecated, see below).

## Deprecated

- **`wrapLoop`** still works. It holds the game's update while the tools are paused, which the loop's frame
  hooks cannot do, so a game that uses that hold keeps calling it. Its update and render times are no longer
  needed when the loop is given to the tools, and it measures them only when it is used without the loop.
- **`eventNames`** still works as a filter. The log records every event of the bus without it.

## Frame hooks

The tools read the loop through `loop.on("afterFrame", ...)`, which is one of the five hooks that `GameLoop`
offers (`beforeFrame`, `beforeUpdate`, `afterUpdate`, `afterRender` and `afterFrame`; see `@velvet/core`).
A game can use the same hooks for its own timing, without wrapping the loop.

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
  replaced. The name must start with a letter and use only letters, digits, `-` and `_`. Dots may group the
  parts of a name, each starting with a letter, as the built-in `log.level` does.
- Built-in commands: `help [name]`, `pause`, `resume`, `step`, `speed [value]` (shows the speed when no value
  is given), `clear`, `fps`, and `log.level <name> <level>`, which sets the level of a logger of the game's
  family by its full name (`game`, `game.combat`) to `debug`, `info`, `warn` or `error`.
- In the prompt: Enter runs the line, Up and Down browse the history (the line being typed comes back after
  the newest), Tab completes a command name or a choice, and Escape leaves the prompt.

## Log

    import { createConsoleSink, createLogger, installErrorReporter } from "@velvet/core";

    const logger = createLogger({ name: "game", sinks: [createConsoleSink()] });
    const reporter = installErrorReporter({ logger, onReport: (report) => showErrorOverlay(report) });
    const devtools = createDevtools({ host, enabled: true, logger, errorReports: reporter });
    devtools.watch("errors", () => devtools.errorCount());

- The Log tab lists the entries of the logger and of its children (the whole family), newest first, up to 200 of
  them. The level filter keeps the entries at that level and above, and the text filter matches the logger's
  name, the message or the fields, ignoring case. The summary counts the entries shown and the errors in the log.
- The error reports of `errorReports` (any object with `reports()`, such as the reporter from `installErrorReporter`)
  are listed above the entries, the newest five. Each report opens to the text of `formatReport`, and its button
  copies that text. A report that is open stays open when another arrives.
- `log.level <name> <level>` sets a logger's level from the console, and the Log tab shows the new level.
- `errorCount()` counts the error entries that the logger holds, so a watch can show them. The reporter logs every
  error it catches at the error level, so those count too.
- Without a `logger` option the tab says that no logger was given, and `errorCount()` is zero.

## Watches

    const unpin = devtools.watch("enemies", () => world.enemies.length);

A watch shows its number on the HUD, and the HUD stays on while the panel is hidden. The value is read about
every 100 ms. A read that throws shows its error in place of the number. The HUD is not interactive, so it
never takes clicks from the game.

The HUD sits in the top-left corner by default. Choose another with `hudCorner: "top-right"` (or `"bottom-left"`,
`"bottom-right"`), and move it later with `devtools.setHudCorner(corner)`; a corner that is not one of the four
throws a RangeError. The panel opens at the top right, so a HUD there is under the panel while the panel is
open. Pass `hud: false` to hide the HUD entirely: then no watch value is read, and the watches can still be
added and removed. A game that draws its own HUD in a corner can use these to keep the two apart.

## Tests

From `js/`, `pnpm test` runs every package's tests, this package's included, with Vitest and happy-dom. The
tests of this package drive their frames through a scheduler of their own, so they need no animation frames.
