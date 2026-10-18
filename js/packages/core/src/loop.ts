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

/** Default update rate, in steps per second. */
const DEFAULT_RATE = 60;

/** Default most steps that one frame may run. */
const DEFAULT_MAX_STEPS = 5;

/** Frame interval of the setTimeout fallback, in milliseconds. */
const FALLBACK_FRAME_MS = 1000 / 60;

/** Tolerance for floating-point error when the accumulator is compared with a step. */
const EPSILON = 1e-9;

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
}

/** A fixed-step loop. Create it, call `start()`, and call `stop()` to end it. */
export class GameLoop {
  private readonly options: GameLoopOptions;
  private readonly scheduler: FrameScheduler;
  private readonly stepSecondsValue: number;
  private readonly maxSteps: number;
  private accumulator = 0;
  private lastTimeMs: number | null = null;
  private pendingFrame: number | null = null;
  private active = false;

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

  private runFrame(timeMs: number): void {
    this.pendingFrame = null;
    if (!this.active) return;
    // Schedule the next frame first, so that `stop()` called from `update` cancels it.
    this.scheduleFrame();

    const previousMs = this.lastTimeMs ?? timeMs;
    this.lastTimeMs = timeMs;
    const elapsed = Math.max(0, timeMs - previousMs) / 1000;
    const budget = this.maxSteps * this.stepSecondsValue;
    this.accumulator = Math.min(this.accumulator + elapsed, budget);

    let steps = 0;
    while (this.active && steps < this.maxSteps && this.accumulator + EPSILON >= this.stepSecondsValue) {
      this.options.update(this.stepSecondsValue);
      this.accumulator = Math.max(0, this.accumulator - this.stepSecondsValue);
      steps += 1;
    }

    if (this.active) {
      const alpha = Math.min(1, Math.max(0, this.accumulator / this.stepSecondsValue));
      this.options.render?.(alpha);
    }
  }
}
