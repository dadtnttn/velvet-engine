// Game time controls: pause, resume, step, speed and slow motion, driven through the core clock.
//
// A paused clock keeps game time still, so logic that reads the clock stands still with it. When the
// devtools have the game loop, a pause also holds the loop (GameLoop.hold), so the game logic stops too,
// and a step runs one update (GameLoop.stepOnce) with the clock running for that call. A wrapped loop
// (Devtools.wrapLoop, older) does the same through the wrapper. With neither, a step only moves the clock
// on by one loop step.

import type { GameClock } from "@velvet/core";

/** The step length used when no loop is given: one frame at 60 Hz. */
export const DEFAULT_STEP_SECONDS = 1 / 60;

/** What the Time tab shows about the clock and the controls. */
export interface TimeState {
  /** False when no clock was given to the devtools. */
  readonly hasClock: boolean;
  /** True while game time is stopped, by the devtools or by the game. */
  readonly paused: boolean;
  /** The clock's speed, or null without a clock. */
  readonly speed: number | null;
  /** The speeds that the clock allows, ascending. */
  readonly speeds: readonly number[];
  /** True while slow motion is on. */
  readonly slowMotion: boolean;
  /** The speed that slow motion uses. */
  readonly slowSpeed: number;
  /** True when the clock allows the slow-motion speed. */
  readonly slowAllowed: boolean;
}

/** The part of the core GameLoop that the controls use to hold and step the game. */
export interface HoldableLoop {
  hold(): () => void;
  stepOnce(): void;
}

/** Holds the state of the controls, and applies them to the clock. */
export class TimeControls {
  private clock: GameClock | null;
  private readonly slowSpeed: number;
  private readonly stepSeconds: number;
  private frozen = false;
  private clockPausedHere = false;
  private stepRequested = false;
  private speedBeforeSlow: number | null = null;
  private wrapped = false;
  private readonly loop: HoldableLoop | null;
  private releaseHold: (() => void) | null = null;

  /**
   * @param clock - the game clock, or null when none was given.
   * @param slowSpeed - the speed that slow motion uses.
   * @param stepSeconds - the length of one loop step, in seconds.
   * @param loop - the game loop, which a pause holds and a step runs once, or null when none was given.
   */
  constructor(clock: GameClock | null, slowSpeed: number, stepSeconds: number, loop: HoldableLoop | null = null) {
    this.clock = clock;
    this.slowSpeed = slowSpeed;
    this.stepSeconds = stepSeconds;
    this.loop = loop;
  }

  /** Records that the loop is wrapped. From then on, a pause holds the game's update. */
  markWrapped(): void {
    this.wrapped = true;
  }

  /**
   * Connects another game clock, or none. The clock that was there is given back first (see `releaseClock`). A pause
   * that the tools hold stays on, so a clock connected while paused is paused too.
   */
  attach(clock: GameClock | null): void {
    if (clock === this.clock) return;
    this.releaseClock();
    this.clock = clock;
    if (clock !== null && this.frozen && !clock.paused) {
      clock.pause();
      this.clockPausedHere = true;
    }
  }

  /** Stops game time. A wrapped loop also holds the game's update. Calling it while paused does nothing. */
  pause(): void {
    if (this.frozen) return;
    this.frozen = true;
    if (this.clock !== null && !this.clock.paused) {
      this.clock.pause();
      this.clockPausedHere = true;
    }
    // A wrapped loop holds the update itself; otherwise the loop, when there is one, is held.
    if (!this.wrapped && this.loop !== null) this.releaseHold = this.loop.hold();
  }

  /** Lets game time run again. Calling it while running does nothing. */
  resume(): void {
    if (!this.frozen) return;
    this.frozen = false;
    this.stepRequested = false;
    if (this.clockPausedHere) this.clock?.resume();
    this.clockPausedHere = false;
    this.releaseHold?.();
    this.releaseHold = null;
  }

  /** Moves game time on by one step. Pauses first when the game is running. */
  step(): void {
    this.pause();
    if (this.wrapped) {
      this.stepRequested = true;
      return;
    }
    if (this.loop !== null) {
      // The held loop runs one update, with the clock running for that call only.
      const clock = this.clock;
      const lift = clock?.paused ?? false;
      if (lift) clock?.resume();
      try {
        this.loop.stepOnce();
      } finally {
        if (lift) clock?.pause();
      }
      return;
    }
    if (this.clock === null) return;
    const wasPaused = this.clock.paused;
    this.clock.resume();
    this.clock.advance(this.stepSeconds);
    if (wasPaused) this.clock.pause();
  }

  /**
   * Runs one game update under the controls. Returns false, and does not call `update`, while the devtools
   * hold the game. A requested step calls it once, with the clock running for that call.
   */
  runUpdate(update: () => void): boolean {
    if (this.frozen && !this.stepRequested) return false;
    const stepping = this.frozen;
    this.stepRequested = false;
    // The clock is held by name, so that the step gives back the clock it lifted, even when the game connects another
    // clock during the update.
    const clock = this.clock;
    const lift = stepping && (clock?.paused ?? false);
    if (lift) clock?.resume();
    try {
      update();
    } finally {
      if (lift) clock?.pause();
    }
    return true;
  }

  /**
   * Sets the clock's speed. This also turns slow motion off.
   * @throws Error when there is no clock, or RangeError when the clock does not allow the speed.
   */
  setSpeed(speed: number): void {
    this.requireClock().setSpeed(speed);
    this.speedBeforeSlow = null;
  }

  /**
   * Turns slow motion on or off. On: the clock runs at the slow speed. Off: it returns to the speed it had.
   * @throws Error when there is no clock, or RangeError when the clock does not allow the slow speed.
   */
  setSlowMotion(on: boolean): void {
    const clock = this.requireClock();
    if (on === (this.speedBeforeSlow !== null)) return;
    if (on) {
      const current = clock.speed;
      clock.setSpeed(this.slowSpeed);
      this.speedBeforeSlow = current;
    } else {
      clock.setSpeed(this.speedBeforeSlow ?? clock.speed);
      this.speedBeforeSlow = null;
    }
  }

  /** The state of the controls, for the Time tab. */
  state(): TimeState {
    const clock = this.clock;
    return {
      hasClock: clock !== null,
      paused: this.frozen || (clock?.paused ?? false),
      speed: clock?.speed ?? null,
      speeds: clock?.speeds ?? [],
      slowMotion: this.speedBeforeSlow !== null,
      slowSpeed: this.slowSpeed,
      slowAllowed: clock !== null && clock.speeds.includes(this.slowSpeed),
    };
  }

  /**
   * Lets go of the clock. Game time runs again when the devtools stopped it, and a slow motion that is on
   * is undone, so that the game is not left at the slow speed.
   */
  dispose(): void {
    this.resume();
    this.releaseClock();
  }

  /**
   * Gives the clock back: game time runs again on it when the tools had stopped it, and a slow motion that is on is
   * undone, so that the game is not left at the slow speed.
   */
  private releaseClock(): void {
    if (this.clockPausedHere) this.clock?.resume();
    this.clockPausedHere = false;
    if (this.speedBeforeSlow !== null) {
      this.clock?.setSpeed(this.speedBeforeSlow);
      this.speedBeforeSlow = null;
    }
  }

  /** The clock, or an error that tells the developer how to give one. */
  private requireClock(): GameClock {
    if (this.clock === null) throw new Error("the devtools have no game clock: pass clock to createDevtools");
    return this.clock;
  }
}
