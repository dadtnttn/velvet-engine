// Gamepads for the action map: the standard mapping, one reading of every connected pad per
// frame, and the value that a pad control gives from a reading.
//
// The browser's Gamepad objects are copied into plain records at each read, so the rest of the
// input code never keeps a reference to a live pad. A pad that reports another mapping is read
// with the standard indexes anyway: right for most pads, a best effort for the rest.
//
// Deadzones. An axis loses its deadzone and the rest is rescaled, so a full push still reads 1.
// A stick loses a round zone (its length is reduced, its direction kept), so a diagonal push is
// not shortened. A button has no deadzone: a digital button is level when it is pressed, and a
// trigger gives its analog pull.

import type { PadAxisName, PadButtonName, PadControl, PadDirection, PadStickName } from "./input";
import { isRecord } from "./guards";

/** The buttons of the standard mapping, in button-index order: 0 is "a", 15 is "dpadRight". */
export const PAD_BUTTON_NAMES: readonly PadButtonName[] = [
  "a",
  "b",
  "x",
  "y",
  "leftShoulder",
  "rightShoulder",
  "leftTrigger",
  "rightTrigger",
  "select",
  "start",
  "leftStick",
  "rightStick",
  "dpadUp",
  "dpadDown",
  "dpadLeft",
  "dpadRight",
];

/** The axes of the standard mapping, in axis-index order: the left stick, then the right. */
export const PAD_AXIS_NAMES: readonly PadAxisName[] = ["leftX", "leftY", "rightX", "rightY"];

/** The two sticks. */
export const PAD_STICK_NAMES: readonly PadStickName[] = ["left", "right"];

/** The four directions of a stick, for 4-way bindings. */
export const PAD_DIRECTION_NAMES: readonly PadDirection[] = ["up", "down", "left", "right"];

/** The deadzone of an axis or a stick when its binding names none. */
export const DEFAULT_DEADZONE = 0.2;

/** The level at which a pad button, stick direction or axis counts as active by default. */
export const DEFAULT_PAD_THRESHOLD = 0.5;

/** One button as read in a frame. */
export interface PadButtonReading {
  /** The digital pressed flag that the browser reports. */
  readonly pressed: boolean;
  /** The analog value from 0 to 1. Digital buttons report 0 or 1. */
  readonly value: number;
}

/** The parameters of a dual-rumble effect, as the browser's haptics take them. */
export interface RumbleParams {
  /** Milliseconds. */
  readonly duration: number;
  /** The low-frequency motor, from 0 to 1. */
  readonly strongMagnitude: number;
  /** The high-frequency motor, from 0 to 1. */
  readonly weakMagnitude: number;
  readonly startDelay: number;
}

/** A pad's vibration motor. Only a pad that can vibrate has one. */
export interface RumbleActuator {
  /** Plays a dual-rumble effect. The promise rejects when the browser refuses it. */
  playEffect(type: "dual-rumble", params: RumbleParams): Promise<unknown>;
}

/** One connected pad in one frame. */
export interface PadReading {
  /** The pad's index, as the browser numbers the connected pads. */
  readonly index: number;
  /** The pad's name, as the browser reports it. Empty when the browser gives none. */
  readonly id: string;
  /** The buttons, in standard-mapping order. */
  readonly buttons: readonly PadButtonReading[];
  /** The axes, in standard-mapping order. */
  readonly axes: readonly number[];
  /** The vibration motor, or null when the pad cannot vibrate. */
  readonly actuator: RumbleActuator | null;
}

/** Options for {@link rumblePads}. */
export interface RumbleOptions {
  /** The pad's index. Default: the first connected pad that can vibrate. */
  readonly pad?: number;
  /** The length of the effect in milliseconds, from 0 to 5000. Default 200. */
  readonly durationMs?: number;
  /** The low-frequency motor, from 0 to 1. Default 0.5. */
  readonly strong?: number;
  /** The high-frequency motor, from 0 to 1. Default 0.5. */
  readonly weak?: number;
}

