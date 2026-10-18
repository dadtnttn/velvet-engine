// The recorder keeps a session as a replay document. It can listen to a driver (in a test, or in a bot
// run), or it can wrap the update of a live loop (in a page). Either way it samples the gamepads once per
// frame, just before the update, so the pad states it writes are the ones that the game's action maps
// read during that frame. Pages record only trusted events: the events that the browser makes from real
// input. The events that the game or the page script makes itself are not part of the player's session.

import type { GameLoopOptions, SaveEnvelope, Unsubscribe } from "@velvet/core";
import type { Driver, DriverListener } from "./driver";
import { TestingError } from "./errors";
import { readNavigatorPads, type NavigatorPad } from "./fake-pads";
import {
  DEFAULT_CHECK_EVERY,
  REPLAY_FORMAT,
  REPLAY_VERSION,
  parseReplay,
  type Replay,
  type ReplayCheckpoint,
  type ReplayEvent,
  type ReplayInput,
  type ReplaySeed,
} from "./replay-format";
import { clientToLogical, createStageMap, type StageMap, type StagePoint } from "./stage";

/** Options for {@link createRecorder}. */
export interface RecorderOptions {
  /** The id of the game. Replays are played by the same game. */
  readonly game: string;
  /** The game's version, written into the replay for reading only. */
  readonly gameVersion?: string;
  /** The loop's update steps per second. Default 60. It must match the rate the session runs at. */
  readonly rate?: number;
  /** The named seeds at the start of the session, such as `{ board: 42 }`. */
  readonly seeds?: Readonly<Record<string, ReplaySeed>>;
  /** The save that the session starts from, in the save file format, or null for a new game. */
  readonly save?: SaveEnvelope | null;
  /** Frames between checkpoints. Default 60 (one second at 60 Hz). */
  readonly checkEvery?: number;
  /**
   * The game's state hash, taken at each checkpoint. Without it the replay has no checkpoints, and a
   * replay can only be run, not checked.
   */
  readonly hash?: () => string;
  /** How page positions become logical ones, for `observe`. Default: 1920 by 1080 at the origin. */
  readonly stage?: StageMap;
}

/** A recorder. Create it with {@link createRecorder}. */
export interface Recorder {
  /** The number of frames recorded so far: the index of the next update. */
  readonly frames: number;
  /**
   * Records the frames and inputs of a driver. The driver's rate must equal the recorder's rate.
   * @returns a function that stops the recording.
   */
  attach(driver: Driver): Unsubscribe;
  /**
   * Returns the options of a live loop with its update wrapped, so that every frame is recorded and
   * checked. Build the GameLoop from the returned options.
   */
  wrap(options: GameLoopOptions): GameLoopOptions;
  /**
   * Records the trusted keys, pointer, wheel and blur events that reach `target` (usually `window`). Use it
   * with {@link Recorder.wrap}, because the frames come from the wrapped loop.
   * @returns a function that stops the recording.
   */
  observe(target: EventTarget): Unsubscribe;
  /** Records one input at the current frame. */
  log(event: ReplayEvent): void;
  /**
   * The replay so far, checked by `parseReplay`. It can be written with `serializeReplay`.
   * @throws TestingError with code "bad-replay" when the options or the inputs do not form a valid replay.
   */
  toReplay(): Replay;
}

/** The control selector of the action map: a press whose target is inside one of these does not count. */
const CONTROL_SELECTOR =
  'input, textarea, select, button, [contenteditable]:not([contenteditable="false"]), [role="menuitem"]';

/** The mouse buttons by DOM button number. */
const BUTTON_NAMES: Readonly<Record<number, "left" | "middle" | "right">> = { 0: "left", 1: "middle", 2: "right" };

/** Creates a recorder. Use one per session. */
export function createRecorder(options: RecorderOptions): Recorder {
  return new SessionRecorder(options);
}

