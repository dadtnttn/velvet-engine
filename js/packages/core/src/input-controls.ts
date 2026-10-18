// Helpers for controls and bindings: the key that names a control, the display data that a UI
// shows for it, the strict reading of a control from untrusted data (a settings file), and the
// mark that lets a UI use an event before the action map counts it.

import { PAD_AXIS_NAMES, PAD_BUTTON_NAMES, PAD_DIRECTION_NAMES, PAD_STICK_NAMES } from "./input-pads";
import { isRecord } from "./guards";
import type { BindingDisplay, Control, MouseButtonName, PadControl, WheelDirection } from "./input";

/** The mouse buttons that a binding can name. */
const MOUSE_BUTTONS: readonly MouseButtonName[] = ["left", "middle", "right"];

/** The wheel directions that a binding can name. */
const WHEEL_DIRECTIONS: readonly WheelDirection[] = ["up", "down", "left", "right"];

/** A KeyboardEvent.code value: letters and digits only, as every standard key name is. */
const KEY_PATTERN = /^[A-Za-z0-9]{1,32}$/;

/** The events that a UI has used. No action map counts a press that is in this set. */
const handledEvents = new WeakSet<Event>();

/**
 * Marks an event as used by the UI, so that no ActionMap counts it as a press. Use it in a handler
 * that acts on a key when it should not call preventDefault, which might cancel an action the
 * browser needs. Releases of the key still count.
 */
export function markHandled(event: Event): void {
  handledEvents.add(event);
}

/** True when {@link markHandled} was called for the event. */
export function isHandled(event: Event): boolean {
  return handledEvents.has(event);
}

/**
 * The key that names a control. Two bindings with the same key are bound to the same control. A
 * pad control names its pad, with "*" for any pad, so pad 0 and any pad are different controls.
 */
export function controlKey(control: Control): string {
  switch (control.type) {
    case "key":
      return `key:${control.key}`;
    case "mouse":
      return `mouse:${control.button}`;
    case "wheel":
      return `wheel:${control.direction}`;
    case "pad-button":
      return `pad-button:${padPart(control)}:${control.button}`;
    case "pad-axis":
      return `pad-axis:${padPart(control)}:${control.axis}`;
    case "pad-stick":
      return `pad-stick:${padPart(control)}:${control.stick}:${control.direction}`;
  }
}

/**
 * The key that conflicts are checked by. An axis pushed one way is a different control from the
 * other way, so an axis control includes the sign of its binding's scale.
 */
export function conflictKeyOf(control: Control, scale: number): string {
  const key = controlKey(control);
  return control.type === "pad-axis" ? `${key}:${scale < 0 ? "-" : "+"}` : key;
}

/**
 * Display data for a binding: the device it belongs to, a short code, and the i18n key ids that a UI
 * looks up for its name (`input.key.Space`, `input.pad.a`, and so on). The game supplies the texts
 * for these ids in its locale catalogs. `negative` is set on an axis whose binding is inverted.
 */
export function describeBinding(binding: { readonly control: Control; readonly scale?: number }): BindingDisplay {
  const { control } = binding;
  switch (control.type) {
    case "key":
      return { device: "keyboard", code: control.key, labelKey: `input.key.${control.key}` };
    case "mouse":
      return { device: "mouse", code: control.button, labelKey: `input.mouse.${control.button}` };
    case "wheel":
      return { device: "mouse", code: control.direction, labelKey: `input.wheel.${control.direction}` };
    case "pad-button":
      return { device: "gamepad", code: control.button, labelKey: `input.pad.${control.button}` };
    case "pad-axis": {
      const display: BindingDisplay = { device: "gamepad", code: control.axis, labelKey: `input.pad.axis.${control.axis}` };
      return (binding.scale ?? 1) < 0 ? { ...display, negative: true } : display;
    }
    case "pad-stick":
      return {
        device: "gamepad",
        code: `${control.stick}.${control.direction}`,
        labelKey: `input.pad.stick.${control.stick}.${control.direction}`,
      };
  }
}

/**
 * Reads a control from a value that came from outside the program, such as a settings file. The
 * result is a new object with only the fields that the control type has.
 * @throws RangeError when the value is not a control, or a field is out of range. The message says
 * which field.
 */
export function readControl(value: unknown): Control {
  if (!isRecord(value)) throw new RangeError("a control must be an object");
  switch (value.type) {
    case "key": {
      const key = value.key;
      if (typeof key !== "string" || !KEY_PATTERN.test(key)) {
        throw new RangeError(`key must be 1 to 32 letters or digits, got ${String(key)}`);
      }
      return { type: "key", key };
    }
    case "mouse":
      return { type: "mouse", button: oneOf(value.button, MOUSE_BUTTONS, "mouse button") };
    case "wheel":
      return { type: "wheel", direction: oneOf(value.direction, WHEEL_DIRECTIONS, "wheel direction") };
    case "pad-button":
      return {
        type: "pad-button",
        button: oneOf(value.button, PAD_BUTTON_NAMES, "pad button"),
        pad: padIndex(value.pad),
        threshold: threshold(value.threshold),
      };
    case "pad-axis":
      return {
        type: "pad-axis",
        axis: oneOf(value.axis, PAD_AXIS_NAMES, "pad axis"),
        pad: padIndex(value.pad),
        deadzone: deadzone(value.deadzone),
        threshold: threshold(value.threshold),
      };
    case "pad-stick":
      return {
        type: "pad-stick",
        stick: oneOf(value.stick, PAD_STICK_NAMES, "stick"),
        direction: oneOf(value.direction, PAD_DIRECTION_NAMES, "stick direction"),
        pad: padIndex(value.pad),
        deadzone: deadzone(value.deadzone),
        threshold: threshold(value.threshold),
      };
    default:
      throw new RangeError(`unknown control type ${String(value.type)}`);
  }
}

/** The part of a pad control's key that names the pad. */
function padPart(control: PadControl): string {
  return control.pad === undefined ? "*" : String(control.pad);
}

/** The member of `allowed` that equals `value`. */
function oneOf<T extends string>(value: unknown, allowed: readonly T[], what: string): T {
  const found = allowed.find((candidate) => candidate === value);
  if (found === undefined) throw new RangeError(`${what} must be one of ${allowed.join(", ")}, got ${String(value)}`);
  return found;
}

/** The pad index of a control: undefined for any pad, else a whole number from 0 up. */
function padIndex(value: unknown): number | undefined {
  if (value === undefined) return undefined;
  if (typeof value !== "number" || !Number.isInteger(value) || value < 0) {
    throw new RangeError(`pad must be a whole number from 0 up, got ${String(value)}`);
  }
  return value;
}

/** The deadzone of an axis or a stick: undefined for the default, else a number from 0 up to below 1. */
function deadzone(value: unknown): number | undefined {
  if (value === undefined) return undefined;
  if (typeof value !== "number" || !Number.isFinite(value) || value < 0 || value >= 1) {
    throw new RangeError(`deadzone must be at least 0 and below 1, got ${String(value)}`);
  }
  return value;
}

/** The threshold of a pad control: undefined for the default, else a number above 0 up to 1. */
function threshold(value: unknown): number | undefined {
  if (value === undefined) return undefined;
  if (typeof value !== "number" || !Number.isFinite(value) || value <= 0 || value > 1) {
    throw new RangeError(`threshold must be above 0 and at most 1, got ${String(value)}`);
  }
  return value;
}
