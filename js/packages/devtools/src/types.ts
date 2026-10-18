// The public types of @velvet/devtools. The function that creates the tools is in devtools.ts.

import type { EventEmitter, FrameScheduler, GameClock, GameLoop, GameLoopOptions } from "@velvet/core";
import type { Stage } from "@velvet/ui";
import type { FrameStats } from "./stats";

/** Options for {@link createDevtools}. */
export interface DevtoolsOptions<Events extends object = object> {
  /** The element that the overlay is added to. The overlay is fixed to the window, so the layout of the host does not change. */
  readonly host: HTMLElement;
  /** False makes every method a no-op, and adds nothing to the page. */
  readonly enabled: boolean;
  /** The game clock. Pause, resume, step and the speed controls act on it. */
  readonly clock?: GameClock;
  /** The game loop. Its step length sets the size of a clock-only step, and its running state is shown. */
  readonly loop?: GameLoop;
  /** The event bus whose events go to the event log. */
  readonly events?: EventEmitter<Events>;
  /** The events to log. A bus cannot list its names, so the game lists the ones it wants to see. */
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
   * @throws RangeError when the name is not letters, digits, "-" or "_", starting with a letter.
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
   * Returns the loop options with the devtools applied: frame timing, the update and render split, and
   * the hold that pause and step put on the game's update. Pass the result to `new GameLoop`.
   * Without this, the loop still runs, but the update and render split is not measured and a pause only
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

  /** Frame and memory numbers at this moment. */
  stats(): DevtoolsStats;
  /** Clears the long-frame counters and the frame history. */
  resetCounters(): void;

  /** Removes the overlay, the listeners and the sampler. A held clock runs again. Safe to call twice. */
  destroy(): void;
}