/** The recorder itself. Its methods are the ones of the {@link Recorder} interface. */
class SessionRecorder implements Recorder {
  private readonly inputs: ReplayInput[] = [];
  private readonly checkpoints: ReplayCheckpoint[] = [];
  private readonly padTexts = new Map<number, string>();
  private readonly rate: number;
  private readonly checkEvery: number;
  private readonly stage: StageMap;
  private completed = 0;

  constructor(private readonly options: RecorderOptions) {
    this.rate = options.rate ?? 60;
    this.checkEvery = options.checkEvery ?? DEFAULT_CHECK_EVERY;
    this.stage = options.stage ?? createStageMap();
    if (!Number.isInteger(this.checkEvery) || this.checkEvery < 1) {
      throw new TestingError("bad-argument", `checkEvery must be a whole number from 1 up, got ${this.checkEvery}`);
    }
    if (!Number.isFinite(this.rate) || this.rate <= 0) {
      throw new TestingError("bad-argument", `rate must be above 0, got ${this.rate}`);
    }
  }

  get frames(): number {
    return this.completed;
  }

  attach(driver: Driver): Unsubscribe {
    if (driver.rate !== this.rate) {
      throw new TestingError("bad-argument", `the driver runs at ${driver.rate} steps per second, the recorder at ${this.rate}`);
    }
    const listener: DriverListener = {
      beforeTick: (frame) => this.beforeTick(frame),
      afterTick: (frame) => this.afterTick(frame),
    };
    const stopTicks = driver.addListener(listener);
    const stopInputs = driver.onInput((event) => this.log(event));
    return () => {
      stopTicks();
      stopInputs();
    };
  }

  wrap(options: GameLoopOptions): GameLoopOptions {
    return {
      ...options,
      update: (dt: number) => {
        const frame = this.completed;
        this.beforeTick(frame);
        options.update(dt);
        this.afterTick(frame);
      },
    };
  }

  observe(target: EventTarget): Unsubscribe {
    const handlers: Array<[string, (event: Event) => void]> = [
      ["keydown", (event) => this.observeKey(event, true)],
      ["keyup", (event) => this.observeKey(event, false)],
      ["pointerdown", (event) => this.observeButton(event, true)],
      ["pointerup", (event) => this.observeButton(event, false)],
      ["pointermove", (event) => this.observePointer(event)],
      ["wheel", (event) => this.observeWheel(event)],
      ["blur", (event) => this.observeBlur(event)],
    ];
    for (const [type, handler] of handlers) target.addEventListener(type, handler);
    return () => {
      for (const [type, handler] of handlers) target.removeEventListener(type, handler);
    };
  }

  log(event: ReplayEvent): void {
    this.inputs.push({ frame: this.completed, ...event });
  }

  toReplay(): Replay {
    const options = this.options;
    const seeds: Record<string, ReplaySeed> = { ...(options.seeds ?? {}) };
    const document: Record<string, unknown> = {
      format: REPLAY_FORMAT,
      version: REPLAY_VERSION,
      game: options.game,
      ...(options.gameVersion === undefined ? {} : { gameVersion: options.gameVersion }),
      rate: this.rate,
      frames: this.completed,
      seeds,
      save: options.save ?? null,
      checkEvery: this.checkEvery,
      checkpoints: this.checkpoints.map((checkpoint) => ({ ...checkpoint })),
      inputs: this.inputs.map((input) => ({ ...input })),
    };
    return parseReplay(document);
  }

  /** Before an update: the pads as the game will read them. */
  private beforeTick(frame: number): void {
    this.completed = frame;
    this.sampleGamepads();
  }

  /** After an update: the frame count moves on, and a checkpoint is taken at each interval. */
  private afterTick(frame: number): void {
    this.completed = frame + 1;
    if (this.options.hash === undefined || this.completed % this.checkEvery !== 0) return;
    const hash: unknown = this.options.hash();
    if (typeof hash !== "string") {
      throw new TestingError("bad-argument", "the game's hash() must return a text");
    }
    this.checkpoints.push({ frame: this.completed, hash });
  }

