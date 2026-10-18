// Test helpers for the stage and the components. happy-dom does no layout, so these helpers give
// each element the rectangle that a browser would draw. The window is the rectangle of every
// .vu-viewport. The root is drawn where its inline position puts it, at its size times the scale
// in its inline transform. Tests can also place any element and give it an offset size.
// Only the tests import this file. The package index does not export it.

import { vi } from "vitest";

/** A rectangle in CSS pixels. */
export interface Box {
  readonly left: number;
  readonly top: number;
  readonly width: number;
  readonly height: number;
}

/** The window's rectangle. */
let windowBox: Box = { left: 0, top: 0, width: 1920, height: 1080 };

/** Rectangles that tests gave to particular elements. */
const placed = new WeakMap<Element, Box>();

/** Sets the size of the fake window. Call it before the stage is created, or dispatch a resize. */
export function setWindowSize(width: number, height: number): void {
  windowBox = { left: 0, top: 0, width, height };
}

/** Gives one element a rectangle of its own, such as an anchor for a tooltip. */
export function placeElement(element: Element, box: Box): void {
  placed.set(element, box);
}

/** Sets the offsetWidth and offsetHeight of an element, which happy-dom reports as zero. */
export function setOffsetSize(element: HTMLElement, width: number, height: number): void {
  Object.defineProperty(element, "offsetWidth", { configurable: true, value: width });
  Object.defineProperty(element, "offsetHeight", { configurable: true, value: height });
}

/**
 * Replaces getBoundingClientRect with the fake geometry. Call `vi.restoreAllMocks()` in
 * afterEach to undo it.
 */
export function installFakeGeometry(): void {
  vi.spyOn(Element.prototype, "getBoundingClientRect").mockImplementation(function (this: Element) {
    const box = boxOf(this);
    return new DOMRect(box.left, box.top, box.width, box.height);
  });
}

/** The rectangle that the fake browser gives an element. */
function boxOf(element: Element): Box {
  const own = placed.get(element);
  if (own !== undefined) return own;
  if (element.classList.contains("vu-viewport")) return windowBox;
  if (element.classList.contains("vu-root")) return rootBox(element as HTMLElement);
  return { left: 0, top: 0, width: 0, height: 0 };
}

/** The root's rectangle as drawn: its inline position, and its size times the scale. */
function rootBox(root: HTMLElement): Box {
  const scaleText = /scale\(([^)]+)\)/.exec(root.style.transform)?.[1];
  const scale = scaleText === undefined ? 1 : Number(scaleText);
  return {
    left: parseFloat(root.style.left) || 0,
    top: parseFloat(root.style.top) || 0,
    width: (parseFloat(root.style.width) || 0) * scale,
    height: (parseFloat(root.style.height) || 0) * scale,
  };
}