/** Clamps a value into [-1, 1]. A NaN becomes 0. */
export function clampUnit(value: number): number {
  if (Number.isNaN(value)) return 0;
  return Math.min(1, Math.max(-1, value));
}

/** Clamps a value into [0, 1]. A NaN becomes 0. */
export function clamp01(value: number): number {
  if (Number.isNaN(value)) return 0;
  return Math.min(1, Math.max(0, value));
}

/**
 * Reads every connected pad now. Returns an empty list where the browser has no gamepad API, or
 * where reading it fails (a page that is not in a secure context, for example).
 */
export function readGamepads(): PadReading[] {
  const nav: unknown = Reflect.get(globalThis, "navigator");
  if (!isRecord(nav)) return [];
  const getGamepads: unknown = Reflect.get(nav, "getGamepads");
  if (typeof getGamepads !== "function") return [];
  let list: unknown;
  try {
    list = Reflect.apply(getGamepads, nav, []);
  } catch {
    return [];
  }
  const entries = readArray(list);
  const readings: PadReading[] = [];
  for (const entry of entries) {
    const reading = readPad(entry);
    if (reading !== null) readings.push(reading);
  }
  return readings;
}

/**
 * The level of a button, from 0 to 1: its analog value, or 1 for a digital button that is pressed.
 * An undefined button (a pad with fewer buttons than the mapping) is at rest.
 */
export function buttonLevel(button: PadButtonReading | undefined): number {
  if (button === undefined) return 0;
  const value = clamp01(button.value);
  if (value > 0) return value;
  return button.pressed ? 1 : 0;
}

/**
 * An axis value with its deadzone removed. Inside the zone the value is 0; outside it the value is
 * rescaled so that the edge of the zone is 0 and a full push is still 1 (or -1).
 */
export function applyAxisDeadzone(value: number, deadzone: number): number {
  const magnitude = Math.abs(value);
  if (magnitude <= deadzone) return 0;
  const scaled = Math.min(1, (magnitude - deadzone) / (1 - deadzone));
  return value < 0 ? -scaled : scaled;
}

/**
 * A stick position with a round deadzone removed. The length is reduced by the deadzone and
 * rescaled, and the direction is kept.
 */
export function applyStickDeadzone(x: number, y: number, deadzone: number): { x: number; y: number } {
  const length = Math.hypot(x, y);
  if (length <= deadzone) return { x: 0, y: 0 };
  const scaled = Math.min(1, (length - deadzone) / (1 - deadzone)) / length;
  return { x: x * scaled, y: y * scaled };
}

/**
 * The value of a pad control in one frame. A button and a stick direction give 0 to 1, and an
 * axis gives -1 to 1 with its sign. A control that names no pad reads every connected pad, and the
 * strongest reading (by magnitude) wins.
 */
export function padValue(pads: readonly PadReading[], control: PadControl): number {
  let best = 0;
  for (const pad of pads) {
    if (control.pad !== undefined && pad.index !== control.pad) continue;
    const reading = readPadControl(pad, control);
    if (Math.abs(reading) > Math.abs(best)) best = reading;
  }
  return best;
}

/**
 * Runs a dual-rumble effect on a pad. Resolves to true when the browser played it, and to false
 * when the pad cannot vibrate, when no such pad is connected, or when the browser refuses. It never
 * rejects.
 */
export async function rumblePads(pads: readonly PadReading[], options: RumbleOptions): Promise<boolean> {
  const pad =
    options.pad === undefined
      ? pads.find((candidate) => candidate.actuator !== null)
      : pads.find((candidate) => candidate.index === options.pad && candidate.actuator !== null);
  const actuator = pad?.actuator ?? null;
  if (actuator === null) return false;
  const params: RumbleParams = {
    duration: finiteOr(options.durationMs, 200, 0, 5000),
    strongMagnitude: finiteOr(options.strong, 0.5, 0, 1),
    weakMagnitude: finiteOr(options.weak, 0.5, 0, 1),
    startDelay: 0,
  };
  try {
    await actuator.playEffect("dual-rumble", params);
    return true;
  } catch {
    return false;
  }
}

