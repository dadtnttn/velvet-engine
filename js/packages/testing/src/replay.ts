// Replays: a recorded session played back on a driver. The game starts with the seeds and the save of the
// recording, then each frame gets the inputs that were recorded for it, in their order, and the state hash
// is compared with each checkpoint. The first checkpoint that differs ends the replay, and the result names
// its frame, so a change in the game shows up where it first matters.

import { SAVE_FORMAT, type LoadedSave, type SaveEnvelope, type SaveStore } from "@velvet/core";
import { Driver, createDriver } from "./driver";
import { TestingError } from "./errors";
import type { Replay, ReplayInput, ReplaySeed } from "./replay-format";
import type { StageMap } from "./stage";

/** What the game gets when a replay starts: the recording's seeds and save, and the driver to run on. */
export interface ReplayStartContext {
  /** The replay being played. */
  readonly replay: Replay;
  /** The named seeds of the recording. The game must build its random sources from them. */
  readonly seeds: Readonly<Record<string, ReplaySeed>>;
  /** The save that the recording started from, or null for a new game. */
  readonly save: SaveEnvelope | null;
  /** The driver that runs the game. Create the game's loop with `driver.createLoop`, and attach its maps to `driver.target`. */
  readonly driver: Driver;
}

/** A started game, as far as a replay needs it: the hash of its state at a checkpoint. */
export interface ReplayGame {
  /** The hash of the game's state now. It must be the same text that the recording's checkpoints hold. */
  hash(): string;
}

/** Starts the game for a replay. It returns the game's hash function, and it may be asynchronous. */
export type ReplayStart = (context: ReplayStartContext) => ReplayGame | Promise<ReplayGame>;

/** The outcome of a replay. */
export type ReplayResult =
  | {
      /** Every checkpoint matched. */
      readonly ok: true;
      /** The frames that ran. */
      readonly frames: number;
      /** The checkpoints that were compared. */
      readonly checkpoints: number;
    }
  | {
      /** A checkpoint differed. The replay stopped there. */
      readonly ok: false;
      /** The frames of the recording. */
      readonly frames: number;
      /** The first checkpoint frame whose hash differs. */
      readonly frame: number;
      /** The last checkpoint frame that matched, or 0 when none had matched yet. The divergence lies after it. */
      readonly previousFrame: number;
      /** The hash that the recording holds at that frame. */
      readonly expected: string;
      /** The hash that the game gave at that frame. */
      readonly actual: string;
    };

/** Options for {@link startReplay} and {@link checkReplay}. */
export interface ReplayRunOptions {
  /**
   * The driver to run on. Default: a new driver at the replay's rate, which the run disposes when it ends.
   * A driver that is passed in is not disposed by the run.
   */
  readonly driver?: Driver;
  /** The stage for a new driver. Ignored when a driver is passed in. */
  readonly stage?: StageMap;
}

/** A replay that is running. Step it, or finish it, then read the result. */
export interface ReplayRun {
  /** The driver that the game runs on. */
  readonly driver: Driver;
  /** The number of frames that have run. */
  readonly frame: number;
  /** True once the result is known: every frame ran, or a checkpoint differed. */
  readonly done: boolean;
  /** The result, or null while the replay is still running. */
  readonly result: ReplayResult | null;
  /**
   * Runs up to `frames` frames, stopping early at the end of the recording or at a difference. Pages use
   * it to pace a replay, one frame at a time.
   */
  step(frames?: number): void;
  /** Runs the rest of the replay, and returns its result. */
  finish(): ReplayResult;
  /** Stops the listener, and disposes the driver when the run created it. */
  dispose(): void;
}

/**
 * Starts a replay: the game is started with the recording's seeds and save, and the run is ready to step.
 * @throws TestingError with code "bad-argument" when the driver's rate is not the replay's rate, and with
 * the errors of the start function, after which the run is disposed.
 */
export async function startReplay(replay: Replay, start: ReplayStart, options: ReplayRunOptions = {}): Promise<ReplayRun> {
  const owned = options.driver === undefined;
  const driver = options.driver ?? createDriver({ rate: replay.rate, stage: options.stage });
  if (driver.rate !== replay.rate) {
    if (owned) driver.dispose();
    throw new TestingError("bad-argument", `the replay runs at ${replay.rate} steps per second, the driver at ${driver.rate}`);
  }
  if (driver.frame !== 0) {
    if (owned) driver.dispose();
    throw new TestingError(
      "bad-argument",
      `the replay counts its frames from the start: the driver has already run ${driver.frame} frames, so pass a new one`,
    );
  }
  const run = new Player(replay, driver, owned);
  try {
    await run.begin(start);
  } catch (error) {
    run.dispose();
    throw error;
  }
  return run;
}

/**
 * Plays a whole replay and checks it. It is {@link startReplay}, then `finish`, then `dispose`.
 * @returns the result: `ok: true` when every checkpoint matched, or the first checkpoint that did not.
 */
export async function checkReplay(replay: Replay, start: ReplayStart, options: ReplayRunOptions = {}): Promise<ReplayResult> {
  const run = await startReplay(replay, start, options);
  try {
    return run.finish();
  } finally {
    run.dispose();
  }
}

