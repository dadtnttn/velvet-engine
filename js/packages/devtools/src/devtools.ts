// The developer tools. One object owns the overlay, the frame sampler, the time controls, the event log, the
// State views, the console and the watches. createDevtools returns the real tools, or, when `enabled` is false,
// an object whose methods do nothing and which touches no DOM and adds no listener.
//
// Keys and clicks. The overlay marks the events it receives with markHandled from @velvet/core, so that no
// ActionMap counts them: typing in the console is not a game key, and a click on a panel is not a game click.
// markHandled leaves the default action alone, so typing and clicking still work. The toggle key is also
// marked, and its default action (the browser's own F3 search) is cancelled.

import {
  createFrameScheduler,
  markHandled,
  type FrameScheduler,
  type GameLoop,
  type GameLoopOptions,
} from "@velvet/core";
import type { Stage } from "@velvet/ui";
import type { InspectView, LoopState, OverlayApi } from "./api";
import {
  ConsoleHistory,
  ConsoleOutput,
  HISTORY_LIMIT,
  OUTPUT_LIMIT,
  completeLine,
  isValidCommandName,
  parseArgs,
  tokenize,
  usageOf,
  type Completion,
  type ConsoleKind,
} from "./console";
import { EventLog } from "./events";
import { Overlay } from "./overlay";
import { FrameSampler, type FrameStats } from "./stats";
import { installStyles } from "./styles";
import { DEFAULT_STEP_SECONDS, TimeControls, type TimeState } from "./time";
import { formatFixed, messageOf } from "./text";
import type {
  CommandSpec,
  CommandValue,
  Devtools,
  DevtoolsOptions,
  DevtoolsStats,
  InspectOptions,
} from "./types";

/** The key that shows and hides the overlay, when the options name none. */
const DEFAULT_TOGGLE_KEY = "F3";

/** The speed that slow motion uses, when the options name none. */
const DEFAULT_SLOW_SPEED = 0.25;

/** The interval of the Auto switch in a State view, when the view names none. */
const DEFAULT_REFRESH_MS = 500;

/** How many events the event log keeps. */
const EVENT_LOG_LIMIT = 200;

/** How often the panel and the watches refresh, in milliseconds. */
const TICK_MS = 100;

/** How often the element count and the heap are read, in milliseconds. */
const MEMORY_MS = 1000;

/** The default clock for frame times: performance.now. */
function defaultNow(): number {
  return performance.now();
}

/** True when the keyboard event names the key, by its key value or its physical code. */
function matchesKey(event: KeyboardEvent, name: string): boolean {
  return event.key === name || event.code === name;
}

/** The JavaScript heap in use, when the browser reports it. Chromium does; other engines do not. */
function readHeapBytes(): number | null {
  const memory: unknown = Reflect.get(performance, "memory");
  if (typeof memory !== "object" || memory === null) return null;
  const used: unknown = Reflect.get(memory, "usedJSHeapSize");
  return typeof used === "number" ? used : null;
}

/**
 * Creates the developer tools for a game.
 *
 * With `enabled: false` nothing is built: no element, no style, no listener, and the methods do nothing.
 * Build the tools only in development, so a release build does not carry them (see the README).
 *
 * @example
 * const devtools = createDevtools({ host, enabled: true, clock, events, eventNames: ["day"] });
 * const loop = new GameLoop(devtools.wrapLoop({ update, render }));
 */
export function createDevtools<Events extends object = object>(options: DevtoolsOptions<Events>): Devtools {
  if (!options.enabled) return createDisabled();
  return new ActiveDevtools(options);
}

/** The tools when they are disabled: every method does nothing, and no DOM is touched. */
function createDisabled(): Devtools {
  const noop = (): void => undefined;
  const unsubscribe = (): void => undefined;
  return {
    enabled: false,
    visible: false,
    show: noop,
    hide: noop,
    toggle: noop,
    inspect: (): (() => void) => unsubscribe,
    command: (): (() => void) => unsubscribe,
    watch: (): (() => void) => unsubscribe,
    wrapLoop<Options extends GameLoopOptions>(options: Options): Options {
      return options;
    },
    pause: noop,
    resume: noop,
    step: noop,
    setSpeed: noop,
    setSlowMotion: noop,
    stats: (): DevtoolsStats => ({
      frames: 0,
      fps: null,
      lastMs: null,
      p50Ms: null,
      p95Ms: null,
      maxMs: null,
      longFrames16: 0,
      longFrames33: 0,
      updateMs: null,
      renderMs: null,
      domNodes: 0,
      heapBytes: null,
    }),
    resetCounters: noop,
    destroy: noop,
  };
}