/** The value of one pad for one control: the reading of the control's button, axis or stick. */
function readPadControl(pad: PadReading, control: PadControl): number {
  switch (control.type) {
    case "pad-button":
      return buttonLevel(pad.buttons[PAD_BUTTON_NAMES.indexOf(control.button)]);
    case "pad-axis": {
      const raw = pad.axes[PAD_AXIS_NAMES.indexOf(control.axis)] ?? 0;
      return applyAxisDeadzone(raw, control.deadzone ?? DEFAULT_DEADZONE);
    }
    case "pad-stick": {
      const first = control.stick === "left" ? 0 : 2;
      const stick = applyStickDeadzone(pad.axes[first] ?? 0, pad.axes[first + 1] ?? 0, control.deadzone ?? DEFAULT_DEADZONE);
      return stickProjection(stick, control.direction);
    }
  }
}

/** How far a stick points in one direction: 0 when it points the other way, up to 1 when it is full over. */
function stickProjection(stick: { x: number; y: number }, direction: PadDirection): number {
  switch (direction) {
    case "up":
      return Math.max(0, -stick.y);
    case "down":
      return Math.max(0, stick.y);
    case "left":
      return Math.max(0, -stick.x);
    case "right":
      return Math.max(0, stick.x);
  }
}

/** The entries of an array-like value from the browser, or none when it is not an array. */
function readArray(value: unknown): readonly unknown[] {
  return Array.isArray(value) ? value : [];
}

/** Copies one Gamepad into a reading, or returns null for a slot that holds no connected pad. */
function readPad(entry: unknown): PadReading | null {
  if (!isRecord(entry) || Reflect.get(entry, "connected") === false) return null;
  const index: unknown = Reflect.get(entry, "index");
  if (typeof index !== "number" || !Number.isInteger(index) || index < 0) return null;
  const id: unknown = Reflect.get(entry, "id");
  return {
    index,
    id: typeof id === "string" ? id : "",
    buttons: readArray(Reflect.get(entry, "buttons")).map(readButton),
    axes: readArray(Reflect.get(entry, "axes")).map(readAxis),
    actuator: readActuator(Reflect.get(entry, "vibrationActuator")),
  };
}

/** Copies one button. A value that is not a finite number reads as 0. */
function readButton(entry: unknown): PadButtonReading {
  if (!isRecord(entry)) return { pressed: false, value: 0 };
  const value: unknown = Reflect.get(entry, "value");
  return {
    pressed: Reflect.get(entry, "pressed") === true,
    value: typeof value === "number" && Number.isFinite(value) ? clamp01(value) : 0,
  };
}

/** Copies one axis. A value that is not a finite number reads as 0. */
function readAxis(entry: unknown): number {
  return typeof entry === "number" && Number.isFinite(entry) ? clampUnit(entry) : 0;
}

/** Wraps a vibration actuator, or returns null when the object has no playEffect method. */
function readActuator(value: unknown): RumbleActuator | null {
  if (!isRecord(value)) return null;
  const actuator = value;
  const playEffect: unknown = Reflect.get(actuator, "playEffect");
  if (typeof playEffect !== "function") return null;
  return {
    playEffect: (type, params) => {
      const result: unknown = Reflect.apply(playEffect, actuator, [type, params]);
      return Promise.resolve(result);
    },
  };
}

/** A number from an option, kept inside a range, or the fallback when the option is missing or not finite. */
function finiteOr(value: number | undefined, fallback: number, min: number, max: number): number {
  if (value === undefined || !Number.isFinite(value)) return fallback;
  return Math.min(max, Math.max(min, value));
}
