// The driver runs a game's loop by hand. It owns the frames, so nothing depends on the real clock: a
// frame runs the loop's callbacks once, with a time that moves by exactly one step, and the same inputs
// give the same states every time.
//
//   const driver = createDriver();
//   const loop = driver.createLoop({ update(dt) { clock.advance(dt); keys.endFrame(); } });
//   keys.attach(driver.target);          // the scripted keys, pointer and wheel reach the map here
//   loop.start();
//   driver.keyDown("Space");
//   driver.until(() => state.coins > 3);
//
// Frames. A frame is one update step. Each frame first applies the scripted input that is due, then calls
// the listeners' `beforeTick`, then runs the loop's callbacks once, then calls `afterTick`. A driven loop
// must therefore run exactly one update per frame: the driver's rate must match the loop's rate, and
// `createLoop` checks that every frame ran one update.
//
// Origins. A GameLoop's first callback after `start()` only sets its time origin and runs no update. A
// callback that is requested from outside a frame (a loop starting between frames) is therefore run once
// before the next counted frame, at the current time, so that the first counted frame is the loop's first
// update. Starting a loop from inside a frame has no such run, so the driver reports the frame where that
// loop runs without an update.
//
// Inputs. Keys, pointer moves, button presses, wheel ticks and blur are sent to `target` as the browser
// would send them, so an ActionMap attached there counts them exactly as it counts real input. Positions
// are logical (see stage.ts). Pads are set on `pads`, which installs a fake `navigator.getGamepads` for
// the driver's lifetime. Every input is also reported to the input listeners, which is how a recorder
// sees the session.
//
// Frame takeover. A game that builds its own GameLoop with the default scheduler still runs on driver
// frames after `takeOverFrames()`, because that scheduler calls the global requestAnimationFrame.

import { GameLoop, type FrameScheduler, type GameLoopOptions, type MouseButtonName, type Unsubscribe } from "@velvet/core";
import type { GameClock } from "@velvet/core";
import { blurEvent, buttonNumber, keyEvent, pointerEvent, wheelEvent } from "./events";
import { TestingError } from "./errors";
import { FakeGamepads } from "./fake-pads";
import type { ReplayEvent } from "./replay-format";
import { createStageMap, logicalToClient, type StageMap, type StagePoint } from "./stage";
import type { InputScript } from "./script";

/** Update steps per second when the options name no rate. */
const DEFAULT_RATE = 60;

/** The fastest rate the driver runs. */
const MAX_RATE = 1000;

/** Frames that `until` runs before it gives up: one minute at 60 Hz. */
const DEFAULT_UNTIL_FRAMES = 3600;

/** Frames of slack that `advanceGame` allows on top of the frames that the game time needs. */
const CLOCK_SLACK_FRAMES = 600;

/** Game seconds that may be short of their target because of floating-point error. */
const TIME_EPSILON = 1e-9;

/** Options for {@link createDriver}. */
export interface DriverOptions {
  /** Update steps per second. It must equal the rate of the loops that run on the driver. Default 60. */
  readonly rate?: number;
  /**
   * Where scripted keys, pointer events and wheel events are sent. Default: `window` where there is one,
   * otherwise a new EventTarget. The game's action maps must be attached to this same target.
   */
  readonly target?: EventTarget;
  /** How the stage is placed in the window. Default: 1920 by 1080 at the window's origin. */
  readonly stage?: StageMap;
  /** The game's clock. It is needed by {@link Driver.advanceGame}. */
  readonly clock?: GameClock;
}

/** Options for the input methods. `skipped` marks a press that the page has already used, so no map counts it. */
export interface InputOptions {
  readonly skipped?: boolean;
}

/** Options for {@link Driver.wheel}. */
export interface WheelOptions {
  /** Horizontal movement. Default 0. */
  readonly deltaX?: number;
  /** The delta mode: 0 for pixels, 1 for lines, 2 for pages. Default 0. */
  readonly deltaMode?: 0 | 1 | 2;
  readonly skipped?: boolean;
}

