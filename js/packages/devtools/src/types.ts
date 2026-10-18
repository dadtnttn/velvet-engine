// The public types of @velvet/devtools. The function that creates the tools is in devtools.ts.

import type {
  ErrorReport,
  EventEmitter,
  FrameScheduler,
  GameClock,
  GameLoop,
  GameLoopOptions,
  Logger,
} from "@velvet/core";
import type { Stage } from "@velvet/ui";
import type { FrameStats } from "./stats";

/** Anything that lists error reports. The reporter from `installErrorReporter` in @velvet/core does. */
export interface ErrorReportSource {
  /** The reports, most recent first, each with its current count. */
  reports(): readonly ErrorReport[];
}

/** The corners of the window where the pinned watches (the HUD) can be drawn. */
export type HudCorner = "top-left" | "top-right" | "bottom-left" | "bottom-right";

/** Options for {@link createDevtools}. */
export interface DevtoolsOptions<Events extends object = object> {
  /** The element that the overlay is added to. The overlay is fixed to the window, so the layout of the host does not change. */
  readonly host: HTMLElement;
  /** False makes every method a no-op, and adds nothing to the page. */
  readonly enabled: boolean;
  /**
   * The game clock. Pause, resume, step and the speed controls act on it. A clock made later is connected with
   * `attachClock`.
   */
  readonly clock?: GameClock;
  /**
   * The game loop. Its frame hooks give the tools the update and render times, its step length sets the size
   * of a clock-only step, and its running state is shown. Create the loop before the tools, and pass it here.
   */
  readonly loop?: GameLoop;
  /**
   * The event bus whose events go to the event log. Every event of the bus is logged, through a tap. A bus made later
   * is connected with `attachEvents`.
   */
  readonly events?: EventEmitter<Events>;
  /**
   * @deprecated The log records every event of `events`, so there is no need to list them. When names are given,
   * the log keeps only those events; the option still works for that.
   */
  readonly eventNames?: ReadonlyArray<keyof Events & string>;
  /** The game's stage. The performance panel shows its pixel sizes and whether it draws natively. */
  readonly stage?: Stage;
  /** The key that shows or hides the overlay, as a KeyboardEvent.key or .code value. Default "F3". */
  readonly toggleKey?: string;
  /** The speed that slow motion uses. Default 0.25. The clock must allow it. */
  readonly slowSpeed?: number;
  /** Where the frame sampler gets its frames. Default: the browser's animation frames. */
  readonly scheduler?: FrameScheduler;
  /** The clock for frame times, in milliseconds. Default: performance.now. */
  readonly now?: () => number;
  /**
   * The game's logger. The Log tab lists its entries and those of its children, and the `log.level` command sets
   * their levels. Pass the root logger, or any logger of the family.
   */
  readonly logger?: Logger;
  /** The error reporter from `installErrorReporter`. The Log tab shows its reports above the entries. */
  readonly errorReports?: ErrorReportSource;
  /**
   * The corner of the window where the pinned watches are drawn. The panel opens at the top right, so a HUD
   * there is under the panel while the panel is open. Default "top-left". `setHudCorner` moves it later.
   */
  readonly hudCorner?: HudCorner;
  /**
   * False hides the pinned watches entirely. Their values are then not read either, so a watch costs nothing
   * while the HUD is off. Watches can still be added and removed. Default true.
   */
  readonly hud?: boolean;
}

/** Options for {@link Devtools.inspect}. */
export interface InspectOptions {
  /** How often the Auto switch refreshes the view, in milliseconds. Default 500. */
  readonly refreshMs?: number;
}

/** The kinds of argument that a console command takes. */
export type ArgType = "string" | "number" | "boolean";

/** One argument of a console command, in the order the command reads them. */
export interface ArgSpec {
  /** The name shown in usage and in error messages. */
  readonly name: string;
  /** How the word is read: as text, as a number, or as on/off. */
  readonly type: ArgType;
  /** An optional argument may be left out. Default false. */
  readonly optional?: boolean;
  /** The only words allowed for a string argument. The console completes them with Tab. */
  readonly choices?: readonly string[];
}

/** A value that a parsed argument can take. An optional argument that was left out is undefined. */
export type CommandValue = string | number | boolean | undefined;

/** The parsed arguments of a command, by name. */
export type CommandValues = Readonly<Record<string, CommandValue>>;

