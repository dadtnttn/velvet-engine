// Synthetic DOM events for the scripted input. Where the browser has the real constructor (happy-dom
// and every browser), the event is built with it. In plain Node there are no keyboard, pointer or
// wheel events, so an ordinary Event carries the same fields as own properties. ActionMap reads only
// those fields, so both forms behave the same for it.

import type { MouseButtonName } from "@velvet/core";

/** The DOM button number of each mouse button, as PointerEvent.button counts them. */
const BUTTON_NUMBERS: Readonly<Record<MouseButtonName, number>> = { left: 0, middle: 1, right: 2 };

/** The DOM button number of a mouse button name. */
export function buttonNumber(button: MouseButtonName): number {
  return BUTTON_NUMBERS[button];
}

/**
 * A keyboard event for a physical key. `code` is what ActionMap reads. A prevented event is marked
 * with preventDefault before it is sent, so that an action map ignores it, as it ignores a press the
 * page has already used.
 */
export function keyEvent(type: "keydown" | "keyup", code: string, prevented: boolean): Event {
  const init = { code, key: code, bubbles: true, cancelable: true };
  const event: Event =
    typeof KeyboardEvent === "function"
      ? new KeyboardEvent(type, init)
      : withFields(new Event(type, init), { code, key: code, repeat: false });
  if (prevented) event.preventDefault();
  return event;
}

/** A pointer event at a client position. `button` is -1 for a move, and `buttons` is the pressed mask. */
export function pointerEvent(
  type: "pointermove" | "pointerdown" | "pointerup",
  client: { readonly x: number; readonly y: number },
  button: number,
  buttons: number,
  prevented: boolean,
): Event {
  const fields = {
    clientX: client.x,
    clientY: client.y,
    button,
    buttons,
    pointerId: 1,
    pointerType: "mouse",
    isPrimary: true,
  };
  const event: Event =
    typeof PointerEvent === "function"
      ? new PointerEvent(type, { ...fields, bubbles: true, cancelable: true })
      : withFields(new Event(type, { bubbles: true, cancelable: true }), fields);
  if (prevented) event.preventDefault();
  return event;
}

/** A wheel event. The deltas and the delta mode are the DOM's: 0 pixels, 1 lines, 2 pages. */
export function wheelEvent(deltaX: number, deltaY: number, deltaMode: number, prevented: boolean): Event {
  const fields = { deltaX, deltaY, deltaMode };
  const event: Event =
    typeof WheelEvent === "function"
      ? new WheelEvent("wheel", { ...fields, bubbles: true, cancelable: true })
      : withFields(new Event("wheel", { bubbles: true, cancelable: true }), fields);
  if (prevented) event.preventDefault();
  return event;
}

/** A blur event, which releases every held control in an action map. */
export function blurEvent(): Event {
  return new Event("blur");
}

/** Adds read-only fields to an event, the way a browser event exposes its own. */
function withFields(event: Event, fields: Readonly<Record<string, unknown>>): Event {
  const descriptors: PropertyDescriptorMap = {};
  for (const [name, value] of Object.entries(fields)) {
    descriptors[name] = { value, enumerable: true };
  }
  return Object.defineProperties(event, descriptors);
}