/** Options for {@link Driver.until}. */
export interface UntilOptions {
  /** The most frames to run before giving up. Default 3600. */
  readonly maxFrames?: number;
}

/** Options for {@link Driver.advanceGame}. */
export interface AdvanceGameOptions {
  /** The most frames to run before giving up. Default: the frames that the game time needs at speed 1, plus 600. */
  readonly maxFrames?: number;
}

/** Callbacks around each frame. Both receive the index of the update: the first update is 0. */
export interface DriverListener {
  /** Runs after the frame's scripted input and before its callbacks run. */
  beforeTick?(frame: number): void;
  /** Runs after the frame's callbacks, that is after its update and render. */
  afterTick?(frame: number): void;
}

/** A script that is playing on a driver. */
export interface ScriptRun {
  /** True once every action has run, or the run was cancelled. */
  readonly done: boolean;
  /** Drops the actions that have not run yet. */
  cancel(): void;
}

/** One action that waits for its frame. */
interface ScheduledAction {
  readonly frame: number;
  readonly order: number;
  readonly run: () => void;
}

/** A callback that waits for a frame. `origin` is set when it was requested from outside a frame. */
interface PendingFrame {
  readonly callback: (timeMs: number) => void;
  readonly origin: boolean;
}

/** A loop that the driver made, with the updates that it ran in the current frame. */
interface CountedLoop {
  /** The loop itself. It is set as soon as the loop is built, which is before any frame runs. */
  loop: GameLoop | null;
  updatesInFrame: number;
}

/** The driver. Create it with {@link createDriver}, and dispose it when the run ends. */
export class Driver {
  /** Update steps per second. */
  readonly rate: number;
  /** The length of one step in seconds. */
  readonly stepSeconds: number;
  /** Where scripted input is sent. The game's action maps are attached here. */
  readonly target: EventTarget;
  /** The stage that logical positions are converted with. */
  readonly stage: StageMap;
  /** The fake gamepads. Set the pads of the run through this. */
  readonly pads: FakeGamepads = new FakeGamepads();
  /** The frame scheduler for a GameLoop that the game builds itself: `new GameLoop({ ..., scheduler: driver.scheduler })`. */
  readonly scheduler: FrameScheduler;

  private readonly clock: GameClock | null;
  private readonly frameMs: number;
  private readonly pending = new Map<number, PendingFrame>();
  private readonly cancelledInBatch = new Set<number>();
  private readonly countedLoops: CountedLoop[] = [];
  private readonly scheduled: ScheduledAction[] = [];
  private readonly tickListeners = new Set<DriverListener>();
  private readonly inputListeners = new Set<(event: ReplayEvent) => void>();
  private readonly restorers: Array<() => void> = [];
  private nextHandle = 1;
  private scheduleOrder = 0;
  private inFrame = false;
  private frameRuns = 0;
  private completedUpdates = 0;
  private pointer: StagePoint = { x: 0, y: 0 };
  private disposed = false;

  /**
   * @throws TestingError with code "bad-argument" when the rate is not above 0 and at most 1000.
   */
  constructor(options: DriverOptions = {}) {
    const rate = options.rate ?? DEFAULT_RATE;
    if (!Number.isFinite(rate) || rate <= 0 || rate > MAX_RATE) {
      throw new TestingError("bad-argument", `rate must be above 0 and at most ${MAX_RATE} steps per second, got ${rate}`);
    }
    this.rate = rate;
    this.stepSeconds = 1 / rate;
    this.frameMs = 1000 / rate;
    this.target = options.target ?? defaultTarget();
    this.stage = options.stage ?? createStageMap();
    this.clock = options.clock ?? null;
    this.scheduler = {
      request: (callback) => this.request(callback),
      cancel: (handle) => this.cancel(handle),
    };
    this.restorers.push(this.pads.install());
  }

  /** The number of update steps that have run. It is also the index of the next update. */
  get frame(): number {
    return this.completedUpdates;
  }

