// A fixed-step game loop.
//
// The loop calls `update(dt)` with the same step length every time, whatever the frame
// rate. Real time goes into an accumulator, and each full step is spent on one update. A
// late frame gets several updates in a row, but never more than `maxSteps`: time beyond
// that is dropped, so a stall (a hidden tab, a long garbage collection) does not become a
// long burst of catch-up work. After the steps, `render(alpha)` runs once. Alpha, in [0, 1),
// is how far the next step has progressed, so drawing can interpolate between states.
//
// Frames come from a FrameScheduler. The default uses requestAnimationFrame, and falls back
// to setTimeout where the host has none. Tests pass their own scheduler and drive time.
//
// Frame hooks. A game or a tool can run code at five points of each frame without wrapping the
// options: `loop.on(phase, listener)` adds a listener, and the function it returns removes it.
// The phases run in this order, once for each frame that the loop runs:
//
//   beforeFrame   once, before the steps. Its payload holds the frame's number, its time and the
//                 real time since the previous frame.
//   beforeUpdate  before each fixed step, with the step's place in the frame and its length.
//   afterUpdate   after each fixed step, with the same, and how long the update took.
//   afterRender   after the render, with alpha and how long the render took. It runs even when
//                 there is no render function, with a duration of zero.
//   afterFrame    last, with the timing of the whole frame: the steps, the update, render and
//                 frame times in milliseconds, the real time since the previous frame, and the
//                 game-step time that the frame ran.
//
// Input and the hooks. The loop never calls `input.endFrame()`: the game does, and the hooks
// see input as it stands at their point of the frame. Edges stay until the game clears them. In
// the README example `endFrame()` is the last call of `update`, so `afterUpdate`, `afterRender`
// and `afterFrame` see the edges already cleared, while `beforeFrame` and `beforeUpdate` see the
// edges that the game has not yet cleared.
//
// Cost and errors. With no hook listening, a frame reads no clock and builds no payload. A
// listener added during a frame takes effect from the next one. A listener that throws stops the
// frame, as an error from `update` does, and the next frame is already scheduled. A `stop()` call
// during a frame ends its remaining steps and its render, and `afterFrame` still runs once for it.

import { EventEmitter, type Listener, type Unsubscribe } from "./events";

/** Default update rate, in steps per second. */
const DEFAULT_RATE = 60;

/** Default most steps that one frame may run. */
const DEFAULT_MAX_STEPS = 5;

/** Frame interval of the setTimeout fallback, in milliseconds. */
const FALLBACK_FRAME_MS = 1000 / 60;

/** Tolerance for floating-point error when the accumulator is compared with a step. */
const EPSILON = 1e-9;

/** The phases of a frame that a hook can listen to, in the order they run. */
export type LoopPhase = "beforeFrame" | "beforeUpdate" | "afterUpdate" | "afterRender" | "afterFrame";

/** The payload of `beforeFrame`: the start of a frame. */
export interface FrameStart {
  /** This frame's number. The first frame that the loop runs is 1. */
  readonly frame: number;
  /** The scheduler's time for this frame, in milliseconds. */
  readonly timeMs: number;
  /** Real seconds since the previous frame. Zero for the first frame. */
  readonly realDt: number;
}

/** The payload of `beforeUpdate`: a fixed step, before its update runs. */
export interface StepStart {
  /** The number of the frame that holds the step. */
  readonly frame: number;
  /** The step's place in its frame, counting from 0. */
  readonly step: number;
  /** The length of the step in seconds: the `dt` that `update` receives. */
  readonly dt: number;
}

/** The payload of `afterUpdate`: a fixed step, after its update ran. */
export interface StepEnd extends StepStart {
  /** Milliseconds that the update took. */
  readonly durationMs: number;
}

/** The payload of `afterRender`: the render of a frame. */
export interface RenderEnd {
  /** The number of the frame. */
  readonly frame: number;
  /** The alpha that `render` received, in [0, 1). */
  readonly alpha: number;
  /** Milliseconds that the render took. Zero without a render function. */
  readonly durationMs: number;
}

