// The screen stack: the title, the game, the pause menu and the dialogues are screens that are pushed,
// popped and replaced in one order, so the game always knows what is on top.
//
// Two kinds of screen. A "screen" covers the screens below it, which are hidden. An "overlay" (a pause
// menu, a dialogue, a confirmation) sits over the screen below it, with a scrim. The screens under the top
// one are inert: the pointer and the keyboard do not reach them, and the focus stays on the top screen.
//
// Hooks, in the order they run:
// - onEnter when a screen comes on the stack, or replaces another.
// - onPause when a screen is pushed over it, and onResume when the screen above it leaves.
// - onLeave when a screen leaves the stack, or is replaced.
// A game stops its clock in onPause and starts it again in onResume. Keep in mind that the player may
// have paused the game before the menu opened, so remember that state at onPause.
// Hooks should not push or pop by themselves: the change would run inside the hook that made it.
//
// Back: Escape and a pad's B press (see startPadMenuKeys in @velvet/core) reach the top screen through
// `back()`. The screen's onBack returns false to stay open, for example after it pushed another screen.
// Any other result pops it. A screen without onBack is popped too, unless it is the only one.
//
// Focus: each screen remembers the element that had the focus when another screen covered it, and that
// element gets the focus back when the screen is on top again. While a screen is on the stack, the focus
// stays in it: a click on empty space that takes the focus away brings it back to the top screen.
//
// Transitions fade a screen in and out, and move it a little, with CSS opacity and transform only. Reduced
// motion turns them off.

import { createElement, focusableWithin } from "./components/dom";

/** How a screen sits on the stack. A screen covers the screens below it; an overlay sits over them. */
export type ScreenKind = "screen" | "overlay";

/**
 * A screen: an element and the hooks that the stack calls. Make one per screen and keep it. A screen can
 * leave the stack and come back, so the element is reused, not copied.
 */
export interface Screen {
  /** The name of the screen. No two screens on the stack share a name. */
  readonly id: string;
  /** The element with the screen's content. The stack shows, hides and removes it; the owner keeps the object. */
  readonly el: HTMLElement;
  /** "screen" (the default) or "overlay". */
  readonly kind?: ScreenKind;
  /** Runs when the screen comes on the stack, or replaces another screen. */
  onEnter?(): void;
  /** Runs when the screen leaves the stack, or is replaced. */
  onLeave?(): void;
  /** Runs when another screen is pushed over this one. */
  onPause?(): void;
  /** Runs when the screen above this one leaves, and this one is on top again. */
  onResume?(): void;
  /**
   * Runs on Escape or a pad's B while this screen is on top. Return false to stay open, which is the
   * answer when the screen pushed another one. Any other result pops this screen.
   */
  onBack?(): boolean | void;
  /** The element that takes the focus when the screen comes on top. Default: its first focusable element. */
  focusTarget?(): HTMLElement | null;
}

/** Options for {@link createScreenStack}. */
export interface ScreenStackOptions {
  /**
   * How long a transition takes, in milliseconds. Default 180. Zero turns transitions off, and so does the
   * player's preference for reduced motion.
   * @throws RangeError when the value is negative or not a number.
   */
  readonly transitionMs?: number;
}

/** A listener for {@link ScreenStack.onChange}. It receives the screen on top, or null when the stack is empty. */
export type ScreenListener = (top: Screen | null) => void;