  /** The seconds of loop time that have run: the frames times the step length. */
  get seconds(): number {
    return this.completedUpdates * this.stepSeconds;
  }

  /** The last logical pointer position that the driver sent. */
  get pointerPosition(): StagePoint {
    return { ...this.pointer };
  }

  /**
   * Runs `frames` frames (1 by default). Each one is one update of a driven loop.
   * @throws TestingError with code "not-running" when no loop is scheduled, and "disposed" after dispose.
   */
  step(frames = 1): void {
    this.assertNotDisposed();
    if (!Number.isInteger(frames) || frames < 0) {
      throw new TestingError("bad-argument", `step needs a whole number of frames from 0 up, got ${frames}`);
    }
    for (let done = 0; done < frames; done += 1) this.stepOne();
  }

  /**
   * Runs the frames that make `seconds` of loop time: `round(seconds * rate)` frames.
   * @returns the number of frames that ran.
   */
  advance(seconds: number): number {
    if (!Number.isFinite(seconds) || seconds < 0) {
      throw new TestingError("bad-argument", `advance needs a time of 0 seconds or more, got ${seconds}`);
    }
    const frames = Math.round(seconds * this.rate);
    this.step(frames);
    return frames;
  }

  /**
   * Runs frames until the game clock has moved on by `seconds` of game time. The clock's speed and pause
   * apply, because the game feeds the clock with each step: at speed 2 the same game time takes half the
   * frames, and a paused clock never arrives.
   * @returns the number of frames that ran.
   * @throws TestingError with code "bad-argument" when the driver has no clock, and "clock-stalled" when
   * game time did not get there within the frame budget.
   */
  advanceGame(seconds: number, options: AdvanceGameOptions = {}): number {
    const clock = this.clock;
    if (clock === null) {
      throw new TestingError("bad-argument", "advanceGame needs a clock: pass { clock } to createDriver");
    }
    if (!Number.isFinite(seconds) || seconds < 0) {
      throw new TestingError("bad-argument", `advanceGame needs a game time of 0 seconds or more, got ${seconds}`);
    }
    const target = clock.elapsed + seconds;
    const budget = options.maxFrames ?? Math.ceil(seconds * this.rate) + CLOCK_SLACK_FRAMES;
    const start = clock.elapsed;
    let frames = 0;
    while (clock.elapsed < target - TIME_EPSILON) {
      if (frames >= budget) {
        throw new TestingError(
          "clock-stalled",
          `game time moved from ${start} to ${clock.elapsed} in ${frames} frames, short of ${target}; ` +
            `the clock is paused, too slow, or the game does not feed it`,
        );
      }
      this.step(1);
      frames += 1;
    }
    return frames;
  }

  /**
   * Runs frames until the predicate is true. It is checked before the first frame and after each one.
   * @returns the number of frames that ran.
   * @throws TestingError with code "timeout" when the frame budget runs out first.
   */
  until(predicate: () => boolean, options: UntilOptions = {}): number {
    const budget = options.maxFrames ?? DEFAULT_UNTIL_FRAMES;
    let frames = 0;
    while (!predicate()) {
      if (frames >= budget) {
        throw new TestingError("timeout", `the condition was not met in ${budget} frames (now at frame ${this.frame})`);
      }
      this.step(1);
      frames += 1;
    }
    return frames;
  }

  /**
   * Creates a GameLoop that runs on this driver. Its rate is the driver's rate, and the driver counts the
   * updates it runs. Start it with `start()`, as usual.
   * @throws TestingError with code "bad-argument" when the options name another rate or a scheduler.
   */
  createLoop(options: GameLoopOptions): GameLoop {
    this.assertNotDisposed();
    if (options.rate !== undefined && options.rate !== this.rate) {
      throw new TestingError("bad-argument", `the loop rate ${options.rate} does not match the driver rate ${this.rate}`);
    }
    if (options.scheduler !== undefined) {
      throw new TestingError("bad-argument", "createLoop supplies the driver's scheduler; do not pass one");
    }
    const counted: CountedLoop = { loop: null, updatesInFrame: 0 };
    const loop = new GameLoop({
      ...options,
      rate: this.rate,
      scheduler: this.scheduler,
      update: (dt: number) => {
        counted.updatesInFrame += 1;
        options.update(dt);
      },
    });
    counted.loop = loop;
    this.countedLoops.push(counted);
    return loop;
  }

