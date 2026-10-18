// The input timeline: a small DSL that puts keys, pointer moves, wheel ticks and gamepad changes on
// frames. Frames in a script count from the frame where the script starts, so one script can be
// played at any point of a run.
//
//   const script = new InputScript();
//   script.at(10).press("Space");                           // down at frame 10, up three frames later
//   script.at(30).click({ x: 960, y: 540 });                // a left click, two frames long
//   script.at(40).drag({ x: 100, y: 100 }, { x: 500, y: 300 }, 30);
//   script.at(90).padButton("a");                           // the A button, pressed for three frames
//   script.at(100).padAxis("leftX", 1, { frames: 20 });     // the stick ramps to full over 20 frames
//   driver.play(script);

import type { MouseButtonName, PadAxisName, PadButtonName } from "@velvet/core";
import type { Driver } from "./driver";
import { TestingError } from "./errors";
import type { PadInit } from "./fake-pads";
import type { StagePoint } from "./stage";

/** Frames that a press holds the key down, when the script names no length. */
const DEFAULT_PRESS_FRAMES = 3;

/** Frames that a click holds the button down, when the script names no length. */
const DEFAULT_CLICK_FRAMES = 2;

/** Frames that a pad button stays pressed, when the script names no length. */
const DEFAULT_PAD_BUTTON_FRAMES = 3;

/** One action of a script: what it does, and the frame where it happens, counted from the script's start. */
export interface ScriptAction {
  readonly frame: number;
  readonly run: (driver: Driver) => void;
}

/** How long a key or a button is held. A press must hold for at least one frame. */
export interface HoldOptions {
  readonly holdFrames?: number;
}

/** Options for {@link ScriptCursor.click}. */
export interface ClickOptions extends HoldOptions {
  readonly button?: MouseButtonName;
}

/** Options for {@link ScriptCursor.drag}. */
export interface DragOptions {
  readonly button?: MouseButtonName;
}

/** Options for {@link ScriptCursor.wheel}. */
export interface WheelStepOptions {
  readonly deltaX?: number;
  readonly deltaMode?: 0 | 1 | 2;
}

/** Options for the pad actions that name a pad. Pad 0 is the default. */
export interface PadOptions {
  readonly pad?: number;
}

/** Options for {@link ScriptCursor.padButton}. */
export interface PadButtonOptions extends PadOptions, HoldOptions {
  /** The level that the button is pressed to, from 0 to 1. Default 1. */
  readonly value?: number;
}

/** Options for {@link ScriptCursor.padAxis}. */
export interface PadAxisOptions extends PadOptions {
  /** Frames over which the axis moves to the value. Default 0, which sets it at once. */
  readonly frames?: number;
}

/** Options for {@link ScriptCursor.padStick}. */
export interface PadStickOptions extends PadOptions {
  /** Frames that the stick stays pushed before it returns to rest. Default 0: it stays pushed. */
  readonly holdFrames?: number;
}

/** Options for {@link ScriptCursor.padConnect}: the pad's index and its initial state. */
export interface PadConnectOptions extends PadOptions, PadInit {}

/**
 * A timeline of input actions. Build it with `at(frame)` and the methods of the cursor it returns,
 * then play it with `driver.play(script)`.
 */
export class InputScript {
  private readonly list: ScriptAction[] = [];

  /**
   * Starts a cursor at a frame, counted from the frame where the script starts.
   * @throws TestingError with code "bad-argument" when the frame is not a whole number from 0 up.
   */
  at(frame: number): ScriptCursor {
    if (!Number.isInteger(frame) || frame < 0) {
      throw new TestingError("bad-argument", `a script frame is a whole number from 0 up, got ${frame}`);
    }
    return new ScriptCursor(this, frame);
  }

  /**
   * Adds an action at a frame. The cursors call this; a game rarely needs it.
   * @throws TestingError with code "bad-argument" when the frame is not a whole number from 0 up.
   */
  add(frame: number, run: (driver: Driver) => void): void {
    if (!Number.isInteger(frame) || frame < 0) {
      throw new TestingError("bad-argument", `a script frame is a whole number from 0 up, got ${frame}`);
    }
    this.list.push({ frame, run });
  }