/** The working tools. Created by {@link createDevtools} when they are enabled. */
class ActiveDevtools<Events extends object> implements Devtools, OverlayApi {
  readonly enabled = true;
  private readonly doc: Document;
  private readonly overlay: Overlay;
  private readonly sampler = new FrameSampler();
  private readonly time: TimeControls;
  private readonly eventLog: EventLog;
  private readonly consoleOutput = new ConsoleOutput(OUTPUT_LIMIT);
  private readonly history = new ConsoleHistory(HISTORY_LIMIT);
  private readonly viewMap = new Map<string, InspectView>();
  private readonly commandMap = new Map<string, CommandSpec>();
  private readonly watchMap = new Map<string, () => number>();
  private readonly cleanups: Array<() => void> = [];
  private readonly scheduler: FrameScheduler;
  private readonly readClock: () => number;
  private readonly toggleKey: string;
  private readonly loop: GameLoop | undefined;
  /** True when the loop was given to the tools, so that its frame hook reports the update and render times. */
  private readonly timingFromHooks: boolean;
  private readonly stage: Stage | undefined;
  private frameHandle: number | null = null;
  private lastTickMs = Number.NEGATIVE_INFINITY;
  private lastMemoryMs = Number.NEGATIVE_INFINITY;
  private domNodes = 0;
  private heapBytes: number | null = null;
  private updateMsThisFrame = 0;
  private wrapped = false;
  private stopped = false;

  /** The frame callback. Each frame feeds the sampler; the panel refreshes every TICK_MS. */
  private readonly onFrame = (timeMs: number): void => {
    if (this.stopped) return;
    this.frameHandle = this.scheduler.request(this.onFrame);
    this.sampler.onFrame(timeMs);
    if (timeMs - this.lastTickMs < TICK_MS) return;
    this.lastTickMs = timeMs;
    if (timeMs - this.lastMemoryMs >= MEMORY_MS) {
      this.lastMemoryMs = timeMs;
      this.domNodes = this.doc.getElementsByTagName("*").length;
      this.heapBytes = readHeapBytes();
    }
    this.overlay.update();
  };

  /** Builds the overlay, subscribes to the events, and starts sampling frames. */
  constructor(options: DevtoolsOptions<Events>) {
    this.doc = options.host.ownerDocument;
    this.readClock = options.now ?? defaultNow;
    this.scheduler = options.scheduler ?? createFrameScheduler();
    this.toggleKey = options.toggleKey ?? DEFAULT_TOGGLE_KEY;
    this.loop = options.loop;
    this.timingFromHooks = options.loop !== undefined;
    this.stage = options.stage;
    this.time = new TimeControls(
      options.clock ?? null,
      options.slowSpeed ?? DEFAULT_SLOW_SPEED,
      options.loop?.stepSeconds ?? DEFAULT_STEP_SECONDS,
    );
    this.eventLog = new EventLog(EVENT_LOG_LIMIT, this.readClock);

    installStyles(this.doc);
    this.overlay = new Overlay(this.doc, this, options.host, this.toggleKey);
    this.registerBuiltins();
    this.subscribe(options);
    this.listen();
    this.frameHandle = this.scheduler.request(this.onFrame);
  }

  /** True while the overlay panel is shown. */
  get visible(): boolean {
    return this.overlay.visible;
  }

  show(): void {
    this.overlay.setVisible(true);
  }

  hide(): void {
    this.overlay.setVisible(false);
  }

  toggle(): void {
    this.overlay.setVisible(!this.overlay.visible);
  }

  inspect(name: string, read: () => unknown, options: InspectOptions = {}): () => void {
    const view: InspectView = { name, read, refreshMs: options.refreshMs ?? DEFAULT_REFRESH_MS };
    this.viewMap.set(name, view);
    return () => {
      if (this.viewMap.get(name) === view) this.viewMap.delete(name);
    };
  }

  command(name: string, spec: CommandSpec): () => void {
    if (!isValidCommandName(name)) {
      throw new RangeError(`command name must be letters, digits, "-" or "_", starting with a letter: "${name}"`);
    }
    for (const arg of spec.args ?? []) {
      if (arg.choices !== undefined && arg.type !== "string") {
        throw new RangeError(`argument "${arg.name}" of "${name}" has choices, so its type must be string`);
      }
    }
    this.commandMap.set(name, spec);
    return () => {
      if (this.commandMap.get(name) === spec) this.commandMap.delete(name);
    };
  }

  watch(name: string, read: () => number): () => void {
    this.watchMap.set(name, read);
    return () => {
      if (this.watchMap.get(name) === read) this.watchMap.delete(name);
    };
  }