  /**
   * Points `requestAnimationFrame` and `cancelAnimationFrame` at the driver, so that every frame the page
   * asks for runs on a driver step. Use it for a game that builds its own GameLoop with the default
   * scheduler. The change lasts until the returned function runs, or until dispose.
   */
  takeOverFrames(): () => void {
    this.assertNotDisposed();
    const restores = [
      replaceGlobal("requestAnimationFrame", (callback: FrameRequestCallback): number => this.request(callback)),
      replaceGlobal("cancelAnimationFrame", (handle: number): void => this.cancel(handle)),
    ];
    let restored = false;
    const restore = (): void => {
      if (restored) return;
      restored = true;
      for (const put of restores) put();
    };
    this.restorers.push(restore);
    return restore;
  }

  /**
   * Plays a script from the current frame. Its frames count from here.
   * @returns a handle that says when the script is done, and that cancels it.
   */
  play(script: InputScript): ScriptRun {
    this.assertNotDisposed();
    const base = this.completedUpdates;
    const actions = script.actions();
    let finished = actions.length === 0;
    let remaining = actions.length;
    const mine: ScheduledAction[] = [];
    for (const action of actions) {
      const entry: ScheduledAction = {
        frame: base + action.frame,
        order: this.scheduleOrder,
        run: () => {
          if (finished) return;
          action.run(this);
          remaining -= 1;
          if (remaining === 0) finished = true;
        },
      };
      this.scheduleOrder += 1;
      mine.push(entry);
      this.scheduled.push(entry);
    }
    this.sortScheduled();
    return {
      get done() {
        return finished;
      },
      cancel: () => {
        if (finished) return;
        finished = true;
        for (const entry of mine) {
          const index = this.scheduled.indexOf(entry);
          if (index >= 0) this.scheduled.splice(index, 1);
        }
      },
    };
  }

  /**
   * Runs a listener around every frame.
   * @returns a function that removes the listener.
   */
  addListener(listener: DriverListener): Unsubscribe {
    this.tickListeners.add(listener);
    return () => {
      this.tickListeners.delete(listener);
    };
  }

  /**
   * Runs a listener for every input the driver sends, in the form a replay stores it (without the frame).
   * @returns a function that removes the listener.
   */
  onInput(listener: (event: ReplayEvent) => void): Unsubscribe {
    this.inputListeners.add(listener);
    return () => {
      this.inputListeners.delete(listener);
    };
  }

  /**
   * Presses a key down: a keydown on `target`.
   * @throws TestingError with code "bad-argument" when the code is not a key name.
   */
  keyDown(code: string, options: InputOptions = {}): void {
    this.key("keydown", code, true, options.skipped === true);
  }

  /** Lets a key go up: a keyup on `target`. */
  keyUp(code: string): void {
    this.key("keyup", code, false, false);
  }

  /** Moves the pointer to a logical position: a pointermove on `target`. */
  pointerMove(x: number, y: number): void {
    assertFinitePoint(x, y);
    this.target.dispatchEvent(pointerEvent("pointermove", logicalToClient(this.stage, x, y), -1, 0, false));
    this.pointer = { x, y };
    this.emit({ type: "pointer", x, y });
  }

  /**
   * Presses a mouse button at a logical position, or where the pointer is when no position is given.
   */
  pointerDown(button: MouseButtonName = "left", at?: StagePoint, options: InputOptions = {}): void {
    this.button("pointerdown", button, true, at ?? this.pointer, options.skipped === true);
  }

  /** Lets a mouse button go up at a logical position, or where the pointer is when no position is given. */
  pointerUp(button: MouseButtonName = "left", at?: StagePoint): void {
    this.button("pointerup", button, false, at ?? this.pointer, false);
  }