  /** The number of actions. */
  get size(): number {
    return this.list.length;
  }

  /** The last frame that holds an action, or 0 for an empty script. */
  get endFrame(): number {
    return this.list.reduce((last, action) => Math.max(last, action.frame), 0);
  }

  /** The actions in frame order. Actions on one frame keep the order in which they were added. */
  actions(): readonly ScriptAction[] {
    return [...this.list].sort((left, right) => left.frame - right.frame);
  }
}

/**
 * The place in a script where the next actions go. Every method adds its actions at the cursor's frame,
 * or at a later frame when it spans time, and returns the cursor, so calls can be chained.
 */
export class ScriptCursor {
  constructor(
    private readonly script: InputScript,
    private readonly start: number,
  ) {}

  /** The frame where this cursor sits, counted from the script's start. */
  get frame(): number {
    return this.start;
  }

  /** A cursor at another frame of the same script. */
  at(frame: number): ScriptCursor {
    return this.script.at(frame);
  }

  /** Presses a key down. */
  keyDown(code: string): ScriptCursor {
    return this.act((driver) => driver.keyDown(code));
  }

  /** Lets a key go up. */
  keyUp(code: string): ScriptCursor {
    return this.act((driver) => driver.keyUp(code));
  }

  /** Holds a key down for `holdFrames` frames (3 by default), then lets it go. */
  press(code: string, options: HoldOptions = {}): ScriptCursor {
    const hold = holdFrames(options.holdFrames, DEFAULT_PRESS_FRAMES, 1);
    this.keyDown(code);
    this.script.add(this.start + hold, (driver) => driver.keyUp(code));
    return this;
  }

  /** A quick press: the key goes down and up on consecutive frames. */
  tap(code: string): ScriptCursor {
    return this.press(code, { holdFrames: 1 });
  }

  /** Moves the pointer to a logical position. */
  pointerMove(at: StagePoint): ScriptCursor {
    return this.act((driver) => driver.pointerMove(at.x, at.y));
  }

  /** Presses a mouse button at a logical position, or where the pointer is when no position is given. */
  pointerDown(button: MouseButtonName = "left", at?: StagePoint): ScriptCursor {
    return this.act((driver) => driver.pointerDown(button, at));
  }

  /** Lets a mouse button go up at a logical position, or where the pointer is when no position is given. */
  pointerUp(button: MouseButtonName = "left", at?: StagePoint): ScriptCursor {
    return this.act((driver) => driver.pointerUp(button, at));
  }

  /** Moves to a position, presses the button, and lets it go after `holdFrames` frames (2 by default). */
  click(at: StagePoint, options: ClickOptions = {}): ScriptCursor {
    const button = options.button ?? "left";
    const hold = holdFrames(options.holdFrames, DEFAULT_CLICK_FRAMES, 1);
    this.pointerMove(at);
    this.pointerDown(button, at);
    this.script.add(this.start + hold, (driver) => driver.pointerUp(button, at));
    return this;
  }

  /**
   * Drags from one logical position to another over `frames` frames: the pointer moves to `from`, the
   * button goes down, the pointer moves in even steps to `to`, and the button goes up. The drag takes
   * `frames + 3` frames in all.
   * @throws TestingError with code "bad-argument" when `frames` is not a whole number from 1 up.
   */
  drag(from: StagePoint, to: StagePoint, frames: number, options: DragOptions = {}): ScriptCursor {
    if (!Number.isInteger(frames) || frames < 1) {
      throw new TestingError("bad-argument", `a drag needs a whole number of frames from 1 up, got ${frames}`);
    }
    const button = options.button ?? "left";
    this.pointerMove(from);
    this.script.add(this.start + 1, (driver) => driver.pointerDown(button, from));
    for (let step = 1; step <= frames; step += 1) {
      const fraction = step / frames;
      const at = { x: from.x + (to.x - from.x) * fraction, y: from.y + (to.y - from.y) * fraction };
      this.script.add(this.start + 1 + step, (driver) => driver.pointerMove(at.x, at.y));
    }
    this.script.add(this.start + 2 + frames, (driver) => driver.pointerUp(button, to));
    return this;
  }

