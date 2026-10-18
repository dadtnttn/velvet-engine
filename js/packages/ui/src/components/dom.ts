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