  /** One wheel tick on `target`. A positive `deltaY` is a wheel that moves down, as in the DOM. */
  wheel(deltaY: number, options: WheelOptions = {}): void {
    const deltaX = options.deltaX ?? 0;
    const deltaMode = options.deltaMode ?? 0;
    if (!Number.isFinite(deltaX) || !Number.isFinite(deltaY) || ![0, 1, 2].includes(deltaMode)) {
      throw new TestingError("bad-argument", "a wheel tick needs finite deltas and a delta mode of 0, 1 or 2");
    }
    const skipped = options.skipped === true;
    this.target.dispatchEvent(wheelEvent(deltaX, deltaY, deltaMode, skipped));
    this.emit({ type: "wheel", deltaX, deltaY, deltaMode: deltaMode as 0 | 1 | 2, ...(skipped ? { skipped: true as const } : {}) });
  }

  /** A window blur on `target`, which releases every held control. */
  blur(): void {
    this.target.dispatchEvent(blurEvent());
    this.emit({ type: "blur" });
  }

  /**
   * Ends the driver: the fake gamepads are removed, the globals that were replaced are put back, and no
   * frame runs any more. Calling it twice does nothing.
   */
  dispose(): void {
    if (this.disposed) return;
    this.disposed = true;
    for (const restore of [...this.restorers].reverse()) restore();
    this.restorers.length = 0;
    this.pending.clear();
    this.scheduled.length = 0;
    this.tickListeners.clear();
    this.inputListeners.clear();
  }

  /** Runs one counted frame: the origins of new loops, then the scheduled input, then the callbacks. */
  private stepOne(): void {
    this.runOrigins();
    this.assertScheduled();
    const frame = this.completedUpdates;
    const running = this.countedLoops.filter((entry) => entry.loop?.running === true);
    for (const entry of running) entry.updatesInFrame = 0;
    this.runDueActions(frame);
    for (const listener of [...this.tickListeners]) listener.beforeTick?.(frame);
    this.runCallbacks();
    for (const entry of running) {
      if (entry.loop?.running === true && entry.updatesInFrame !== 1) {
        throw new TestingError(
          "bad-argument",
          `a driven loop ran ${entry.updatesInFrame} updates in frame ${frame}; it must run exactly one. ` +
            `Check that its rate is ${this.rate}, and that it was started between frames`,
        );
      }
    }
    this.completedUpdates += 1;
    for (const listener of [...this.tickListeners]) listener.afterTick?.(frame);
  }

  /**
   * Runs the callbacks that were requested from outside a frame, once, at the time of the last frame.
   * The clock does not move and no other callback runs, so the loop's first counted frame is its first update.
   */
  private runOrigins(): void {
    const origins = [...this.pending].filter(([, entry]) => entry.origin);
    if (origins.length === 0) return;
    for (const [handle] of origins) this.pending.delete(handle);
    this.runBatch(origins, this.frameRuns * this.frameMs);
  }

  /** Runs every callback that is waiting, once, in the order they were requested. The clock moves one step. */
  private runCallbacks(): void {
    this.frameRuns += 1;
    const batch = [...this.pending];
    this.pending.clear();
    this.runBatch(batch, this.frameRuns * this.frameMs);
  }

  /** Runs a batch of callbacks with the given time. A callback cancelled by an earlier one of the batch is skipped. */
  private runBatch(batch: ReadonlyArray<readonly [number, PendingFrame]>, timeMs: number): void {
    this.inFrame = true;
    try {
      for (const [handle, entry] of batch) {
        if (this.cancelledInBatch.has(handle)) continue;
        entry.callback(timeMs);
      }
    } finally {
      this.inFrame = false;
      this.cancelledInBatch.clear();
    }
  }

  /** Runs the scheduled actions that are due by this frame, in frame and insertion order. */
  private runDueActions(frame: number): void {
    while (this.scheduled.length > 0 && (this.scheduled[0]?.frame ?? Infinity) <= frame) {
      const next = this.scheduled.shift();
      next?.run();
    }
  }

