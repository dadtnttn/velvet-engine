// Capturing the next control: the first key, mouse button or pad input that the player presses,
// for a rebinding screen.
//
// Keys and mouse buttons are listened to in the capture phase, so the capture sees them before any
// element or any action map does, and it stops them there: the key that the player is binding does
// not also act in the game. Gamepads are read on their own frames, so a capture works while the
// game's loop is paused. A pad input counts when a button goes from released to pressed, or a stick
// moves from near rest past a clear push. A button or stick that is already down when the capture
// starts does not count until it is released.

import { isHandled } from "./input-controls";
import { PAD_AXIS_NAMES, PAD_BUTTON_NAMES, buttonLevel, readGamepads, type PadReading } from "./input-pads";
import type { FrameScheduler } from "./loop";
import type { CapturedInput, InputDevice, KeyCode, MouseButtonName } from "./input";

/** The level at which a button counts as pressed while capturing. */
const BUTTON_CAPTURE_LEVEL = 0.5;

/** The level at which a stick push counts while capturing. */
const AXIS_CAPTURE_LEVEL = 0.6;

/** A stick that was above this level before the push is not a fresh push. */
const AXIS_REST_LEVEL = 0.3;

/** The mouse buttons by DOM button number: 0 is left, 1 is middle and 2 is right. */
const MOUSE_BUTTONS: readonly MouseButtonName[] = ["left", "middle", "right"];

/** The settings of one capture, after the map has filled in the defaults. */
export interface CaptureSettings {
  /** The devices that may answer. */
  readonly devices: readonly InputDevice[];
  /** Milliseconds before the capture gives up. Infinity never gives up. */
  readonly timeoutMs: number;
  /** Keys that cancel the capture instead of being captured. */
  readonly cancelKeys: readonly KeyCode[];
  /** Aborting it cancels the capture. */
  readonly signal: AbortSignal | undefined;
  /** Where keys and mouse buttons are listened to. Null means no keys and no mouse. */
  readonly target: EventTarget | null;
  /** Where the gamepads are polled, one request per frame. */
  readonly scheduler: FrameScheduler;
}

/** A capture in progress. */
export interface StartedCapture {
  /** Resolves with the captured input, or null when the capture was cancelled, timed out or aborted. */
  readonly result: Promise<CapturedInput | null>;
  /** Cancels the capture. Its result resolves with null. Does nothing after the capture has ended. */
  readonly cancel: () => void;
}

/** Starts a capture. The listeners and the frame requests are removed as soon as it ends. */
export function startCapture(settings: CaptureSettings): StartedCapture {
  if (settings.signal?.aborted === true) {
    return { result: Promise.resolve(null), cancel: () => undefined };
  }

  let finished = false;
  let resolveResult: ((value: CapturedInput | null) => void) | null = null;
  const result = new Promise<CapturedInput | null>((resolve) => {
    resolveResult = resolve;
  });
  const cleanups: Array<() => void> = [];
  let frame: number | null = null;
  let timer: ReturnType<typeof setTimeout> | null = null;

  const finish = (value: CapturedInput | null): void => {
    if (finished) return;
    finished = true;
    for (const cleanup of cleanups.splice(0)) cleanup();
    if (frame !== null) settings.scheduler.cancel(frame);
    if (timer !== null) clearTimeout(timer);
    resolveResult?.(value);
  };

  const target = settings.target;
  if (target !== null && settings.devices.includes("keyboard")) {
    const onKeyDown = (event: Event): void => {
      if (isHandled(event)) return;
      const keyboard = event as KeyboardEvent;
      if (keyboard.repeat === true || typeof keyboard.code !== "string" || keyboard.code === "") return;
      event.preventDefault();
      event.stopPropagation();
      if (settings.cancelKeys.includes(keyboard.code)) {
        finish(null);
        return;
      }
      finish({ device: "keyboard", control: { type: "key", key: keyboard.code } });
    };
    target.addEventListener("keydown", onKeyDown, true);
    cleanups.push(() => target.removeEventListener("keydown", onKeyDown, true));
  }

  if (target !== null && settings.devices.includes("mouse")) {
    const onPointerDown = (event: Event): void => {
      if (isHandled(event)) return;
      const button = MOUSE_BUTTONS[(event as PointerEvent).button];
      if (button === undefined) return;
      event.stopPropagation();
      finish({ device: "mouse", control: { type: "mouse", button } });
    };
    target.addEventListener("pointerdown", onPointerDown, true);
    cleanups.push(() => target.removeEventListener("pointerdown", onPointerDown, true));
  }

  if (settings.devices.includes("gamepad")) {
    let previous: PadReading[] = readGamepads();
    const poll = (): void => {
      frame = null;
      if (finished) return;
      const current = readGamepads();
      const found = detectPadInput(previous, current);
      previous = current;
      if (found !== null) {
        finish(found);
        return;
      }
      frame = settings.scheduler.request(poll);
    };
    frame = settings.scheduler.request(poll);
  }

  if (Number.isFinite(settings.timeoutMs)) {
    timer = setTimeout(() => finish(null), settings.timeoutMs);
  }

  const signal = settings.signal;
  if (signal !== undefined) {
    const onAbort = (): void => finish(null);
    signal.addEventListener("abort", onAbort);
    cleanups.push(() => signal.removeEventListener("abort", onAbort));
  }

  return { result, cancel: () => finish(null) };
}

/**
 * The first pad input that is new since the last reading: a button that was released and is now
 * pressed, which is tried before any stick, or a stick that moved from near rest past the push level.
 */
function detectPadInput(previous: readonly PadReading[], current: readonly PadReading[]): CapturedInput | null {
  for (const pad of current) {
    const before = previous.find((candidate) => candidate.index === pad.index);
    for (let index = 0; index < PAD_BUTTON_NAMES.length; index += 1) {
      const now = buttonLevel(pad.buttons[index]);
      const was = buttonLevel(before?.buttons[index]);
      const button = PAD_BUTTON_NAMES[index];
      if (button !== undefined && now >= BUTTON_CAPTURE_LEVEL && was < BUTTON_CAPTURE_LEVEL) {
        return { device: "gamepad", control: { type: "pad-button", button } };
      }
    }
  }
  for (const pad of current) {
    const before = previous.find((candidate) => candidate.index === pad.index);
    for (let index = 0; index < PAD_AXIS_NAMES.length; index += 1) {
      const value = pad.axes[index] ?? 0;
      const was = before?.axes[index] ?? 0;
      const axis = PAD_AXIS_NAMES[index];
      if (axis !== undefined && Math.abs(value) >= AXIS_CAPTURE_LEVEL && Math.abs(was) < AXIS_REST_LEVEL) {
        return { device: "gamepad", control: { type: "pad-axis", axis }, sign: value < 0 ? -1 : 1 };
      }
    }
  }
  return null;
}
