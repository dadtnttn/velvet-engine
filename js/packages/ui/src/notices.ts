// Notices: short messages on the screen, in a corner that the game picks. They queue by priority: the most
// important show first, and at most `maxVisible` show at once. A notice stays on screen for its duration,
// then fades. A notice can carry buttons (the player must answer it), and a sticky one (no duration) stays
// until it is dismissed. A notice with a key is counted, not repeated, while the same key is still on screen.
//
// The column never covers a given rectangle (`avoid`, such as a dialogue box or a minimap). When the column
// would cover it, the column moves to whichever side of the rectangle has more room, and the notices that
// do not fit wait until there is room. A history drawer lists the notices, newest first.
//
// The column and the drawer are made in logical pixels, so the host should be the stage's root (or another
// element that uses the same logical size). Make the notices after the screens, so they draw over them.

import type { TranslateFn } from "./rebind-panel";
import type { VuComponent } from "./components/component";
import { createElement } from "./components/dom";
import type { LogicalRect } from "./stage";

/** The colour of a notice: information, a warning, or a loss or a refusal. */
export type NoticeTone = "info" | "warn" | "danger";

/** The corner of the surface that the notices stack from. */
export type NoticeAnchor = "top-left" | "top-right" | "bottom-left" | "bottom-right";

/** A button on a notice. The player chooses it, and the notice goes away. */
export interface NoticeAction {
  /** Given to `onAction` when the button is chosen. */
  readonly id: string;
  /** The text of the button. */
  readonly label: string;
}

/** What to show. Only `message` is needed. */
export interface NoticeSpec {
  /** The text of the notice. It is set as plain text. */
  readonly message: string;
  /** The colour. Default "info". */
  readonly tone?: NoticeTone;
  /** The order: a higher priority shows first, and can take the place of a lower one. Default 0. */
  readonly priority?: number;
  /** How long it stays on screen, in milliseconds. Null keeps it until it is dismissed. Default: the notices' default. */
  readonly durationMs?: number | null;
  /** A key for the notice. While one with the same key is on screen, the new one only adds to its count. */
  readonly key?: string;
  /** Buttons under the text. */
  readonly actions?: readonly NoticeAction[];
  /** Runs with the id of the button that the player chose, before the notice goes away. */
  readonly onAction?: (actionId: string) => void;
}

/** A notice in the history, newest first. */
export interface NoticeRecord {
  /** The id that `notify` returned. */
  readonly id: string;
  readonly message: string;
  readonly tone: NoticeTone;
  /** How many times it was raised while it was on screen. */
  readonly count: number;
}

/** Props of the notices. They apply from now on. */
export interface NoticesProps {
  /** The corner that the column stacks from. Default "bottom-right". */
  readonly anchor?: NoticeAnchor;
  /** A rectangle in logical pixels that the notices must never cover, or a function that gives it (or null). */
  readonly avoid?: LogicalRect | (() => LogicalRect | null) | null;
  /** The most notices on screen at once. Default 3. Must be a whole number from 1. */
  readonly maxVisible?: number;
  /** The text lookup for the history drawer. Without one, the English fallbacks are used. */
  readonly t?: TranslateFn;
}

/** Options for {@link createNotices}. */
export interface NoticesOptions extends NoticesProps {
  /** The element that holds the notices and the drawer. Usually the stage's root. */
  readonly host: HTMLElement;
  /** Logical width of the surface. Default 1920. */
  readonly width?: number;
  /** Logical height of the surface. Default 1080. */
  readonly height?: number;
  /** The gap between the column and the surface's edges, in logical pixels. Default 32. */
  readonly margin?: number;
  /** The default time on screen, in milliseconds. Default 4000. */
  readonly durationMs?: number;
  /** How long a notice takes to fade, in milliseconds. Default 240. Zero when the player prefers reduced motion. */
  readonly fadeMs?: number;
  /** How many notices the history keeps. Default 50. */
  readonly historyLimit?: number;
}

