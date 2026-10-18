// Dirty marks: each element (or panel) registers how it draws itself, and the game marks the ones that
// changed. The marks are drawn in one batch per frame, so ten changes in a frame cost one pass of DOM
// writes, and an element that nothing marked costs nothing. The write helpers at the bottom skip a write
// that would change nothing, so a value that repeats touches no DOM at all.
//
//   const marks = createDirtyMarks();
//   marks.register("gold", () => setText(goldEl, String(run.gold)));
//   marks.mark("gold");   // any number of marks in this frame ...
//                         // ... are drawn once, at the next animation frame

import { releaseFocus } from "./components/dom";

/** Options for {@link createDirtyMarks}. */
export interface DirtyOptions {
  /**
   * How a flush is asked for. It is called once when the first mark arrives after a flush, and the
   * function it receives draws the marks. Default: the next animation frame, or a 16 ms timer where the
   * host has no frames. Null means the game calls `flush` itself, for example from its loop.
   */
  readonly schedule?: ((flush: () => void) => void) | null;
}

/** The marks of a set of elements. See {@link createDirtyMarks}. */
export interface DirtyMarks {
  /**
   * Adds an element, with the function that draws it. A name can be registered once: a second
   * registration replaces the first. The element starts marked, so it draws at the next flush.
   * @param name - the element's name.
   * @param render - draws the element from the game's state.
   */
  register(name: string, render: () => void): void;
  /** Removes an element. It is no longer drawn, even when marked. */
  unregister(name: string): void;
  /**
   * Marks an element to draw at the next flush. Marking it twice costs no more than marking it once. A
   * name that is not registered is kept, so an element registered later still draws once.
   */
  mark(name: string): void;
  /** Marks every registered element, for a change that touches all of them (such as a new language). */
  markAll(): void;
  /** True when the element is marked and waiting for the next flush. */
  isMarked(name: string): boolean;
  /**
   * Draws every marked element once, in the order they were registered, and clears the marks. An element
   * marked while the flush runs is drawn by the next flush.
   * @returns how many elements were drawn.
   */
  flush(): number;
  /** Forgets every element and stops the scheduling. Marks made after this do nothing. */
  destroy(): void;
}

/** The fallback frame interval, in milliseconds, where the host has no animation frames. */
const FALLBACK_FRAME_MS = 16;

/**
 * Creates a set of dirty marks.
 * @param options - how a flush is scheduled.
 */
export function createDirtyMarks(options: DirtyOptions = {}): DirtyMarks {
  const schedule = options.schedule === undefined ? nextFrame : options.schedule;
  const renders = new Map<string, () => void>();
  const marked = new Set<string>();
  let scheduled = false;
  let destroyed = false;

  /** Draws the marked elements in registration order, and clears their marks. */
  const flush = (): number => {
    if (marked.size === 0) return 0;
    let drawn = 0;
    for (const [name, render] of renders) {
      if (!marked.has(name)) continue;
      marked.delete(name);
      render();
      drawn += 1;
    }
    return drawn;
  };

  /** Asks for one flush, unless one is already on its way. */
  const requestFlush = (): void => {
    if (scheduled || destroyed || schedule === null) return;
    scheduled = true;
    schedule(() => {
      scheduled = false;
      if (!destroyed) flush();
    });
  };

  return {
    register(name, render) {
      if (destroyed) return;
      renders.set(name, render);
      marked.add(name);
      requestFlush();
    },
    unregister(name) {
      renders.delete(name);
      marked.delete(name);
    },
    mark(name) {
      if (destroyed) return;
      marked.add(name);
      requestFlush();
    },
    markAll() {
      if (destroyed) return;
      for (const name of renders.keys()) marked.add(name);
      if (renders.size > 0) requestFlush();
    },
    isMarked(name) {
      return marked.has(name);
    },
    flush,
    destroy() {
      destroyed = true;
      renders.clear();
      marked.clear();
    },
  };
}

/**
 * Sets the text of a node, and writes nothing when it already has that text.
 * @param node - the node whose text is set.
 * @param text - the new text.
 * @returns true when the node was written.
 */
export function setText(node: Node, text: string): boolean {
  if (node.textContent === text) return false;
  node.textContent = text;
  return true;
}

/**
 * Shows or hides an element with the `hidden` attribute, and writes nothing when it is already in that state.
 * @param node - the element.
 * @param shown - true to show it.
 * @returns true when the element was written.
 */
export function setShown(node: HTMLElement, shown: boolean): boolean {
  const hidden = !shown;
  if (node.hidden === hidden) return false;
  node.hidden = hidden;
  // A hidden element must not keep the keyboard focus, so the focus leaves it here.
  if (hidden) releaseFocus(node);
  return true;
}

/**
 * Marks an element as active with the `is-active` class, and with `aria-pressed` when it is a button.
 * Writes nothing when the element is already in that state.
 * @param node - the element.
 * @param active - true when it is active.
 * @returns true when the element was written.
 */
export function setActive(node: HTMLElement, active: boolean): boolean {
  let written = false;
  if (node.classList.contains("is-active") !== active) {
    node.classList.toggle("is-active", active);
    written = true;
  }
  if (node instanceof HTMLButtonElement) {
    const pressed = String(active);
    if (node.getAttribute("aria-pressed") !== pressed) {
      node.setAttribute("aria-pressed", pressed);
      written = true;
    }
  }
  return written;
}

/**
 * Asks for a callback on the next animation frame, or after a short timer where the host has none.
 * @param flush - the callback.
 */
function nextFrame(flush: () => void): void {
  if (typeof requestAnimationFrame === "function") {
    requestAnimationFrame(() => {
      flush();
    });
    return;
  }
  setTimeout(flush, FALLBACK_FRAME_MS);
}