/**
 * The save envelope of a loaded save, for a recording that starts from it. The loaded data is already at
 * the store's version (the store migrates it on load), so the envelope carries the store's version.
 */
export function saveEnvelopeOf(
  store: Pick<SaveStore, "game" | "version">,
  loaded: Pick<LoadedSave, "savedAt" | "data">,
): SaveEnvelope {
  return { format: SAVE_FORMAT, version: store.version, game: store.game, savedAt: loaded.savedAt, data: loaded.data };
}

/** The run itself. It holds the position in the inputs and in the checkpoints. */
class Player implements ReplayRun {
  private game: ReplayGame | null = null;
  private inputIndex = 0;
  private checkIndex = 0;
  private lastMatch = 0;
  private outcome: ReplayResult | null = null;
  private stopListening: (() => void) | null = null;

  constructor(
    private readonly replay: Replay,
    private readonly runDriver: Driver,
    private readonly owned: boolean,
  ) {}

  get driver(): Driver {
    return this.runDriver;
  }

  get frame(): number {
    return this.runDriver.frame;
  }

  get done(): boolean {
    return this.outcome !== null;
  }

  get result(): ReplayResult | null {
    return this.outcome;
  }

  /** Subscribes to the frames, then starts the game. The subscription comes first, so no frame is missed. */
  async begin(start: ReplayStart): Promise<void> {
    this.stopListening = this.runDriver.addListener({
      beforeTick: (frame) => this.applyInputsUpTo(frame),
      afterTick: (frame) => this.checkAfter(frame),
    });
    this.game = await start({
      replay: this.replay,
      seeds: this.replay.seeds,
      save: this.replay.save,
      driver: this.runDriver,
    });
  }

  step(frames = 1): void {
    if (!Number.isInteger(frames) || frames < 0) {
      throw new TestingError("bad-argument", `step needs a whole number of frames from 0 up, got ${frames}`);
    }
    for (let done = 0; done < frames; done += 1) {
      if (this.outcome !== null) return;
      if (this.runDriver.frame >= this.replay.frames) {
        this.conclude();
        return;
      }
      this.runDriver.step(1);
    }
    if (this.outcome === null && this.runDriver.frame >= this.replay.frames) this.conclude();
  }

  finish(): ReplayResult {
    // Each step runs a frame or concludes the replay, so this loop always ends.
    while (this.outcome === null) this.step(1);
    const result = this.outcome;
    if (result === null) throw new TestingError("bad-argument", "the replay did not finish");
    return result;
  }

  dispose(): void {
    this.stopListening?.();
    this.stopListening = null;
    if (this.owned) this.runDriver.dispose();
  }

  /** Feeds the recorded inputs of one frame, in order. Inputs at or before the frame are fed, so none is skipped. */
  private applyInputsUpTo(frame: number): void {
    const inputs = this.replay.inputs;
    while (this.inputIndex < inputs.length) {
      const input = inputs[this.inputIndex];
      if (input === undefined || input.frame > frame) break;
      this.inputIndex += 1;
      this.apply(input);
    }
  }

  /** Puts one recorded input into the driver, the way the player's own input went in. */
  private apply(input: ReplayInput): void {
    const driver = this.runDriver;
    switch (input.type) {
      case "key":
        if (input.down) driver.keyDown(input.code, { skipped: input.skipped === true });
        else driver.keyUp(input.code);
        return;
      case "button":
        if (input.down) driver.pointerDown(input.button, { x: input.x, y: input.y }, { skipped: input.skipped === true });
        else driver.pointerUp(input.button, { x: input.x, y: input.y });
        return;
      case "pointer":
        driver.pointerMove(input.x, input.y);
        return;
      case "wheel":
        driver.wheel(input.deltaY, { deltaX: input.deltaX, deltaMode: input.deltaMode, skipped: input.skipped === true });
        return;
      case "pad":
        if (input.connected) driver.pads.connect(input.pad, { id: input.id, buttons: input.buttons, axes: input.axes });
        else driver.pads.disconnect(input.pad);
        return;
      case "blur":
        driver.blur();
        return;
    }
  }

  /** After a frame: if it ends at a checkpoint, the hash must match. A mismatch ends the replay. */
  private checkAfter(frame: number): void {
    const completed = frame + 1;
    const expected = this.replay.checkpoints[this.checkIndex];
    if (expected === undefined || expected.frame !== completed || this.game === null) return;
    this.checkIndex += 1;
    const actual = this.game.hash();
    if (actual === expected.hash) {
      this.lastMatch = completed;
      return;
    }
    this.outcome = {
      ok: false,
      frames: this.replay.frames,
      frame: completed,
      previousFrame: this.lastMatch,
      expected: expected.hash,
      actual,
    };
  }

  /** The last frame has run: the inputs recorded after it are fed, and the replay is judged passed. */
  private conclude(): void {
    if (this.outcome !== null) return;
    this.applyInputsUpTo(this.replay.frames);
    this.outcome = { ok: true, frames: this.replay.frames, checkpoints: this.replay.checkpoints.length };
  }
}
