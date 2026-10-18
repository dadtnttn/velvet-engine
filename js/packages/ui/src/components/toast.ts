// Toasts: short notices that fade away by themselves.
//
// Notices queue up. At most `maxVisible` show at once. A notice stays for `duration`
// milliseconds, then fades over `fadeMs` and is removed. The next queued notice then takes its
// place. A notice is shown in the order it was queued. The toasts never take clicks, so they
// never block the game under them.

import { mergeDefined, type VuComponent } from "./component";
import { createElement } from "./dom";

/** The kind of a notice. It sets the colour of the notice. */
export type ToastTone = "info" | "warn" | "danger";

/** Props of a toast queue. They apply to the notices that show from now on. */
export interface ToastProps {
  /** Most notices on screen at once. Default 3. Must be a whole number from 1. */
  readonly maxVisible?: number;
  /** How long a notice stays before it starts to fade, in milliseconds. Default 3000. */
  readonly duration?: number;
  /** How long the fade takes, in milliseconds. Default 300. Zero when the player prefers reduced motion. */
  readonly fadeMs?: number;
}

/** A toast queue. Call `show` to add a notice. */
export interface Toast extends VuComponent<ToastProps> {
  /** Adds a notice to the queue. It shows as soon as a slot is free. */
  show(message: string, tone?: ToastTone): void;
}

/** A timer handle from setTimeout. */
type TimerHandle = ReturnType<typeof setTimeout>;

/** A notice waiting in the queue. */
interface Notice {
  readonly message: string;
  readonly tone: ToastTone;
}

/** A notice on screen, with the timers that will fade and remove it. */
interface Showing {
  readonly el: HTMLElement;
  readonly fadeMs: number;
  hideTimer: TimerHandle | null;
  removeTimer: TimerHandle | null;
}

const DEFAULTS: ToastProps = { maxVisible: 3, duration: 3000, fadeMs: 300 };

/**
 * Creates a toast queue. Append its `el` to the stage's root.
 * @param props - the starting props.
 * @throws RangeError when `maxVisible` is not a whole number from 1, or a time is negative.
 */
export function createToast(props: ToastProps = {}): Toast {
  const el = createElement("div", "vu-toasts");
  el.setAttribute("role", "status");
  el.setAttribute("aria-live", "polite");
  let config = mergeDefined(DEFAULTS, props);
  checkConfig(config);
  const queue: Notice[] = [];
  let showing: Showing[] = [];
  let destroyed = false;

  /** Starts as many queued notices as there are free slots. */
  const pump = (): void => {
    while (showing.length < (config.maxVisible ?? 0)) {
      const next = queue.shift();
      if (next === undefined) break;
      start(next);
    }
  };

  const start = (notice: Notice): void => {
    const node = createElement("div", "vu-toast", notice.message);
    node.setAttribute("data-tone", notice.tone);
    el.append(node);
    const fadeMs = prefersReducedMotion() ? 0 : (config.fadeMs ?? 0);
    const entry: Showing = { el: node, fadeMs, hideTimer: null, removeTimer: null };
    showing.push(entry);
    entry.hideTimer = setTimeout(() => fade(entry), config.duration ?? 0);
  };

  const fade = (entry: Showing): void => {
    entry.hideTimer = null;
    if (entry.fadeMs <= 0) {
      remove(entry);
      return;
    }
    entry.el.classList.add("is-leaving");
    entry.removeTimer = setTimeout(() => remove(entry), entry.fadeMs);
  };

  const remove = (entry: Showing): void => {
    if (entry.hideTimer !== null) clearTimeout(entry.hideTimer);
    if (entry.removeTimer !== null) clearTimeout(entry.removeTimer);
    entry.hideTimer = null;
    entry.removeTimer = null;
    entry.el.remove();
    showing = showing.filter((other) => other !== entry);
    pump();
  };

  return {
    el,
    show(message: string, tone: ToastTone = "info"): void {
      if (destroyed) return;
      queue.push({ message, tone });
      pump();
    },
    update(next: Partial<ToastProps>): void {
      const merged = mergeDefined(config, next);
      checkConfig(merged);
      config = merged;
      pump();
    },
    destroy(): void {
      destroyed = true;
      for (const entry of showing) {
        if (entry.hideTimer !== null) clearTimeout(entry.hideTimer);
        if (entry.removeTimer !== null) clearTimeout(entry.removeTimer);
      }
      showing = [];
      queue.length = 0;
      el.remove();
    },
  };
}

/** True when the player asked for less motion in the operating system. */
function prefersReducedMotion(): boolean {
  return typeof matchMedia === "function" && matchMedia("(prefers-reduced-motion: reduce)").matches;
}

/** Throws when the queue settings cannot work. */
function checkConfig(config: ToastProps): void {
  const maxVisible = config.maxVisible ?? 0;
  if (!Number.isInteger(maxVisible) || maxVisible < 1) {
    throw new RangeError(`maxVisible must be a whole number from 1, got ${maxVisible}`);
  }
  for (const [name, value] of [
    ["duration", config.duration],
    ["fadeMs", config.fadeMs],
  ] as const) {
    if (value === undefined || !Number.isFinite(value) || value < 0) {
      throw new RangeError(`${name} must be a number of milliseconds from 0, got ${value}`);
    }
  }
}