/** The notices. The component's element is the column of notices. */
export interface Notices extends VuComponent<NoticesProps> {
  /**
   * Raises a notice. It shows as soon as a place is free, by priority.
   * @returns the id of the notice (the same id for a repeated key while the notice is on screen).
   * @throws RangeError when the duration is negative or not a number.
   */
  notify(spec: NoticeSpec): string;
  /** Takes a notice off the screen, with its fade. It stays in the history. Does nothing for an unknown id. */
  dismiss(id: string): void;
  /** Takes every notice off the screen at once. The history is kept. */
  clear(): void;
  /** The ids of the notices on screen, in the order they are drawn. */
  visible(): readonly string[];
  /** The history, newest first. */
  history(): readonly NoticeRecord[];
  /** Places the column again, and shows the notices that now fit. Call it when the rectangle to avoid moves. */
  relayout(): void;
  /** True while the history drawer is open. */
  readonly historyOpen: boolean;
  /** Opens the history drawer. Its close button takes the focus. */
  openHistory(): void;
  /** Closes the history drawer, and gives the focus back to the element that had it. */
  closeHistory(): void;
  /** Opens the drawer when it is closed, and closes it when it is open. */
  toggleHistory(): void;
}

/** One notice that is known to the list: on screen, waiting for a place, or fading out. */
interface Entry {
  readonly id: string;
  /** The order that the notices were raised in. */
  readonly seq: number;
  readonly key: string | null;
  readonly priority: number;
  readonly tone: NoticeTone;
  /** The time on screen in full, or null when sticky. */
  readonly durationMs: number | null;
  readonly onAction: ((actionId: string) => void) | undefined;
  readonly el: HTMLElement;
  /** The badge that shows the count, hidden while the count is one. */
  readonly badge: HTMLElement;
  /** The record in the history, which the count updates. */
  readonly record: { id: string; message: string; tone: NoticeTone; count: number };
  state: "waiting" | "visible" | "leaving";
  /** Time left on screen, in milliseconds, or null when sticky. */
  remainingMs: number | null;
  /** When the current timer started, in milliseconds since the epoch. */
  startedAt: number;
  /** The timer that dismisses the notice, or the timer of its fade. Null when none runs. */
  timer: ReturnType<typeof setTimeout> | null;
}

/** The gap between two notices in the column, in logical pixels. It matches the stylesheet. */
const NOTICE_GAP = 10;

/** The gap between the column and a rectangle that it moves away from, in logical pixels. */
const AVOID_GAP = 16;

/** Defaults. */
const DEFAULT_WIDTH = 1920;
const DEFAULT_HEIGHT = 1080;
const DEFAULT_MARGIN = 32;
const DEFAULT_DURATION_MS = 4000;
const DEFAULT_FADE_MS = 240;
const DEFAULT_HISTORY_LIMIT = 50;
const DEFAULT_MAX_VISIBLE = 3;

/**
 * Creates the notices. Append nothing: the column and the drawer are added to `host` at once.
 * @param options - the host, the surface and the props.
 * @throws RangeError when a size, a time or a count is out of range.
 */
