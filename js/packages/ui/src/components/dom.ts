// Small DOM helpers that the components share. Every element is made here, and text always goes
// in with textContent, never innerHTML.

/** The SVG namespace, for the mouse icon in a keycap. */
const SVG_NS = "http://www.w3.org/2000/svg";

/** Selector for elements that a keyboard user can reach. */
const FOCUSABLE = [
  "button:not([disabled])",
  "[href]",
  "input:not([disabled])",
  "select:not([disabled])",
  "textarea:not([disabled])",
  '[tabindex]:not([tabindex="-1"])',
].join(", ");

/** Last number given to an id, so that every id on the page is different. */
let lastId = 0;

/**
 * Creates an HTML element with an optional class name and text.
 * @param tag - the tag name.
 * @param className - the class attribute, if any.
 * @param text - the text content, if any.
 */
export function createElement<Tag extends "div" | "span" | "button" | "kbd" | "h2" | "p">(
  tag: Tag,
  className?: string,
  text?: string,
): HTMLElementTagNameMap[Tag] {
  const node = document.createElement(tag);
  if (className !== undefined) node.className = className;
  if (text !== undefined) node.textContent = text;
  return node;
}

/** Creates an SVG element in the SVG namespace. */
export function createSvgElement(tag: "svg" | "rect" | "path"): SVGElement {
  return document.createElementNS(SVG_NS, tag);
}

/** An id that no other call returns, used by aria-labelledby and aria-describedby. */
export function uniqueId(prefix: string): string {
  lastId += 1;
  return `vu-${prefix}-${lastId}`;
}

/**
 * The elements inside `container` that a keyboard user can reach, in document order. Elements
 * that are hidden or disabled are left out.
 */
export function focusableWithin(container: HTMLElement): HTMLElement[] {
  return Array.from(container.querySelectorAll<HTMLElement>(FOCUSABLE)).filter(
    (node) => !node.hidden && node.getAttribute("aria-disabled") !== "true",
  );
}

/**
 * Takes the keyboard focus off a container that is about to be hidden or removed. A focus outside the
 * container is left alone. A focus inside it moves to `fallback` when that is given and can take the
 * focus now, and otherwise the focused element is blurred, so the focus goes to the page.
 *
 * @param container - the element that is being hidden or removed.
 * @param fallback - where the focus should go instead, such as the button that opened the container.
 */
export function releaseFocus(container: Element, fallback: HTMLElement | null = null): void {
  if (!container.contains(document.activeElement)) return;
  if (fallback !== null && !container.contains(fallback) && canTakeFocus(fallback)) {
    fallback.focus({ preventScroll: true });
    return;
  }
  const focused = document.activeElement;
  if (focused instanceof HTMLElement || focused instanceof SVGElement) focused.blur();
}

/** True when an element can take the focus now: it is connected, enabled, and not inside a hidden or inert part. */
function canTakeFocus(node: HTMLElement): boolean {
  return node.isConnected && node.closest("[hidden], [inert], [disabled], [aria-disabled='true']") === null;
}