  /**
   * @deprecated Pass the loop to `createDevtools` as `loop`. The update and render times then come from the
   * loop's frame hooks, and the tools need no wrapper for them. This still returns the loop options with the
   * hold on the game's update while the tools are paused (which the hooks cannot do), and one run of that
   * update per step. When the tools were not given the loop, it also measures the update and render times.
   * The options' own update and render run unchanged otherwise.
   */
  wrapLoop<Options extends GameLoopOptions>(options: Options): Options {
    this.wrapped = true;
    this.time.markWrapped();
    return {
      ...options,
      update: (dt: number): void => {
        if (this.stopped) {
          options.update(dt);
          return;
        }
        const startMs = this.readClock();
        const ran = this.time.runUpdate(() => {
          options.update(dt);
        });
        if (ran && !this.timingFromHooks) this.updateMsThisFrame += this.readClock() - startMs;
      },
      render: (alpha: number): void => {
        if (this.timingFromHooks) {
          options.render?.(alpha);
          return;
        }
        const startMs = this.readClock();
        options.render?.(alpha);
        if (!this.stopped) this.sampler.onLoopFrame(this.updateMsThisFrame, this.readClock() - startMs);
        this.updateMsThisFrame = 0;
      },
    };
  }

  pause(): void {
    this.time.pause();
  }

  resume(): void {
    this.time.resume();
  }

  step(): void {
    this.time.step();
  }

  setSpeed(speed: number): void {
    this.time.setSpeed(speed);
  }

  setSlowMotion(on: boolean): void {
    this.time.setSlowMotion(on);
  }

  stats(): DevtoolsStats {
    return { ...this.sampler.stats(), domNodes: this.domNodes, heapBytes: this.heapBytes };
  }

  resetCounters(): void {
    this.sampler.reset();
    this.updateMsThisFrame = 0;
  }

  destroy(): void {
    if (this.stopped) return;
    this.stopped = true;
    if (this.frameHandle !== null) this.scheduler.cancel(this.frameHandle);
    this.frameHandle = null;
    this.time.dispose();
    for (const cleanup of this.cleanups.splice(0)) cleanup();
    this.overlay.destroy();
    this.viewMap.clear();
    this.commandMap.clear();
    this.watchMap.clear();
  }

  // The overlay's view of the devtools.

  now(): number {
    return this.readClock();
  }

  frameHistory(): readonly number[] {
    return this.sampler.recentIntervals();
  }

  loopState(): LoopState {
    return {
      timed: this.timingFromHooks || this.wrapped,
      held: this.wrapped,
      running: this.loop === undefined ? null : this.loop.running,
    };
  }

  stageText(): string {
    const stage = this.stage;
    if (stage === undefined) return "no stage given";
    const layout = stage.layout;
    const drawing = stage.native ? "native" : "scaled by the compositor";
    return `${layout.width} × ${layout.height} logical · ${layout.pixelWidth} × ${layout.pixelHeight} pixels · ${drawing}`;
  }

  timeState(): TimeState {
    return this.time.state();
  }

  events(): EventLog {
    return this.eventLog;
  }

  views(): ReadonlyMap<string, InspectView> {
    return this.viewMap;
  }

  watches(): ReadonlyMap<string, () => number> {
    return this.watchMap;
  }

  output(): ConsoleOutput {
    return this.consoleOutput;
  }

  print(kind: ConsoleKind, text: string): void {
    this.consoleOutput.add(kind, text);
  }

  /** Runs one console line: echoes it, runs its command, and prints the result or the error. */
  runLine(line: string): void {
    this.history.add(line);
    const text = line.trim();
    if (text === "") return;
    this.consoleOutput.add("input", `> ${text}`);
    try {
      const [name, ...words] = tokenize(text);
      if (name === undefined) return;
      const spec = this.commandMap.get(name);
      if (spec === undefined) throw new Error(`unknown command "${name}": type help to list them`);
      const result = spec.run(parseArgs(spec.args ?? [], words));
      if (typeof result === "string" && result !== "") this.consoleOutput.add("output", result);
    } catch (error) {
      this.consoleOutput.add("error", messageOf(error));
    }
  }

  complete(line: string): Completion {
    return completeLine(this.commandMap, line);
  }

  historyPrevious(draft: string): string | null {
    return this.history.previous(draft);
  }

  historyNext(): string | null {
    return this.history.next();
  }

  historyReset(): void {
    this.history.reset();
  }

  // Internals.

