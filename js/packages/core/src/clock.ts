// The game clock: game time, fed by the loop, with pause, speed multipliers and a day cycle.
//
// `advance(dt)` takes real seconds from the loop. While the clock runs, game time moves by
// `dt * speed`; while paused it does not move. Game time is counted in game seconds, and a
// day lasts `dayLength` game seconds. Day 1 is the day the clock starts on, and `onDay(day)`
// runs each time a later day begins.

/** Speed multipliers offered by default. */
const DEFAULT_SPEEDS: readonly number[] = [1, 2, 3];

/** Length of one in-game day by default, in game seconds. */
const DEFAULT_DAY_LENGTH = 600;

/** Options for {@link GameClock}. */
export interface GameClockOptions {
  /** Allowed speed multipliers, sorted ascending. The clock starts at the first one. Default [1, 2, 3]. */
  speeds?: readonly number[];
  /** Length of one in-game day, in game seconds. Default 600. */
  dayLength?: number;
  /** Runs with the new day number each time a day begins. Day 1 never fires it. */
  onDay?(day: number): void;
}

/** What a save keeps of the clock: game time, speed and pause. */
export interface GameClockState {
  /** Game seconds elapsed. */
  elapsed: number;
  /** The speed multiplier: one of the clock's speeds. */
  speed: number;
  /** Whether the clock was paused. */
  paused: boolean;
}

/** Game time with pause, speed multipliers and days. */
export class GameClock {
  private readonly speedList: readonly number[];
  private readonly dayLengthSeconds: number;
  private readonly onNewDay: ((day: number) => void) | undefined;
  private speedIndex = 0;
  private isPaused = false;
  private elapsedSeconds = 0;
  private currentDay = 1;

  /**
   * @throws RangeError when `speeds` is empty or holds a value that is not a positive finite
   * number, or when `dayLength` is not a positive finite number.
   */
  constructor(options: GameClockOptions = {}) {
    const speeds = [...(options.speeds ?? DEFAULT_SPEEDS)].sort((a, b) => a - b);
    if (speeds.length === 0 || speeds.some((speed) => !Number.isFinite(speed) || speed <= 0)) {
      throw new RangeError("speeds must be a non-empty list of positive numbers");
    }
    const dayLength = options.dayLength ?? DEFAULT_DAY_LENGTH;
    if (!Number.isFinite(dayLength) || dayLength <= 0) {
      throw new RangeError(`dayLength must be a positive number, got ${dayLength}`);
    }
    this.speedList = speeds;
    this.dayLengthSeconds = dayLength;
    this.onNewDay = options.onDay;
  }

  /** The speed multipliers the clock can run at, ascending. */
  get speeds(): readonly number[] {
    return this.speedList;
  }

  /** The current speed multiplier. */
  get speed(): number {
    return speedAt(this.speedList, this.speedIndex);
  }

  /**
   * Sets the speed.
   * @throws RangeError when `speed` is not one of the allowed speeds.
   */
  setSpeed(speed: number): void {
    const index = this.speedList.indexOf(speed);
    if (index < 0) {
      throw new RangeError(`speed ${speed} is not allowed; the allowed speeds are ${this.speedList.join(", ")}`);
    }
    this.speedIndex = index;
  }

  /** Moves to the next allowed speed, wrapping around after the last. Returns the new speed. */
  cycleSpeed(): number {
    this.speedIndex = (this.speedIndex + 1) % this.speedList.length;
    return this.speed;
  }

  /** True while the clock is paused. */
  get paused(): boolean {
    return this.isPaused;
  }

  /** Stops game time. Calling it while paused does nothing. */
  pause(): void {
    this.isPaused = true;
  }

  /** Lets game time run again. */
  resume(): void {
    this.isPaused = false;
  }

  /** Game seconds elapsed since the clock started: speed applied, pauses excluded. */
  get elapsed(): number {
    return this.elapsedSeconds;
  }

  /** The current day, starting at 1. */
  get day(): number {
    return this.currentDay;
  }

  /** Game seconds into the current day, in the range [0, dayLength). */
  get timeOfDay(): number {
    return this.elapsedSeconds % this.dayLengthSeconds;
  }

  /** Length of one in-game day, in game seconds. */
  get dayLength(): number {
    return this.dayLengthSeconds;
  }

  /** The clock's state, for a save. */
  snapshot(): GameClockState {
    return { elapsed: this.elapsedSeconds, speed: this.speed, paused: this.isPaused };
  }

  /**
   * Puts the clock back where a save left it. The day follows from the time, and `onDay` does
   * not fire for the days skipped over.
   * @throws RangeError when `elapsed` is negative or not finite, or `speed` is not allowed.
   */
  restore(state: GameClockState): void {
    if (!Number.isFinite(state.elapsed) || state.elapsed < 0) {
      throw new RangeError(`elapsed must be a non-negative finite number, got ${state.elapsed}`);
    }
    this.setSpeed(state.speed);
    this.elapsedSeconds = state.elapsed;
    this.currentDay = 1 + Math.floor(state.elapsed / this.dayLengthSeconds);
    this.isPaused = state.paused;
  }

  /**
   * Advances the clock by `dt` real seconds, the loop's step. Does nothing while paused.
   * Each day that begins during the step triggers `onDay`, in order.
   * @throws RangeError when `dt` is negative or not finite.
   */
  advance(dt: number): void {
    if (!Number.isFinite(dt) || dt < 0) {
      throw new RangeError(`dt must be a non-negative finite number, got ${dt}`);
    }
    if (this.isPaused) return;
    this.elapsedSeconds += dt * this.speed;
    const day = 1 + Math.floor(this.elapsedSeconds / this.dayLengthSeconds);
    while (this.currentDay < day) {
      this.currentDay += 1;
      this.onNewDay?.(this.currentDay);
    }
  }
}

/** The speed at `index`. The index always comes from the clock, so a miss is a bug. */
function speedAt(speeds: readonly number[], index: number): number {
  const speed = speeds[index];
  if (speed === undefined) throw new RangeError(`no speed at index ${index}`);
  return speed;
}
