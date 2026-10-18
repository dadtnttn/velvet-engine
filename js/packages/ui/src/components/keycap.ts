// A keycap: a key drawn like a keyboard key, for hints such as "E" or "Space". It can also draw
// a small mouse with one part lit, for the mouse buttons and the wheel.
//
// The key is given as a KeyboardEvent.code ("KeyE", "Space", "ArrowUp"), which is shown with its
// usual label, or as plain text ("E"), which is shown as it is. "MouseLeft", "MouseRight",
// "MouseMiddle" and "MouseWheel" draw the mouse.

import { mergeDefined, type VuComponent } from "./component";
import { createElement, createSvgElement } from "./dom";

/**
 * How the text of a keycap is cased. "upper" draws capitals, as a keyboard's keys are drawn, and is the default.
 * "as-is" keeps the case that the text is written in, so that "Esc" and "Espacio" show as written.
 */
export type KeycapTextCase = "upper" | "as-is";

/** Props of a keycap. */
export interface KeycapProps {
  /** A KeyboardEvent.code, a label, or one of the mouse names: "MouseLeft", "MouseRight", "MouseMiddle", "MouseWheel". */
  readonly key: string;
  /** How the text is cased. Default "upper". A mouse key has no text, so it ignores this. */
  readonly textCase?: KeycapTextCase;
}

/** Labels for keys whose names are too long to show as they are. */
const KEY_LABELS: ReadonlyMap<string, string> = new Map([
  ["Space", "Space"],
  ["Enter", "Enter"],
  ["NumpadEnter", "Enter"],
  ["Escape", "Esc"],
  ["Tab", "Tab"],
  ["Backspace", "Bksp"],
  ["ShiftLeft", "Shift"],
  ["ShiftRight", "Shift"],
  ["ControlLeft", "Ctrl"],
  ["ControlRight", "Ctrl"],
  ["AltLeft", "Alt"],
  ["AltRight", "Alt"],
  ["MetaLeft", "Win"],
  ["MetaRight", "Win"],
  ["CapsLock", "Caps"],
  ["Delete", "Del"],
  ["Insert", "Ins"],
  ["PageUp", "PgUp"],
  ["PageDown", "PgDn"],
  ["ArrowUp", "↑"],
  ["ArrowDown", "↓"],
  ["ArrowLeft", "←"],
  ["ArrowRight", "→"],
]);

/** Which part of the mouse drawing is lit for each mouse name, and what a screen reader says. */
interface MouseKey {
  readonly part: "left" | "right" | "wheel";
  readonly name: string;
}

const MOUSE_KEYS: ReadonlyMap<string, MouseKey> = new Map([
  ["MouseLeft", { part: "left", name: "Left mouse button" }],
  ["MouseRight", { part: "right", name: "Right mouse button" }],
  ["MouseMiddle", { part: "wheel", name: "Middle mouse button" }],
  ["MouseWheel", { part: "wheel", name: "Mouse wheel" }],
]);

/**
 * The label that a keycap shows for a key. A KeyboardEvent.code gets its usual label: "KeyE"
 * gives "E", "Digit1" gives "1", "Space" gives "Space". Anything else is shown as it is.
 */
export function keyLabel(code: string): string {
  const letter = /^Key([A-Z])$/.exec(code)?.[1];
  if (letter !== undefined) return letter;
  const digit = /^(?:Digit|Numpad)([0-9])$/.exec(code)?.[1];
  if (digit !== undefined) return digit;
  return KEY_LABELS.get(code) ?? code;
}

/**
 * Creates a keycap.
 * @param props - the starting props.
 */
export function createKeycap(props: KeycapProps): VuComponent<KeycapProps> {
  const el = createElement("kbd", "vu-keycap");
  let current: KeycapProps = { ...props };

  const render = (): void => {
    el.replaceChildren();
    // The stylesheet capitalises a keycap by default; this class turns that off for the text as written.
    el.classList.toggle("vu-keycap-as-is", current.textCase === "as-is");
    const mouse = MOUSE_KEYS.get(current.key);
    if (mouse !== undefined) {
      el.classList.add("vu-keycap-mouse");
      el.setAttribute("role", "img");
      el.setAttribute("aria-label", mouse.name);
      el.append(mouseIcon(mouse.part));
    } else {
      el.classList.remove("vu-keycap-mouse");
      el.removeAttribute("role");
      el.removeAttribute("aria-label");
      el.textContent = keyLabel(current.key);
    }
  };
  render();

  return {
    el,
    update(next: Partial<KeycapProps>): void {
      current = mergeDefined(current, next);
      render();
    },
    destroy(): void {
      el.remove();
    },
  };
}

/** A small mouse drawn in SVG, with the part that the key names lit. */
function mouseIcon(lit: MouseKey["part"]): SVGElement {
  const svg = createSvgElement("svg");
  svg.setAttribute("class", "vu-mouse");
  svg.setAttribute("viewBox", "0 0 24 34");
  svg.setAttribute("aria-hidden", "true");
  svg.append(
    shape("rect", { x: "1", y: "1", width: "22", height: "32", rx: "11" }, "vu-mouse-body"),
    shape("rect", { x: "3", y: "3", width: "8.5", height: "12" }, partClass("left", lit)),
    shape("rect", { x: "12.5", y: "3", width: "8.5", height: "12" }, partClass("right", lit)),
    shape("rect", { x: "10.5", y: "5", width: "3", height: "7", rx: "1.5" }, partClass("wheel", lit)),
  );
  return svg;
}

/** The class of one part of the mouse, with the lit class when that part is the one lit. */
function partClass(part: MouseKey["part"], lit: MouseKey["part"]): string {
  return part === lit ? "vu-mouse-part is-lit" : "vu-mouse-part";
}

/** Creates an SVG shape with a class and attributes. */
function shape(tag: "rect", attributes: Readonly<Record<string, string>>, className: string): SVGElement {
  const node = createSvgElement(tag);
  node.setAttribute("class", className);
  for (const [name, value] of Object.entries(attributes)) node.setAttribute(name, value);
  return node;
}