  /** Logs the pads that changed since the last frame: a new pad, a change of state, or a disconnection. */
  private sampleGamepads(): void {
    const current = new Map<number, NavigatorPad>();
    for (const pad of readNavigatorPads()) current.set(pad.index, pad);
    const indexes = [...new Set([...this.padTexts.keys(), ...current.keys()])].sort((left, right) => left - right);
    for (const index of indexes) {
      const pad = current.get(index);
      if (pad === undefined) {
        this.padTexts.delete(index);
        this.log({ type: "pad", pad: index, connected: false });
        continue;
      }
      const text = JSON.stringify({ id: pad.id, buttons: pad.buttons, axes: pad.axes });
      if (this.padTexts.get(index) === text) continue;
      this.padTexts.set(index, text);
      this.log({ type: "pad", pad: index, connected: true, id: pad.id, buttons: pad.buttons, axes: pad.axes });
    }
  }

  /** A key from a trusted event. Repeats are left out, since an action map counts only the first press. */
  private observeKey(event: Event, down: boolean): void {
    if (!event.isTrusted) return;
    const keyboard = event as KeyboardEvent;
    if (typeof keyboard.code !== "string" || !/^[A-Za-z0-9]{1,32}$/.test(keyboard.code)) return;
    if (down && keyboard.repeat === true) return;
    const skipped = down && isSkipped(event);
    this.log({ type: "key", code: keyboard.code, down, ...(skipped ? { skipped: true as const } : {}) });
  }

  /** A mouse button from a trusted event, with the pointer position at that moment. */
  private observeButton(event: Event, down: boolean): void {
    if (!event.isTrusted) return;
    const pointer = event as PointerEvent;
    const button = BUTTON_NAMES[pointer.button];
    const point = this.toLogical(pointer);
    if (button === undefined || point === null) return;
    const skipped = down && isSkipped(event);
    this.log({ type: "button", button, down, x: point.x, y: point.y, ...(skipped ? { skipped: true as const } : {}) });
  }

  /** A pointer move from a trusted event. */
  private observePointer(event: Event): void {
    if (!event.isTrusted) return;
    const point = this.toLogical(event as PointerEvent);
    if (point !== null) this.log({ type: "pointer", x: point.x, y: point.y });
  }

  /** A wheel tick from a trusted event. */
  private observeWheel(event: Event): void {
    if (!event.isTrusted) return;
    const wheel = event as WheelEvent;
    if (wheel.deltaMode !== 0 && wheel.deltaMode !== 1 && wheel.deltaMode !== 2) return;
    const skipped = isSkipped(event);
    this.log({
      type: "wheel",
      deltaX: wheel.deltaX,
      deltaY: wheel.deltaY,
      deltaMode: wheel.deltaMode,
      ...(skipped ? { skipped: true as const } : {}),
    });
  }

  /** A window blur from a trusted event. */
  private observeBlur(event: Event): void {
    if (event.isTrusted) this.log({ type: "blur" });
  }

  /** The logical position of a pointer event, or null when the stage has no size on screen. */
  private toLogical(event: MouseEvent): StagePoint | null {
    try {
      return clientToLogical(this.stage, event.clientX, event.clientY);
    } catch {
      return null;
    }
  }
}

/**
 * True when the press must not count, as the action map sees it: the event's default was prevented
 * before the window saw it, or its target is a control that keeps its own keys.
 */
function isSkipped(event: Event): boolean {
  return event.defaultPrevented || isControlTarget(event.target);
}

/** True when the target is, or is inside, a control. It is false where there is no DOM. */
function isControlTarget(target: EventTarget | null): boolean {
  if (typeof Element === "undefined" || !(target instanceof Element)) return false;
  return target.closest(CONTROL_SELECTOR) !== null;
}
