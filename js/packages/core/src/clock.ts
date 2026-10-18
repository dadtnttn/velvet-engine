// The game clock: game time, fed by the loop, with pause, speed multipliers and a day cycle.
//
// `advance(dt)` takes real seconds from the loop. While the clock runs, game time moves by
// `dt * speed`; while paused it does not move. Game time is counted in game seconds, and a
// day lasts `dayLength` game seconds. Day 1 is the day the clock starts on, and `onDay(day)`
// runs each time a later day begins.
//
// Events. The clock emits `paused` and `resumed` when its pause changes, `speed` when its speed
// changes, and `stepped` each time `advance` moves it. They go through `on`, `once` and `off`, like
// the other emitters. Pausing a paused clock, or setting the speed it has, sends nothing. A
// `restore` sends the changes that it makes, so a listener stays right after a load.

import { EventEmitter, type Listener, type Unsubscribe } from "./events";

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

/**
 * The clock's state for tools and the screen: what a save keeps, plus the day and the speeds. A copy:
 * changing it changes nothing in the clock.
 */
export interface GameClockStatus extends GameClockState {
  /** The current day, starting at 1. */
  readonly day: number;
  /** Game seconds into the current day, in the range [0, dayLength). */
  readonly timeOfDay: number;
  /** Length of one in-game day, in game seconds. */
  readonly dayLength: number;
  /** The speeds the clock allows, ascending. */
  readonly speeds: readonly number[];
}

/** The events of a game clock, by name. */
export interface GameClockEvents {
  /** The clock was paused. Sent once per pause, with the state after it. */
  paused: GameClockStatus;
  /** The clock resumed. Sent once per resume, with the state after it. */
  resumed: GameClockStatus;
  /** The speed changed from `previous` to `current`. */
  speed: { previous: number; current: number };
  /** The clock moved on by one `advance` while it runs: `dt` real seconds became `gameSeconds` of game time. */
  stepped: { dt: number; gameSeconds: number };
}

/** Game time with pause, speed multipliers and days. */
export class GameClock {
  private readonly speedList: readonly number[];
  private readonly dayLengthSeconds: number;
  private readonly onNewDay: ((day: number) => void) | undefined;
  private readonly events = new EventEmitter<GameClockEvents>();
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
   * Sets the speed. Sends a `speed` event when it changes.
   * @throws RangeError when `speed` is not one of the allowed speeds.
   */
  setSpeed(speed: number): void {
    const index = this.indexOfSpeed(speed);
    const previous = this.speed;
    this.speedIndex = index;
    this.announceSpeed(previous);
  }

  /**
   * Moves to the next allowed speed, wrapping around after the last. Returns the new speed. Sends a
   * `speed` event when the speed changes.
   */
  cycleSpeed(): number {
    const previous = this.speed;
    this.speedIndex = (this.speedIndex + 1) % this.speedList.length;
    this.announceSpeed(previous);
    return this.speed;
  }

  /** True while the clock is paused. */
  get paused(): boolean {
    return this.isPaused;
  }

  /** Stops game time. Calling it while paused does nothing. */
  pause(): void {
    if (this.isPaused) return;
    this.isPaused = true;
    this.events.emit("paused", this.state());
  }

  /** Lets game time run again. Calling it while running does nothing. */
  resume(): void {
    if (!this.isPaused) return;
    this.isPaused = false;
    this.events.emit("resumed", this.state());
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
   * The clock's state for tools and the screen: what `snapshot` keeps, plus the day, the time of the
   * day, the length of the day and the speeds. It is a copy.
   */
  state(): GameClockStatus {
    return {
      elapsed: this.elapsedSeconds,
      speed: this.speed,
      paused: this.isPaused,
      day: this.currentDay,
      timeOfDay: this.timeOfDay,
      dayLength: this.dayLengthSeconds,
      speeds: [...this.speedList],
    };
  }

  /**
   * Puts the clock back where a save left it. The day follows from the time, and `onDay` does
   * not fire for the days skipped over. The events for the changes it makes are sent after the
   * clock has been set.
   * @throws RangeError when `elapsed` is negative or not finite, or `speed` is not allowed.
   */
  restore(state: GameClockState): void {
    if (!Number.isFinite(state.elapsed) || state.elapsed < 0) {
      throw new RangeError(`elapsed must be a non-negative finite number, got ${state.elapsed}`);
    }
    const index = this.indexOfSpeed(state.speed);
    const previousSpeed = this.speed;
    const wasPaused = this.isPaused;
    this.speedIndex = index;
    this.elapsedSeconds = state.elapsed;
    this.currentDay = 1 + Math.floor(state.elapsed / this.dayLengthSeconds);
    this.isPaused = state.paused;
    this.announceSpeed(previousSpeed);
    if (wasPaused !== this.isPaused) this.events.emit(this.isPaused ? "paused" : "resumed", this.state());
  }

  /**
   * Advances the clock by `dt` real seconds, the loop's step. Does nothing while paused.
   * Each day that begins during the step triggers `onDay`, in order. A running clock then sends a
   * `stepped` event.
   * @throws RangeError when `dt` is negative or not finite.
   */
  advance(dt: number): void {
    if (!Number.isFinite(dt) || dt < 0) {
      throw new RangeError(`dt must be a non-negative finite number, got ${dt}`);
    }
    if (this.isPaused) return;
    const gameSeconds = dt * this.speed;
    this.elapsedSeconds += gameSeconds;
    const day = 1 + Math.floor(this.elapsedSeconds / this.dayLengthSeconds);
    while (this.currentDay < day) {
      this.currentDay += 1;
      this.onNewDay?.(this.currentDay);
    }
    // Only build the payload when someone reads it: this runs once per loop step.
    if (this.events.listenerCount("stepped") > 0) this.events.emit("stepped", { dt, gameSeconds });
  }

  /** Adds a listener for a clock event. @returns a function that removes it. */
  on<Type extends keyof GameClockEvents>(type: Type, listener: Listener<GameClockEvents[Type]>): Unsubscribe {
    return this.events.on(type, listener);
  }

  /** Adds a listener that runs once, at the next event of that type. @returns a function that removes it. */
  once<Type extends keyof GameClockEvents>(type: Type, listener: Listener<GameClockEvents[Type]>): Unsubscribe {
    return this.events.once(type, listener);
  }

  /** Removes a listener added with `on` or `once`. */
  off<Type extends keyof GameClockEvents>(type: Type, listener: Listener<GameClockEvents[Type]>): void {
    this.events.off(type, listener);
  }

  /** The index of an allowed speed. @throws RangeError when the speed is not allowed. */
  private indexOfSpeed(speed: number): number {
    const index = this.speedList.indexOf(speed);
    if (index < 0) {
      throw new RangeError(`speed ${speed} is not allowed; the allowed speeds are ${this.speedList.join(", ")}`);
    }
    return index;
  }

  /** Sends a `speed` event when the speed differs from `previous`. */
  private announceSpeed(previous: number): void {
    const current = this.speed;
    if (current !== previous) this.events.emit("speed", { previous, current });
  }
}

/** The speed at `index`. The index always comes from the clock, so a miss is a bug. */
function speedAt(speeds: readonly number[], index: number): number {
  const speed = speeds[index];
  if (speed === undefined) throw new RangeError(`no speed at index ${index}`);
  return speed;
}