/** A console command. */
export interface CommandSpec {
  /** The arguments, in order. */
  readonly args?: readonly ArgSpec[];
  /** One line that `help` shows. */
  readonly help?: string;
  /**
   * Runs the command. A returned string is printed. Throwing prints the error message instead, so a
   * command can refuse bad input by throwing.
   */
  readonly run: (values: CommandValues) => string | void;
}

/** Frame and memory numbers, as the Performance tab shows them. */
export interface DevtoolsStats extends FrameStats {
  /** Elements in the document, counted once a second. */
  readonly domNodes: number;
  /** Bytes of JavaScript heap in use, where the browser reports them. Otherwise null. */
  readonly heapBytes: number | null;
}

/** The developer tools. Create them with {@link createDevtools}. */
export interface Devtools {
  /** False when the tools were created disabled. Then every method does nothing. */
  readonly enabled: boolean;
  /** True while the overlay panel is shown. */
  readonly visible: boolean;
  /** Shows the overlay panel. */
  show(): void;
  /** Hides the overlay panel. Focus that was inside it goes back to the page. */
  hide(): void;
  /** Shows the panel when it is hidden, and hides it when it is shown. */
  toggle(): void;

  /**
   * Adds a view to the State tab. `read` runs when the view is refreshed: by its Refresh button, or every
   * `refreshMs` while its Auto switch is on. The result is shown as a tree whose rows open on demand.
   * @returns a function that removes the view.
   */
  inspect(name: string, read: () => unknown, options?: InspectOptions): () => void;

  /**
   * Adds a console command. A name that is already taken is replaced.
   * @throws RangeError when the name is not letters, digits, "-" or "_", or a dotted group of it does not start
   * with a letter. Names such as "log.level" are allowed.
   * @returns a function that removes the command.
   */
  command(name: string, spec: CommandSpec): () => void;

  /**
   * Pins a number to the HUD in the corner, where it stays visible while the overlay is hidden.
   * A name that is already pinned is replaced.
   * @returns a function that unpins it.
   */
  watch(name: string, read: () => number): () => void;

  /**
   * Moves the pinned watches to another corner of the window. The panel stays at the top right.
   * @throws RangeError when the corner is not one of the four.
   */
  setHudCorner(corner: HudCorner): void;

  /**
   * How many error entries the game's log holds, for a watch: `watch("errors", () => devtools.errorCount())`.
   * Errors that the reporter catches are logged at the error level, so they count too. Zero when no logger was given.
   */
  errorCount(): number;

  /**
   * @deprecated Pass the loop to `createDevtools` as `loop`: the update and render times then come from the
   * loop's frame hooks. This still returns the loop options with the hold that pause and step put on the
   * game's update, which the hooks cannot do. Pass the result to `new GameLoop`. Without it, a pause only
   * stops the clock.
   */
  wrapLoop<Options extends GameLoopOptions>(options: Options): Options;

  /** Stops game time, and holds the game's update when the loop is wrapped. */
  pause(): void;
  /** Lets game time run again. */
  resume(): void;
  /** Moves game time on by one loop step, pausing first when the game is running. */
  step(): void;
  /**
   * Sets the clock's speed. Clears slow motion.
   * @throws Error when no clock was given, or RangeError when the clock does not allow the speed.
   */
  setSpeed(speed: number): void;
  /**
   * Turns slow motion on or off. On, the clock runs at the slow speed; off, it returns to the speed it had.
   * @throws Error when no clock was given, or RangeError when the clock does not allow the slow speed.
   */
  setSlowMotion(on: boolean): void;

  /**
   * Connects a game clock, or disconnects the one that is there when `clock` is null. The Time tab and the speed and
   * slow-motion controls follow the clock that is connected, and say so when there is none. The clock that was there
   * is given back first: game time runs again on it when the tools had stopped it, and a slow motion on it is undone.
   * A pause that the tools hold stays on, so a clock connected while paused is paused too.
   */
  attachClock(clock: GameClock | null): void;

  /**
   * Connects an event bus, or disconnects the one that is there when `bus` is null. The Events tab logs the events of
   * the bus that is connected. `names` keeps only some of them, as the `eventNames` option does. Attaching clears the
   * log, whose entries belonged to the bus before; detaching leaves the tab saying that no bus is connected.
   */
  attachEvents<Bus extends object>(bus: EventEmitter<Bus> | null, names?: ReadonlyArray<keyof Bus & string>): void;

  /** Frame and memory numbers at this moment. */
  stats(): DevtoolsStats;
  /** Clears the long-frame counters and the frame history. */
  resetCounters(): void;

  /** Removes the overlay, the listeners and the sampler. A held clock runs again. Safe to call twice. */
  destroy(): void;
}
