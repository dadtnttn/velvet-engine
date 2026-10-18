// Gamepad menu keys: while a menu is open, the pad's directions, A and B press the keys that a
// keyboard would press (the arrows, Enter and Escape), so that menus which follow the keyboard focus
// rules (see @velvet/ui) work with a pad too.
//
// The keys go to the focused element. They are marked handled, so that no action map counts them as
// the game's own presses, and a capture does not take them as bindings. A held arrow repeats after a
// short delay, as a held key does. A synthetic Enter does not press a button by itself, so this
// clicks a focused button, which is what the browser does for a real Enter.

import { markHandled } from "./input-controls";
import { DEFAULT_PAD_THRESHOLD, padValue, readGamepads } from "./input-pads";
import { createFrameScheduler, type FrameScheduler } from "./loop";
import type { KeyCode, PadControl } from "./input";

/** A pad control and the key that it presses. */
export interface PadKeyMapping {
  readonly control: PadControl;
  readonly key: KeyCode;
}

/** Options for {@link startPadMenuKeys}. */
export interface PadMenuKeysOptions {
  /**
   * The controls and their keys. Default: the d-pad and the left stick press the arrows, A presses
   * Enter and B presses Escape.
   */
  readonly mappings?: readonly PadKeyMapping[];
  /** Where the frames come from. Default: the browser's frames, or timers where there are none. */
  readonly scheduler?: FrameScheduler;
  /** Where the keys go. Default: the focused element, or the body when nothing has the focus. */
  readonly target?: () => EventTarget | null;
}

/** Milliseconds that a held arrow waits before it repeats, and the gap between repeats. */
const REPEAT_DELAY_MS = 400;
const REPEAT_INTERVAL_MS = 100;

/** The default mapping. */
const DEFAULT_MAPPINGS: readonly PadKeyMapping[] = [
  { control: { type: "pad-button", button: "dpadUp" }, key: "ArrowUp" },
  { control: { type: "pad-button", button: "dpadDown" }, key: "ArrowDown" },
  { control: { type: "pad-button", button: "dpadLeft" }, key: "ArrowLeft" },
  { control: { type: "pad-button", button: "dpadRight" }, key: "ArrowRight" },
  { control: { type: "pad-stick", stick: "left", direction: "up" }, key: "ArrowUp" },
  { control: { type: "pad-stick", stick: "left", direction: "down" }, key: "ArrowDown" },
  { control: { type: "pad-stick", stick: "left", direction: "left" }, key: "ArrowLeft" },
  { control: { type: "pad-stick", stick: "left", direction: "right" }, key: "ArrowRight" },
  { control: { type: "pad-button", button: "a" }, key: "Enter" },
  { control: { type: "pad-button", button: "b" }, key: "Escape" },
];

/** When a key went down, and when it last repeated, in milliseconds of frame time. */
interface HeldKey {
  since: number;
  last: number;
}

/**
 * Starts forwarding the gamepad to the focused element, as keys. Returns the function that stops it.
 * Stopping releases the keys that are still held.
 */
export function startPadMenuKeys(options: PadMenuKeysOptions = {}): () => void {
  const mappings = options.mappings ?? DEFAULT_MAPPINGS;
  const scheduler = options.scheduler ?? createFrameScheduler();
  const target = options.target ?? focusedTarget;
  const held = new Map<KeyCode, HeldKey>();
  let frame: number | null = null;
  let stopped = false;

  const tick = (timeMs: number): void => {
    frame = null;
    if (stopped) return;
    const pads = readGamepads();
    const down = new Set<KeyCode>();
    for (const mapping of mappings) {
      const level = padValue(pads, mapping.control);
      if (level >= (mapping.control.threshold ?? DEFAULT_PAD_THRESHOLD)) down.add(mapping.key);
    }
    for (const key of down) {
      if (held.has(key)) continue;
      held.set(key, { since: timeMs, last: timeMs });
      dispatch(target(), key, "keydown", false);
    }
    for (const [key, state] of [...held]) {
      if (!down.has(key)) {
        held.delete(key);
        dispatch(target(), key, "keyup", false);
      } else if (isArrow(key) && timeMs - state.since >= REPEAT_DELAY_MS && timeMs - state.last >= REPEAT_INTERVAL_MS) {
        state.last = timeMs;
        dispatch(target(), key, "keydown", true);
      }
    }
    frame = scheduler.request(tick);
  };
  frame = scheduler.request(tick);

  return () => {
    if (stopped) return;
    stopped = true;
    if (frame !== null) scheduler.cancel(frame);
    for (const key of held.keys()) dispatch(target(), key, "keyup", false);
    held.clear();
  };
}

/** Sends one key event to the target. The event is marked handled, and Enter clicks a button. */
function dispatch(destination: EventTarget | null, key: KeyCode, type: "keydown" | "keyup", repeat: boolean): void {
  if (destination === null || typeof KeyboardEvent === "undefined") return;
  const event = new KeyboardEvent(type, { key, code: key, bubbles: true, cancelable: true, repeat });
  markHandled(event);
  destination.dispatchEvent(event);
  if (type === "keydown" && key === "Enter" && typeof HTMLButtonElement !== "undefined" && destination instanceof HTMLButtonElement) {
    destination.click();
  }
}

/** True for an arrow key, which repeats while it is held. */
function isArrow(key: KeyCode): boolean {
  return key.startsWith("Arrow");
}

/** The element that has the focus, or the body when none has it. Null where there is no document. */
function focusedTarget(): EventTarget | null {
  if (typeof document === "undefined") return null;
  return document.activeElement ?? document.body;
}