  /** Keeps the scheduled actions in frame order, and in insertion order within a frame. */
  private sortScheduled(): void {
    this.scheduled.sort((left, right) => left.frame - right.frame || left.order - right.order);
  }

  /**
   * Accepts a callback from a loop or from the page, and waits for the next frame to run it. A request made
   * from outside a frame is an origin: it runs once before the next counted frame (see runOrigins).
   */
  private request(callback: (timeMs: number) => void): number {
    this.assertNotDisposed();
    const handle = this.nextHandle;
    this.nextHandle += 1;
    this.pending.set(handle, { callback, origin: !this.inFrame });
    return handle;
  }

  /** Drops a waiting callback. One that is already in the frame's batch is skipped when its turn comes. */
  private cancel(handle: number): void {
    if (!this.pending.delete(handle) && this.inFrame) this.cancelledInBatch.add(handle);
  }

  /** Throws unless a frame is waiting to run. */
  private assertScheduled(): void {
    if (this.pending.size === 0) {
      throw new TestingError(
        "not-running",
        "no frame is scheduled: start the game's loop before stepping, or check that the game did not stop it",
      );
    }
  }

  /** Throws when the driver was disposed. */
  private assertNotDisposed(): void {
    if (this.disposed) throw new TestingError("disposed", "the driver was disposed and cannot run any more");
  }

  /** Sends one key event, and reports it. */
  private key(type: "keydown" | "keyup", code: string, down: boolean, skipped: boolean): void {
    if (!/^[A-Za-z0-9]{1,32}$/.test(code)) {
      throw new TestingError("bad-argument", `a key code is 1 to 32 letters or digits, got ${JSON.stringify(code)}`);
    }
    this.target.dispatchEvent(keyEvent(type, code, skipped));
    this.emit({ type: "key", code, down, ...(skipped ? { skipped: true as const } : {}) });
  }

  /** Sends one mouse button event at a logical position, and reports it. */
  private button(
    type: "pointerdown" | "pointerup",
    button: MouseButtonName,
    down: boolean,
    at: StagePoint,
    skipped: boolean,
  ): void {
    assertFinitePoint(at.x, at.y);
    this.target.dispatchEvent(
      pointerEvent(type, logicalToClient(this.stage, at.x, at.y), buttonNumber(button), down ? 1 : 0, skipped),
    );
    this.pointer = { x: at.x, y: at.y };
    this.emit({ type: "button", button, down, x: at.x, y: at.y, ...(skipped ? { skipped: true as const } : {}) });
  }

  /** Tells the input listeners about one input. */
  private emit(event: ReplayEvent): void {
    for (const listener of [...this.inputListeners]) listener(event);
  }
}

/**
 * Creates a driver. It is the only way to make one; the class is exported for its type.
 * @throws TestingError with code "bad-argument" for a rate that the driver cannot run.
 */
export function createDriver(options: DriverOptions = {}): Driver {
  return new Driver(options);
}

/** The target of scripted input when the options name none: the page's window, or a plain target without a page. */
function defaultTarget(): EventTarget {
  return typeof window === "undefined" ? new EventTarget() : window;
}

/**
 * Replaces a global with a value, and returns the function that puts the original back. The original is
 * deleted again when the global did not exist before.
 */
function replaceGlobal(name: string, value: unknown): () => void {
  const previous = Object.getOwnPropertyDescriptor(globalThis, name);
  Object.defineProperty(globalThis, name, { value, configurable: true, writable: true, enumerable: false });
  return () => {
    if (previous !== undefined) Object.defineProperty(globalThis, name, previous);
    else Reflect.deleteProperty(globalThis, name);
  };
}

/** Throws unless a logical position is made of finite numbers. */
function assertFinitePoint(x: number, y: number): void {
  if (!Number.isFinite(x) || !Number.isFinite(y)) {
    throw new TestingError("bad-argument", `a position needs finite coordinates, got ${x}, ${y}`);
  }
}