  /** One wheel tick: `deltaY` is positive for a wheel that moves down, as in the DOM. */
  wheel(deltaY: number, options: WheelStepOptions = {}): ScriptCursor {
    return this.act((driver) => driver.wheel(deltaY, options));
  }

  /** Presses a pad button to `value` (1 by default) for `holdFrames` frames (3 by default), then releases it. */
  padButton(button: PadButtonName, options: PadButtonOptions = {}): ScriptCursor {
    const pad = options.pad ?? 0;
    const value = options.value ?? 1;
    const hold = holdFrames(options.holdFrames, DEFAULT_PAD_BUTTON_FRAMES, 1);
    this.act((driver) => driver.pads.setButton(pad, button, value));
    this.script.add(this.start + hold, (driver) => driver.pads.setButton(pad, button, 0));
    return this;
  }

  /**
   * Sets an axis, from -1 to 1. With `frames`, the axis moves in even steps from where it is to the value,
   * one step each frame, and reaches the value on the last frame.
   * @throws TestingError with code "bad-argument" when `frames` is not a whole number from 0 up.
   */
  padAxis(axis: PadAxisName, value: number, options: PadAxisOptions = {}): ScriptCursor {
    const pad = options.pad ?? 0;
    const frames = holdFrames(options.frames, 0, 0);
    if (frames === 0) return this.act((driver) => driver.pads.setAxis(pad, axis, value));
    let from = 0;
    this.act((driver) => {
      from = driver.pads.axis(pad, axis);
    });
    for (let step = 1; step <= frames; step += 1) {
      const level = from + (value - from) * (step / frames);
      this.script.add(this.start + step, (driver) => driver.pads.setAxis(pad, axis, level));
    }
    return this;
  }

  /**
   * Pushes a stick to a position, each axis from -1 to 1. With `holdFrames`, the stick returns to rest
   * after that many frames; without it, the stick stays pushed until the script moves it again.
   */
  padStick(stick: "left" | "right", x: number, y: number, options: PadStickOptions = {}): ScriptCursor {
    const pad = options.pad ?? 0;
    const hold = holdFrames(options.holdFrames, 0, 0);
    this.act((driver) => driver.pads.setStick(pad, stick, x, y));
    if (hold > 0) this.script.add(this.start + hold, (driver) => driver.pads.setStick(pad, stick, 0, 0));
    return this;
  }

  /** Connects a pad with its initial state. */
  padConnect(options: PadConnectOptions = {}): ScriptCursor {
    const pad = options.pad ?? 0;
    const init: PadInit = { id: options.id, buttons: options.buttons, axes: options.axes };
    return this.act((driver) => driver.pads.connect(pad, init));
  }

  /** Disconnects a pad. */
  padDisconnect(pad = 0): ScriptCursor {
    return this.act((driver) => driver.pads.disconnect(pad));
  }

  /** A window blur, which releases every control that is held. */
  blur(): ScriptCursor {
    return this.act((driver) => driver.blur());
  }

  /** Runs any code at this frame. It gets the driver, so it can read the game through the test's own references. */
  run(action: (driver: Driver) => void): ScriptCursor {
    return this.act(action);
  }

  /** Adds one action at this cursor's frame. */
  private act(run: (driver: Driver) => void): ScriptCursor {
    this.script.add(this.start, run);
    return this;
  }
}

/**
 * A whole number of frames from an option, or the fallback when the option is missing.
 * @throws TestingError with code "bad-argument" when the value is below `min`.
 */
function holdFrames(value: number | undefined, fallback: number, min: number): number {
  const frames = value ?? fallback;
  if (!Number.isInteger(frames) || frames < min) {
    throw new TestingError("bad-argument", `a length in frames is a whole number from ${min} up, got ${frames}`);
  }
  return frames;
}