/** The screen stack. See {@link createScreenStack}. */
export interface ScreenStack {
  /** The layer that holds every screen. It covers the host. */
  readonly el: HTMLElement;
  /** The screen on top, or null when the stack is empty. */
  readonly top: Screen | null;
  /** How many screens are on the stack. */
  readonly depth: number;
  /** True when a screen with this id is on the stack. */
  has(id: string): boolean;
  /** The ids of the screens, from the bottom of the stack to the top. */
  ids(): readonly string[];
  /**
   * Puts a screen on top. The screen under it is paused. A screen of kind "screen" covers the ones below it.
   * @throws Error when a screen with the same id is already on the stack.
   */
  push(screen: Screen): void;
  /** Takes the top screen off the stack, and returns it. The screen under it resumes. Null when the stack is empty. */
  pop(): Screen | null;
  /** Swaps the top screen for another one. The screens under it stay as they are. Pushes when the stack is empty. */
  replace(screen: Screen): void;
  /** Takes every screen above the one named `id` off the stack. Returns false when no such screen is on the stack. */
  popTo(id: string): boolean;
  /**
   * The back action: it goes to the top screen's onBack, or pops the screen when there is no onBack.
   * @returns true when the key was taken, and false when nothing could go back (an empty stack, or the only screen).
   */
  back(): boolean;
  /**
   * Adds a listener that runs after every change of the stack.
   * @returns a function that removes the listener.
   */
  onChange(listener: ScreenListener): () => void;
  /** Removes the layer and the listeners. The screens' hooks do not run. */
  destroy(): void;
}

/** A screen on the stack, with the state that the stack keeps for it. */
interface Entry {
  readonly screen: Screen;
  readonly kind: ScreenKind;
  /** True while another screen covers this one: onPause has run and onResume has not. */
  paused: boolean;
  /** The element that had the focus when this screen was covered, or null. */
  savedFocus: HTMLElement | null;
}

/** The transition length when the options give none, in milliseconds. */
const DEFAULT_TRANSITION_MS = 180;

/**
 * Creates a screen stack inside `host`. The host must be positioned, as the stage's root is; a static
 * host is made relative, and that is undone by `destroy`.
 *
 * @example
 * const screens = createScreenStack(stage.root);
 * screens.push(titleScreen);
 * // in the game's key handler, after Escape has gone to the stack:
 * screens.push(pauseScreen); // the pause screen's onPause stops the clock
 *
 * @param host - the element that holds the screens.
 * @param options - see {@link ScreenStackOptions}.
 */