export function createNotices(options: NoticesOptions): Notices {
  const host = options.host;
  const surface = {
    width: positiveNumber(options.width ?? DEFAULT_WIDTH, "width"),
    height: positiveNumber(options.height ?? DEFAULT_HEIGHT, "height"),
  };
  const margin = nonNegativeNumber(options.margin ?? DEFAULT_MARGIN, "margin");
  const defaultDuration = nonNegativeNumber(options.durationMs ?? DEFAULT_DURATION_MS, "durationMs");
  const fadeMs = prefersReducedMotion() ? 0 : nonNegativeNumber(options.fadeMs ?? DEFAULT_FADE_MS, "fadeMs");
  const historyLimit = wholeNumber(options.historyLimit ?? DEFAULT_HISTORY_LIMIT, 0, "historyLimit");
  let current: { anchor: NoticeAnchor; avoid: NoticesProps["avoid"]; maxVisible: number; t: TranslateFn | undefined } = {
    anchor: options.anchor ?? "bottom-right",
    avoid: options.avoid,
    maxVisible: checkedMaxVisible(options.maxVisible ?? DEFAULT_MAX_VISIBLE),
    t: options.t,
  };
  let entries: Entry[] = [];
  const history: Array<{ id: string; message: string; tone: NoticeTone; count: number }> = [];
  let sequence = 0;
  let destroyed = false;
  let returnFocus: HTMLElement | null = null;

  const column = createElement("div", "vu-notices");
  column.setAttribute("role", "status");
  column.setAttribute("aria-live", "polite");
  Object.assign(column.style, { position: "absolute", left: "0", top: "0" });
  host.append(column);

  const drawer = createElement("div", "vu-history");
  drawer.setAttribute("role", "dialog");
  drawer.hidden = true;
  const drawerHead = createElement("div", "vu-history-head");
  const drawerTitle = createElement("h2", "vu-history-title");
  const closeButton = createElement("button", "vu-button");
  closeButton.type = "button";
  drawerHead.append(drawerTitle, closeButton);
  const drawerList = document.createElement("ol");
  drawerList.className = "vu-history-list";
  drawer.append(drawerHead, drawerList);
  host.append(drawer);

  /** The text of a key, or its English fallback when the lookup has no text for it. */
  const text = (key: string, fallback: string): string => {
    if (current.t === undefined) return fallback;
    const value = current.t(key);
    return value === key ? fallback : value;
  };

  /** The entries on screen, in the order they are drawn: higher priority first, then the older one. */
  const shownEntries = (): Entry[] =>
    entries.filter((entry) => entry.state === "visible").sort(byDisplayOrder);

  /** The entries that wait for a place, in the order they will get one. */
  const waitingEntries = (): Entry[] => entries.filter((entry) => entry.state === "waiting").sort(byDisplayOrder);

  /** Stops the timer of an entry, and keeps the time it still had on screen. */
  const stopTimer = (entry: Entry): void => {
    if (entry.timer === null) return;
    clearTimeout(entry.timer);
    entry.timer = null;
    if (entry.remainingMs !== null) {
      entry.remainingMs = Math.max(0, entry.remainingMs - (Date.now() - entry.startedAt));
    }
  };

  /** Starts the timer that dismisses an entry, with the time it has left. Sticky notices get none. */
  const startTimer = (entry: Entry): void => {
    if (entry.durationMs === null) return;
    const remaining = entry.remainingMs ?? entry.durationMs;
    entry.remainingMs = remaining;
    entry.startedAt = Date.now();
    entry.timer = setTimeout(() => {
      entry.timer = null;
      dismiss(entry.id);
    }, remaining);
  };

  /** Puts a waiting notice on screen. */
  const show = (entry: Entry): void => {
    entry.state = "visible";
    column.append(entry.el);
    startTimer(entry);
  };

  /** Takes a notice off the screen and back to the queue, keeping the time it has left. */
  const hide = (entry: Entry): void => {
    stopTimer(entry);
    entry.state = "waiting";
    entry.el.remove();
  };

  /** Removes an entry from the list, and from the page. */
  const forget = (entry: Entry): void => {
    stopTimer(entry);
    entry.el.remove();
    entries = entries.filter((other) => other !== entry);
  };

  /**
   * Fills the places on screen from the queue. A notice with a higher priority than the lowest one on
   * screen takes its place, and that one goes back to the queue.
   */
  const promote = (): void => {
    for (;;) {
      const next = waitingEntries()[0];
      if (next === undefined) return;
      const shown = shownEntries();
      if (shown.length < current.maxVisible) {
        show(next);
        continue;
      }
      const lowest = shown[shown.length - 1];
      if (lowest === undefined || next.priority <= lowest.priority) return;
      hide(lowest);
    }
  };

  /** The height of a list of notices in the column, from their measured heights and the gaps between them. */
  const heightOf = (list: readonly Entry[]): number =>
    list.reduce((sum, entry) => sum + entry.el.offsetHeight, 0) + Math.max(0, list.length - 1) * NOTICE_GAP;

  /** The rectangle to avoid, from the props: a rectangle, a function that gives one, or null. */
  const avoidedRect = (): LogicalRect | null => {
    const avoid = current.avoid;
    if (avoid === undefined || avoid === null) return null;
    return typeof avoid === "function" ? avoid() : avoid;
  };

  /**
   * Places the column in its corner. When that would cover the rectangle to avoid, the column goes to the side
   * of the rectangle with more room, and the lowest notices that do not fit are sent back to the queue.
   */
  const place = (): void => {
    const anchor = current.anchor;
    const bottom = anchor.startsWith("bottom");
    const right = anchor.endsWith("right");
    const width = column.offsetWidth;
    let kept = shownEntries();
    let height = heightOf(kept);
    let top = bottom ? surface.height - margin - height : margin;
    const left = right ? surface.width - margin - width : margin;
    const obstacle = avoidedRect();
    if (obstacle !== null && overlaps({ left, top, width, height }, obstacle)) {
      const above = obstacle.top - AVOID_GAP;
      const below = obstacle.top + obstacle.height + AVOID_GAP;
      const roomAbove = above - margin;
      const roomBelow = surface.height - margin - below;
      const placeAbove = roomAbove > roomBelow || (roomAbove === roomBelow && bottom);
      const room = placeAbove ? roomAbove : roomBelow;
      while (kept.length > 0 && heightOf(kept) > room) {
        const lowest = kept.pop();
        if (lowest !== undefined) hide(lowest);
      }
      height = heightOf(kept);
      top = placeAbove ? above - height : below;
    }
    column.style.left = `${left}px`;
    column.style.top = `${top}px`;
  };

  /** Fills the places from the queue and places the column. */
  const pump = (): void => {
    if (destroyed) return;
    promote();
    place();
  };

  /** Writes the count of a notice into its badge. */
  const paintCount = (entry: Entry): void => {
    const count = entry.record.count;
    entry.badge.textContent = count > 1 ? `×${count}` : "";
    entry.badge.hidden = count <= 1;
  };

  /** Chooses a button of a notice: its callback runs, then the notice goes away. */
  const activate = (id: string, actionId: string): void => {
    const entry = entries.find((candidate) => candidate.id === id);
    if (entry === undefined) return;
    entry.onAction?.(actionId);
    dismiss(id);
  };

  /** Builds a notice's element: its text, its count badge and its buttons. */
  const buildElement = (id: string, message: string, tone: NoticeTone, actions: readonly NoticeAction[]): { el: HTMLElement; badge: HTMLElement } => {
    const el = createElement("div", "vu-notice");
    el.dataset["tone"] = tone;
    el.dataset["id"] = id;
    el.append(createElement("span", "vu-notice-text", message));
    const badge = createElement("span", "vu-notice-count");
    badge.hidden = true;
    el.append(badge);
    if (actions.length > 0) {
      const row = createElement("div", "vu-notice-actions");
      for (const action of actions) {
        const button = createElement("button", "vu-button", action.label);
        button.type = "button";
        button.dataset["action"] = action.id;
        button.addEventListener("click", () => {
          activate(id, action.id);
        });
        row.append(button);
      }
      el.append(row);
    }
    return { el, badge };
  };

  /** Takes a notice off the screen. It fades first, and its place goes to the next notice at once. */
  const dismiss = (id: string): void => {
    if (destroyed) return;
    const entry = entries.find((candidate) => candidate.id === id);
    if (entry === undefined || entry.state === "leaving") return;
    if (entry.state === "waiting") {
      forget(entry);
      return;
    }
    stopTimer(entry);
    entry.state = "leaving";
    entry.el.dataset["state"] = "leaving";
    if (fadeMs <= 0) {
      forget(entry);
    } else {
      entry.timer = setTimeout(() => {
        entry.timer = null;
        forget(entry);
        pump();
      }, fadeMs);
    }
    pump();
  };

  /**
   * Adds one to the count of a notice that is still here. A notice on screen gets its full time again; a
   * waiting one gets its full time when it is shown.
   */
  const repeat = (entry: Entry): void => {
    entry.record.count += 1;
    paintCount(entry);
    if (entry.state === "visible") {
      stopTimer(entry);
      entry.remainingMs = entry.durationMs;
      startTimer(entry);
    } else if (entry.state === "waiting") {
      entry.remainingMs = entry.durationMs;
    }
    pump();
    refreshHistory();
  };

  /** Draws the history drawer, when it is open. */
  const refreshHistory = (): void => {
    if (drawer.hidden) return;
    const rows: HTMLElement[] = history.map((item) => {
      const row = document.createElement("li");
      row.className = "vu-history-item";
      row.dataset["tone"] = item.tone;
      row.append(createElement("span", "vu-history-text", item.message));
      const count = createElement("span", "vu-history-count", item.count > 1 ? `×${item.count}` : "");
      count.hidden = item.count <= 1;
      row.append(count);
      return row;
    });
    if (rows.length === 0) {
      const empty = document.createElement("li");
      empty.className = "vu-history-empty";
      empty.textContent = text("notice.history.empty", "Nothing yet.");
      rows.push(empty);
    }
    drawerList.replaceChildren(...rows);
  };

  /** Writes the words of the drawer and of the badges in the language in use. */
  const paintTexts = (): void => {
    drawerTitle.textContent = text("notice.history.title", "History");
    closeButton.textContent = text("notice.history.close", "Close");
    drawer.setAttribute("aria-label", text("notice.history.title", "History"));
    refreshHistory();
  };

  const openDrawer = (): void => {
    if (destroyed || !drawer.hidden) return;
    returnFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    drawer.hidden = false;
    paintTexts();
    closeButton.focus();
  };

  const closeDrawer = (): void => {
    if (destroyed || drawer.hidden) return;
    drawer.hidden = true;
    const back = returnFocus;
    returnFocus = null;
    if (back !== null && back.isConnected) back.focus();
  };

  closeButton.addEventListener("click", closeDrawer);
  /** Escape closes the drawer, when the focus is inside it. */
  const onDrawerKeyDown = (event: KeyboardEvent): void => {
    if (event.key !== "Escape") return;
    event.preventDefault();
    closeDrawer();
  };
  drawer.addEventListener("keydown", onDrawerKeyDown);
  paintTexts();
  pump();

  return {
    el: column,
    get historyOpen(): boolean {
      return !drawer.hidden;
    },
    notify(spec: NoticeSpec): string {
      if (destroyed) return "";
      const duration = spec.durationMs === undefined ? defaultDuration : spec.durationMs;
      if (duration !== null && (!Number.isFinite(duration) || duration < 0)) {
        throw new RangeError(`durationMs must be a number of milliseconds from 0, or null, got ${duration}`);
      }
      if (spec.key !== undefined) {
        const existing = entries.find((entry) => entry.key === spec.key && entry.state !== "leaving");
        if (existing !== undefined) {
          repeat(existing);
          return existing.id;
        }
      }
      sequence += 1;
      const id = `notice-${sequence}`;
      const tone = spec.tone ?? "info";
      const actions = spec.actions ?? [];
      const built = buildElement(id, spec.message, tone, actions);
      const record = { id, message: spec.message, tone, count: 1 };
      const entry: Entry = {
        id,
        seq: sequence,
        key: spec.key ?? null,
        priority: spec.priority ?? 0,
        tone,
        durationMs: duration,
        onAction: spec.onAction,
        el: built.el,
        badge: built.badge,
        record,
        state: "waiting",
        remainingMs: duration,
        startedAt: 0,
        timer: null,
      };
      entries.push(entry);
      history.unshift(record);
      if (history.length > historyLimit) history.length = historyLimit;
      pump();
      refreshHistory();
      return id;
    },
    dismiss,
    clear(): void {
      if (destroyed) return;
      for (const entry of entries) stopTimer(entry);
      for (const entry of entries) entry.el.remove();
      entries = [];
    },
    visible(): readonly string[] {
      return shownEntries().map((entry) => entry.id);
    },
    history(): readonly NoticeRecord[] {
      return history.map((item) => ({ ...item }));
    },
    relayout(): void {
      pump();
    },
    openHistory: openDrawer,
    closeHistory: closeDrawer,
    toggleHistory(): void {
      if (drawer.hidden) openDrawer();
      else closeDrawer();
    },
    update(next: Partial<NoticesProps>): void {
      current = {
        anchor: next.anchor ?? current.anchor,
        avoid: next.avoid === undefined ? current.avoid : next.avoid,
        maxVisible: next.maxVisible === undefined ? current.maxVisible : checkedMaxVisible(next.maxVisible),
        t: next.t ?? current.t,
      };
      paintTexts();
      for (const entry of entries) paintCount(entry);
      pump();
    },
    destroy(): void {
      if (destroyed) return;
      destroyed = true;
      for (const entry of entries) stopTimer(entry);
      entries = [];
      history.length = 0;
      column.remove();
      drawer.removeEventListener("keydown", onDrawerKeyDown);
      drawer.remove();
    },
  };
}