/** The payload of `afterFrame`: the timing of a whole frame. */
export interface FrameTiming {
  /** This frame's number. */
  readonly frame: number;
  /** The scheduler's time for this frame, in milliseconds. */
  readonly timeMs: number;
  /** Real seconds since the previous frame. Zero for the first frame. */
  readonly realDt: number;
  /** Game-step seconds that this frame ran: the number of steps times the step length. */
  readonly scaledDt: number;
  /** The fixed steps that this frame ran. */
  readonly steps: number;
  /** Milliseconds spent in the update calls, all steps together. */
  readonly updateMs: number;
  /** Milliseconds spent in the render call. Zero without a render function, or when the loop stopped first. */
  readonly renderMs: number;
  /** Milliseconds from the start of the frame to the end of its last phase. */
  readonly frameMs: number;
}

/** The payload of each phase, by the phase's name. */
export interface LoopHooks {
  beforeFrame: FrameStart;
  beforeUpdate: StepStart;
  afterUpdate: StepEnd;
  afterRender: RenderEnd;
  afterFrame: FrameTiming;
}

/** Requests frames and cancels them. Inject one to drive the loop from a test or a custom host. */
export interface FrameScheduler {
  /**
   * Asks for `callback` to run on the next frame. The callback receives a timestamp in
   * milliseconds.
   * @returns a handle that `cancel` accepts.
   */
  request(callback: (timeMs: number) => void): number;
  /** Cancels a request, given the handle that `request` returned. */
  cancel(handle: number): void;
}

/**
 * Creates the default scheduler: requestAnimationFrame where the host has it, otherwise
 * setTimeout at 60 Hz.
 */
export function createFrameScheduler(): FrameScheduler {
  if (typeof requestAnimationFrame === "function" && typeof cancelAnimationFrame === "function") {
    return {
      request: (callback) => requestAnimationFrame(callback),
      cancel: (handle) => cancelAnimationFrame(handle),
    };
  }
  // Without frames (a worker, Node): a timer per frame. Node's timers are objects, not numbers,
  // so the handles are kept in a table under numbers of our own.
  const timers = new Map<number, ReturnType<typeof setTimeout>>();
  let next = 1;
  return {
    request: (callback) => {
      const handle = next++;
      timers.set(
        handle,
        setTimeout(() => {
          timers.delete(handle);
          callback(Date.now());
        }, FALLBACK_FRAME_MS),
      );
      return handle;
    },
    cancel: (handle) => {
      const timer = timers.get(handle);
      if (timer !== undefined) clearTimeout(timer);
      timers.delete(handle);
    },
  };
}

/** The default clock for the frame timings: performance.now where the host has it. */
function defaultNow(): number {
  return typeof performance === "undefined" ? Date.now() : performance.now();
}

/** Options for {@link GameLoop}. */
export interface GameLoopOptions {
  /**
   * Runs once per fixed step, with the step length in seconds. An error thrown here reaches
   * the host's error handler. The next frame is already scheduled, so call `stop()` in that
   * handler to end the loop.
   */
  update(dt: number): void;
  /** Runs once per frame after the steps, with `alpha` in [0, 1). Optional. */
  render?(alpha: number): void;
  /** Steps per second. Default 60. */
  rate?: number;
  /** The most steps one frame may run. Time beyond that is dropped. Default 5. */
  maxSteps?: number;
  /** Where frames come from. Default: {@link createFrameScheduler}. */
  scheduler?: FrameScheduler;
  /**
   * The clock for the frame timings, in milliseconds, on a clock that never goes back. Default:
   * performance.now. It is read only while a hook listens.
   */
  now?: () => number;
}

/** A fixed-step loop. Create it, call `start()`, and call `stop()` to end it. */
export class GameLoop {
  private readonly options: GameLoopOptions;
  private readonly scheduler: FrameScheduler;
  private readonly stepSecondsValue: number;
  private readonly maxSteps: number;
  private readonly now: () => number;
  private readonly hooks = new EventEmitter<LoopHooks>();
  private accumulator = 0;
  private lastTimeMs: number | null = null;
  private pendingFrame: number | null = null;
  private active = false;
  private frameNumber = 0;

  /**
   * @throws RangeError when `rate` is not a positive finite number, or `maxSteps` is not a
   * positive integer.
   */
  constructor(options: GameLoopOptions) {
    const rate = options.rate ?? DEFAULT_RATE;
    const maxSteps = options.maxSteps ?? DEFAULT_MAX_STEPS;
    if (!Number.isFinite(rate) || rate <= 0) {
      throw new RangeError(`rate must be a positive number, got ${rate}`);
    }
    if (!Number.isInteger(maxSteps) || maxSteps < 1) {
      throw new RangeError(`maxSteps must be a positive integer, got ${maxSteps}`);
    }
    this.options = options;
    this.scheduler = options.scheduler ?? createFrameScheduler();
    this.stepSecondsValue = 1 / rate;
    this.maxSteps = maxSteps;
    this.now = options.now ?? defaultNow;
  }