export function createScreenStack(host: HTMLElement, options: ScreenStackOptions = {}): ScreenStack {
  const transitionMs = transitionLength(options.transitionMs);
  const layer = createElement("div", "vu-screens");
  Object.assign(layer.style, { position: "absolute", left: "0", top: "0", right: "0", bottom: "0" });
  // The stylesheet fades for as long as the timer removes the screen, so the two always agree.
  layer.style.setProperty("--vu-screen-ms", `${transitionMs}ms`);
  const hostChange = makeHostPositioned(host);
  host.append(layer);

  const entries: Entry[] = [];
  /** Screens that are fading out, with the timer that removes each one. */
  const leaving = new Map<HTMLElement, ReturnType<typeof setTimeout>>();
  const listeners = new Set<ScreenListener>();
  let destroyed = false;

  const topEntry = (): Entry | undefined => entries[entries.length - 1];

  /** Tells the listeners what is on top now. */
  const emit = (): void => {
    const top = topEntry()?.screen ?? null;
    for (const listener of [...listeners]) listener(top);
  };

  /**
   * Puts a screen's element in the layer, and starts its fade-in. The forced layout read makes the browser
   * apply the starting style first, so the transition runs.
   */
  const mount = (screen: Screen, kind: ScreenKind): void => {
    const node = screen.el;
    const pending = leaving.get(node);
    if (pending !== undefined) {
      clearTimeout(pending);
      leaving.delete(node);
    }
    node.classList.add("vu-screen");
    node.classList.toggle("vu-screen-overlay", kind === "overlay");
    node.dataset["screen"] = screen.id;
    if (!node.hasAttribute("tabindex")) node.tabIndex = -1;
    node.removeAttribute("inert");
    const animate = transitionMs > 0;
    if (animate) node.dataset["phase"] = "enter";
    layer.append(node);
    if (animate) {
      void node.offsetWidth;
      delete node.dataset["phase"];
    }
  };

  /**
   * Takes a screen's element off the stack. It is made inert at once, and removed when its fade-out has
   * run, or straight away when transitions are off.
   */
  const detach = (node: HTMLElement): void => {
    node.setAttribute("inert", "");
    const previous = leaving.get(node);
    if (previous !== undefined) clearTimeout(previous);
    leaving.delete(node);
    if (transitionMs <= 0) {
      node.remove();
      return;
    }
    node.dataset["phase"] = "leave";
    const timer = setTimeout(() => {
      leaving.delete(node);
      node.remove();
    }, transitionMs);
    leaving.set(node, timer);
  };

  /**
   * Shows the screens that should be seen, and makes every screen below the top one inert. A screen hides
   * the screens under it, and an overlay leaves them visible under its scrim.
   */
  const applyLayers = (): void => {
    let base = 0;
    for (let index = entries.length - 1; index >= 0; index -= 1) {
      if (entries[index]?.kind === "screen") {
        base = index;
        break;
      }
    }
    const last = entries.length - 1;
    entries.forEach((entry, index) => {
      const node = entry.screen.el;
      node.hidden = index < base;
      if (index === last) node.removeAttribute("inert");
      else node.setAttribute("inert", "");
    });
  };

  /** Gives the focus to a screen's focus target, or to its first focusable element, or to the screen itself. */
  const focusScreen = (screen: Screen): void => {
    const target = screen.focusTarget?.() ?? focusableWithin(screen.el)[0] ?? screen.el;
    target.focus({ preventScroll: true });
  };

  /** Gives the focus back to the element that the screen had when it was covered, else to the screen. */
  const restoreFocus = (entry: Entry): void => {
    const saved = entry.savedFocus;
    entry.savedFocus = null;
    const node = entry.screen.el;
    if (saved !== null && saved.isConnected && (saved === node || focusableWithin(node).includes(saved))) {
      saved.focus({ preventScroll: true });
      return;
    }
    focusScreen(entry.screen);
  };

  /** The element of a screen that has the focus, or null when the focus is elsewhere. */
  const focusedWithin = (container: HTMLElement): HTMLElement | null => {
    const active = document.activeElement;
    return active instanceof HTMLElement && container.contains(active) ? active : null;
  };

  const pauseEntry = (entry: Entry): void => {
    if (entry.paused) return;
    entry.paused = true;
    entry.screen.onPause?.();
  };

  const resumeEntry = (entry: Entry): void => {
    if (!entry.paused) return;
    entry.paused = false;
    entry.screen.onResume?.();
  };

  /** Throws when a screen with this id is already on the stack, other than `except`. */
  const assertFreeId = (id: string, except: Entry | undefined): void => {
    if (entries.some((entry) => entry !== except && entry.screen.id === id)) {
      throw new Error(`a screen named "${id}" is already on the stack`);
    }
  };

  /**
   * Takes the screens above index `keep` off the stack, top first, then brings the screen at `keep` back to
   * the top: it resumes and gets its focus back.
   */
  const unwind = (keep: number): void => {
    while (entries.length - 1 > keep) {
      const entry = entries.pop();
      if (entry === undefined) break;
      entry.screen.onLeave?.();
      detach(entry.screen.el);
    }
    applyLayers();
    const top = topEntry();
    if (top === undefined) return;
    resumeEntry(top);
    restoreFocus(top);
  };

  const push = (screen: Screen): void => {
    if (destroyed) return;
    assertFreeId(screen.id, undefined);
    const under = topEntry();
    if (under !== undefined) {
      under.savedFocus = focusedWithin(under.screen.el);
      pauseEntry(under);
    }
    const entry: Entry = { screen, kind: screen.kind ?? "screen", paused: false, savedFocus: null };
    entries.push(entry);
    mount(screen, entry.kind);
    applyLayers();
    screen.onEnter?.();
    if (topEntry() === entry) focusScreen(screen);
    emit();
  };

  const pop = (): Screen | null => {
    if (destroyed) return null;
    const top = topEntry();
    if (top === undefined) return null;
    unwind(entries.length - 2);
    emit();
    return top.screen;
  };

  const replace = (screen: Screen): void => {
    if (destroyed) return;
    const old = topEntry();
    if (old === undefined) {
      push(screen);
      return;
    }
    assertFreeId(screen.id, old);
    entries.pop();
    old.screen.onLeave?.();
    detach(old.screen.el);
    const entry: Entry = { screen, kind: screen.kind ?? "screen", paused: false, savedFocus: null };
    entries.push(entry);
    mount(screen, entry.kind);
    applyLayers();
    screen.onEnter?.();
    if (topEntry() === entry) focusScreen(screen);
    emit();
  };

  const popTo = (id: string): boolean => {
    if (destroyed) return false;
    const index = entries.findIndex((entry) => entry.screen.id === id);
    if (index < 0) return false;
    if (index < entries.length - 1) {
      unwind(index);
      emit();
    }
    return true;
  };

  const back = (): boolean => {
    if (destroyed) return false;
    const top = topEntry();
    if (top === undefined) return false;
    if (top.screen.onBack === undefined) {
      if (entries.length < 2) return false;
      pop();
      return true;
    }
    const result = top.screen.onBack();
    // A screen that stays, or that changed the stack itself, is left alone.
    if (result === false || topEntry() !== top) return true;
    if (entries.length > 1) pop();
    return true;
  };

  /** Escape goes to the top screen. A control that already took the key (a menu, a modal) prevents the default. */
  const onKeyDown = (event: KeyboardEvent): void => {
    if (event.key !== "Escape" || event.defaultPrevented || event.repeat) return;
    if (back()) event.preventDefault();
  };

  /** Brings the focus back when a click on empty space has taken it away from the stack. */
  const onFocusOut = (event: FocusEvent): void => {
    if (destroyed || event.relatedTarget !== null) return;
    queueMicrotask(() => {
      if (destroyed) return;
      const active = document.activeElement;
      if (active !== null && active !== document.body) return;
      const top = topEntry();
      if (top !== undefined) focusScreen(top.screen);
    });
  };

  window.addEventListener("keydown", onKeyDown);
  window.addEventListener("focusout", onFocusOut);

  return {
    el: layer,
    get top(): Screen | null {
      return topEntry()?.screen ?? null;
    },
    get depth(): number {
      return entries.length;
    },
    has(id: string): boolean {
      return entries.some((entry) => entry.screen.id === id);
    },
    ids(): readonly string[] {
      return entries.map((entry) => entry.screen.id);
    },
    push,
    pop,
    replace,
    popTo,
    back,
    onChange(listener: ScreenListener): () => void {
      listeners.add(listener);
      return () => {
        listeners.delete(listener);
      };
    },
    destroy(): void {
      if (destroyed) return;
      destroyed = true;
      window.removeEventListener("keydown", onKeyDown);
      window.removeEventListener("focusout", onFocusOut);
      for (const timer of leaving.values()) clearTimeout(timer);
      leaving.clear();
      entries.length = 0;
      listeners.clear();
      layer.remove();
      if (hostChange !== null) host.style.position = hostChange.previous;
    },
  };
}

/**
 * The transition length to use: the requested one, or the default, and zero when the player prefers
 * reduced motion.
 * @param requested - the length from the options, if any.
 * @throws RangeError when the length is negative or not a number.
 */
function transitionLength(requested: number | undefined): number {
  if (requested !== undefined && (!Number.isFinite(requested) || requested < 0)) {
    throw new RangeError(`transitionMs must be a number of milliseconds from 0, got ${requested}`);
  }
  if (prefersReducedMotion()) return 0;
  return requested ?? DEFAULT_TRANSITION_MS;
}

/** True when the player asked for less motion in the operating system. */
function prefersReducedMotion(): boolean {
  return typeof matchMedia === "function" && matchMedia("(prefers-reduced-motion: reduce)").matches;
}

/**
 * Makes the host a positioning context when it is static. Returns what to restore, or null when the host
 * was already positioned.
 * @param host - the element that holds the screens.
 */
function makeHostPositioned(host: HTMLElement): { readonly previous: string } | null {
  const position = getComputedStyle(host).position;
  if (position !== "" && position !== "static") return null;
  const previous = host.style.position;
  host.style.position = "relative";
  return { previous };
}