/** Orders notices as they are drawn: the higher priority first, and for equal priorities the older one. */
function byDisplayOrder(a: Entry, b: Entry): number {
  return b.priority - a.priority || a.seq - b.seq;
}

/**
 * True when a box and a rectangle share some area. A box with no height or width shares none.
 * @param box - the column's box.
 * @param rect - the rectangle to avoid.
 */
function overlaps(box: LogicalRect, rect: LogicalRect): boolean {
  return (
    box.left < rect.left + rect.width &&
    box.left + box.width > rect.left &&
    box.top < rect.top + rect.height &&
    box.top + box.height > rect.top
  );
}

/** True when the player asked for less motion in the operating system. */
function prefersReducedMotion(): boolean {
  return typeof matchMedia === "function" && matchMedia("(prefers-reduced-motion: reduce)").matches;
}

/**
 * Checks a number of notices on screen.
 * @param value - the number.
 * @throws RangeError when it is not a whole number from 1.
 */
function checkedMaxVisible(value: number): number {
  return wholeNumber(value, 1, "maxVisible");
}

/**
 * Checks a whole number from a minimum.
 * @param value - the number.
 * @param minimum - the smallest value allowed.
 * @param name - the option's name, for the message.
 * @throws RangeError when the value is not a whole number from the minimum.
 */
function wholeNumber(value: number, minimum: number, name: string): number {
  if (!Number.isInteger(value) || value < minimum) {
    throw new RangeError(`${name} must be a whole number from ${minimum}, got ${value}`);
  }
  return value;
}

/**
 * Checks a positive number.
 * @param value - the number.
 * @param name - the option's name, for the message.
 * @throws RangeError when it is not a finite number above zero.
 */
function positiveNumber(value: number, name: string): number {
  if (!Number.isFinite(value) || value <= 0) throw new RangeError(`${name} must be a number above zero, got ${value}`);
  return value;
}

/**
 * Checks a number that is zero or more.
 * @param value - the number.
 * @param name - the option's name, for the message.
 * @throws RangeError when it is not a finite number from zero.
 */
function nonNegativeNumber(value: number, name: string): number {
  if (!Number.isFinite(value) || value < 0) throw new RangeError(`${name} must be a number from 0, got ${value}`);
  return value;
}