  /** The length of one fixed step, in seconds. */
  get stepSeconds(): number {
    return this.stepSecondsValue;
  }

  /** True between `start()` and `stop()`. */
  get running(): boolean {
    return this.active;
  }

  /**
   * Adds a listener for one phase of each frame. The phases and their payloads are described at the top
   * of this file. Adding a listener costs nothing to the frames that have none.
   * @returns a function that removes the listener.
   */
  on<Phase extends LoopPhase>(phase: Phase, listener: Listener<LoopHooks[Phase]>): Unsubscribe {
    return this.hooks.on(phase, listener);
  }

  /**
   * Starts the loop. The first frame only sets the time origin, so it renders without
   * stepping. Calling it while the loop runs does nothing.
   */
  start(): void {
    if (this.active) return;
    this.active = true;
    this.accumulator = 0;
    this.lastTimeMs = null;
    this.scheduleFrame();
  }

  /** Stops the loop and cancels the pending frame. Calling it while stopped does nothing. */
  stop(): void {
    if (!this.active) return;
    this.active = false;
    if (this.pendingFrame !== null) {
      this.scheduler.cancel(this.pendingFrame);
      this.pendingFrame = null;
    }
  }

  private scheduleFrame(): void {
    this.pendingFrame = this.scheduler.request((timeMs) => this.runFrame(timeMs));
  }

  /** True when a hook listens to some phase. Only then does a frame read the clock and build payloads. */
  private isListening(): boolean {
    return (
      this.hooks.listenerCount("beforeFrame") +
        this.hooks.listenerCount("beforeUpdate") +
        this.hooks.listenerCount("afterUpdate") +
        this.hooks.listenerCount("afterRender") +
        this.hooks.listenerCount("afterFrame") >
      0
    );
  }

  private runFrame(timeMs: number): void {
    this.pendingFrame = null;
    if (!this.active) return;
    // Schedule the next frame first, so that `stop()` called from `update` cancels it.
    this.scheduleFrame();

    const timed = this.isListening();
    const frameStartMs = timed ? this.now() : 0;
    const previousMs = this.lastTimeMs ?? timeMs;
    this.lastTimeMs = timeMs;
    const elapsed = Math.max(0, timeMs - previousMs) / 1000;
    this.frameNumber += 1;
    const frame = this.frameNumber;
    if (timed) this.hooks.emit("beforeFrame", { frame, timeMs, realDt: elapsed });

    const stepSeconds = this.stepSecondsValue;
    const budget = this.maxSteps * stepSeconds;
    this.accumulator = Math.min(this.accumulator + elapsed, budget);

    let steps = 0;
    let updateMs = 0;
    while (this.active && steps < this.maxSteps && this.accumulator + EPSILON >= stepSeconds) {
      if (timed) this.hooks.emit("beforeUpdate", { frame, step: steps, dt: stepSeconds });
      const updateStartMs = timed ? this.now() : 0;
      this.options.update(stepSeconds);
      const durationMs = timed ? this.now() - updateStartMs : 0;
      updateMs += durationMs;
      if (timed) this.hooks.emit("afterUpdate", { frame, step: steps, dt: stepSeconds, durationMs });
      this.accumulator = Math.max(0, this.accumulator - stepSeconds);
      steps += 1;
    }

    let renderMs = 0;
    if (this.active) {
      const alpha = Math.min(1, Math.max(0, this.accumulator / stepSeconds));
      // Without a render function the render phase takes no time, so its duration stays zero.
      const measureRender = timed && this.options.render !== undefined;
      const renderStartMs = measureRender ? this.now() : 0;
      this.options.render?.(alpha);
      renderMs = measureRender ? this.now() - renderStartMs : 0;
      if (timed) this.hooks.emit("afterRender", { frame, alpha, durationMs: renderMs });
    }

    if (timed) {
      this.hooks.emit("afterFrame", {
        frame,
        timeMs,
        realDt: elapsed,
        scaledDt: steps * stepSeconds,
        steps,
        updateMs,
        renderMs,
        frameMs: this.now() - frameStartMs,
      });
    }
  }
}