  /**
   * Connects the tools to the game: the event log taps the bus, and the loop's frame hook gives the update and
   * render times. Every event of the bus is logged; when the options name events, only those are.
   */
  private subscribe(options: DevtoolsOptions<Events>): void {
    const bus = options.events;
    if (bus !== undefined) {
      const only = options.eventNames === undefined ? null : new Set<string>(options.eventNames);
      this.cleanups.push(
        bus.tap((event) => {
          const name = String(event.type);
          if (only === null || only.has(name)) this.eventLog.record(name, event.payload);
        }),
      );
    }
    const loop = options.loop;
    if (loop !== undefined) {
      this.cleanups.push(
        loop.on("afterFrame", (timing) => {
          this.sampler.onLoopFrame(timing.updateMs, timing.renderMs);
        }),
      );
    }
  }

  /**
   * Listens for the toggle key on the window, in the capture phase, so that it works before any game
   * listener. The panel and the root mark their keys, pointer presses and wheel turns as handled, so the
   * game's ActionMap never counts them.
   */
  private listen(): void {
    const view = this.doc.defaultView;
    if (view !== null) {
      const onKey = (event: KeyboardEvent): void => {
        this.onWindowKey(event);
      };
      view.addEventListener("keydown", onKey, true);
      this.cleanups.push(() => view.removeEventListener("keydown", onKey, true));
    }

    const root = this.overlay.root;
    const claim = (event: Event): void => {
      markHandled(event);
    };
    for (const type of ["keydown", "pointerdown", "wheel"] as const) {
      root.addEventListener(type, claim, { passive: true });
      this.cleanups.push(() => root.removeEventListener(type, claim));
    }
  }

  /** The toggle key shows or hides the panel. Its own default action is cancelled, and no game sees it. */
  private onWindowKey(event: KeyboardEvent): void {
    if (event.repeat || !matchesKey(event, this.toggleKey)) return;
    event.preventDefault();
    markHandled(event);
    this.toggle();
  }

  /** The commands that every devtools has. A game can replace any of them with `command`. */
  private registerBuiltins(): void {
    this.commandMap.set("help", {
      args: [{ name: "name", type: "string", optional: true }],
      help: "lists the commands, or shows the usage of one",
      run: (values) => this.helpText(values["name"]),
    });
    this.commandMap.set("pause", {
      help: "stops game time",
      run: () => {
        this.pause();
        return "paused";
      },
    });
    this.commandMap.set("resume", {
      help: "lets game time run again",
      run: () => {
        this.resume();
        return "running";
      },
    });
    this.commandMap.set("step", {
      help: "moves game time on by one step, pausing first",
      run: () => {
        const state = this.time.state();
        if (!state.hasClock && !this.wrapped) return "nothing to step: pass a clock, or wrap the loop";
        this.step();
        return "stepped one frame";
      },
    });
    this.commandMap.set("speed", {
      args: [{ name: "value", type: "number", optional: true }],
      help: "shows the speed, or sets it to one that the clock allows",
      run: (values) => {
        const value = values["value"];
        if (typeof value !== "number") return this.speedText();
        this.setSpeed(value);
        return `speed x${value}`;
      },
    });
    this.commandMap.set("clear", {
      help: "clears the console output",
      run: () => {
        this.consoleOutput.clear();
      },
    });
    this.commandMap.set("fps", {
      help: "frame rate, frame times and long frames",
      run: () => this.fpsText(this.stats()),
    });
  }

  /** Help for all commands, or for one. */
  private helpText(name: CommandValue): string {
    if (typeof name === "string") {
      const spec = this.commandMap.get(name);
      if (spec === undefined) throw new Error(`unknown command "${name}"`);
      return [usageOf(name, spec.args ?? []), spec.help ?? ""].filter((part) => part !== "").join("\n");
    }
    return [...this.commandMap.entries()]
      .sort(([a], [b]) => a.localeCompare(b))
      .map(([command, spec]) => {
        const help = spec.help === undefined ? "" : `  ${spec.help}`;
        return `${usageOf(command, spec.args ?? [])}${help}`;
      })
      .join("\n");
  }

  /** The speed now, and the speeds that the clock allows. */
  private speedText(): string {
    const state = this.time.state();
    if (!state.hasClock) return "no game clock was given";
    return `speed x${state.speed ?? 1}, allowed: ${state.speeds.join(", ")}`;
  }

  /** The one-line frame summary of the fps command. */
  private fpsText(stats: FrameStats): string {
    return [
      `fps ${formatFixed(stats.fps, 1)}`,
      `frame p50 ${formatFixed(stats.p50Ms, 1)} ms`,
      `p95 ${formatFixed(stats.p95Ms, 1)} ms`,
      `max ${formatFixed(stats.maxMs, 1)} ms`,
      `over 16.7 ms: ${stats.longFrames16}`,
      `over 33 ms: ${stats.longFrames33}`,
    ].join(" · ");
  }
}
