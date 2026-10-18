// Fake gamepads. The core input code reads pads through `navigator.getGamepads()`, so a test or a
// replay installs this provider in its place. The provider keeps the state of each pad, and the
// game's own ActionMap reads it exactly as it would read a real pad. Reading the real pads (for a
// recording in a page) goes through `readNavigatorPads`, which takes whatever the navigator reports.

import { PAD_AXIS_NAMES, PAD_BUTTON_NAMES, type PadAxisName, type PadButtonName } from "@velvet/core";
import { TestingError } from "./errors";

/** The number of buttons in the standard mapping. The core reads the first sixteen of them by name. */
export const STANDARD_BUTTON_COUNT = 17;

/** The number of axes in the standard mapping: each stick has an x and a y. */
export const STANDARD_AXIS_COUNT = 4;

/** The highest pad index the fake provider holds. Browsers report four pads. */
const MAX_PAD_INDEX = 15;

/** One button of a pad, as the browser reports it: a digital flag and an analog value from 0 to 1. */
export interface PadButtonState {
  readonly pressed: boolean;
  readonly value: number;
}

/** The state of one connected pad: its name, its buttons and its axes. */
export interface PadState {
  readonly id: string;
  readonly buttons: readonly PadButtonState[];
  readonly axes: readonly number[];
}

/** A pad as read from the navigator, with its index. */
export interface NavigatorPad extends PadState {
  readonly index: number;
}

/** The fields of a pad that a test or a replay sets. Missing fields keep their defaults. */
export interface PadInit {
  readonly id?: string;
  readonly buttons?: readonly PadButtonState[];
  readonly axes?: readonly number[];
}

/** A pad in the fake provider. The fields follow the browser's Gamepad object. */
interface FakeGamepad {
  readonly id: string;
  readonly index: number;
  readonly connected: true;
  readonly mapping: "standard";
  readonly timestamp: number;
  readonly buttons: Array<{ pressed: boolean; touched: boolean; value: number }>;
  readonly axes: number[];
  readonly vibrationActuator: null;
}

/**
 * A fake gamepad provider. Connect and change pads between frames; `install` makes
 * `navigator.getGamepads` return them.
 */
export class FakeGamepads {
  private slots: Array<FakeGamepad | null> = [];
  private restoreCurrent: (() => void) | null = null;
  private stamp = 0;

  /** True while this provider is installed on the navigator. */
  get installed(): boolean {
    return this.restoreCurrent !== null;
  }

  /**
   * Makes `navigator.getGamepads` return this provider's pads. Several providers can be installed at once:
   * the last one installed supplies the pads, and removing any of them leaves the others in place. Calling
   * it again while installed does nothing and returns the same restore function.
   * @returns a function that removes this provider again.
   */
  install(): () => void {
    if (this.restoreCurrent !== null) return this.restoreCurrent;
    if (installedProviders.length === 0) attachNavigatorHook();
    installedProviders.push(this);
    const restore = (): void => {
      if (this.restoreCurrent !== restore) return;
      this.restoreCurrent = null;
      const index = installedProviders.indexOf(this);
      if (index >= 0) installedProviders.splice(index, 1);
      if (installedProviders.length === 0) detachNavigatorHook();
    };
    this.restoreCurrent = restore;
    return restore;
  }

  /** The pad slots as the navigator reports them: a copy, with null for an empty place. */
  padSlots(): Array<FakeGamepad | null> {
    return [...this.slots];
  }

  /**
   * Connects a pad, or replaces the whole state of a connected one. The buttons default to released and
   * the axes to rest.
   * @throws TestingError with code "bad-argument" for an index out of range or a value out of range.
   */
  connect(index: number, init: PadInit = {}): void {
    assertIndex(index);
    const buttons = init.buttons ?? [];
    const axes = init.axes ?? [];
    if (buttons.length > 32 || axes.length > 16) {
      throw new TestingError("bad-argument", "a pad has at most 32 buttons and 16 axes");
    }
    const count = Math.max(STANDARD_BUTTON_COUNT, buttons.length);
    const padButtons: Array<{ pressed: boolean; touched: boolean; value: number }> = [];
    for (let button = 0; button < count; button += 1) {
      const given = buttons[button] ?? { pressed: false, value: 0 };
      const value = clampedUnit(given.value, `button ${button}`);
      padButtons.push({ pressed: given.pressed, touched: given.pressed || value > 0, value });
    }
    const padAxes = Array.from({ length: Math.max(STANDARD_AXIS_COUNT, axes.length) }, (_, axis) =>
      clampedSigned(axes[axis] ?? 0, `axis ${axis}`),
    );
    this.stamp += 1;
    this.ensureSlot(index);
    this.slots[index] = {
      id: init.id ?? `Fake pad ${index}`,
      index,
      connected: true,
      mapping: "standard",
      timestamp: this.stamp,
      buttons: padButtons,
      axes: padAxes,
      vibrationActuator: null,
    };
  }

  /** Disconnects a pad. Its slot stays in the list, as an empty place, as in the browser. */
  disconnect(index: number): void {
    assertIndex(index);
    if (index < this.slots.length) this.slots[index] = null;
  }

  /** Disconnects every pad. */
  clear(): void {
    this.slots = [];
  }

  /**
   * Sets one button of a connected pad. A value above 0 is a press, and a digital button reads 1.
   * @throws TestingError with code "bad-argument" when the pad is not connected or the value is out of range.
   */
  setButton(index: number, button: PadButtonName, value: number): void {
    const pad = this.connectedPad(index);
    const position = PAD_BUTTON_NAMES.indexOf(button);
    const clamped = clampedUnit(value, `button ${button}`);
    pad.buttons[position] = { pressed: clamped > 0, touched: clamped > 0, value: clamped };
    this.stamp += 1;
  }

  /**
   * Sets one axis of a connected pad, from -1 to 1.
   * @throws TestingError with code "bad-argument" when the pad is not connected or the value is out of range.
   */
  setAxis(index: number, axis: PadAxisName, value: number): void {
    const pad = this.connectedPad(index);
    pad.axes[PAD_AXIS_NAMES.indexOf(axis)] = clampedSigned(value, `axis ${axis}`);
    this.stamp += 1;
  }

  /**
   * Sets both axes of one stick, from -1 to 1 each.
   * @throws TestingError with code "bad-argument" when the pad is not connected or a value is out of range.
   */
  setStick(index: number, stick: "left" | "right", x: number, y: number): void {
    const first = stick === "left" ? 0 : 2;
    const pad = this.connectedPad(index);
    pad.axes[first] = clampedSigned(x, `${stick} stick x`);
    pad.axes[first + 1] = clampedSigned(y, `${stick} stick y`);
    this.stamp += 1;
  }

  /** The state of a connected pad, or null when that pad is not connected. */
  state(index: number): PadState | null {
    const pad = this.slots[index];
    if (pad === undefined || pad === null) return null;
    return { id: pad.id, buttons: pad.buttons.map((button) => ({ pressed: button.pressed, value: button.value })), axes: [...pad.axes] };
  }

  /** The value of one axis of a connected pad, or 0 when the pad or the axis is missing. */
  axis(index: number, axis: PadAxisName): number {
    const pad = this.slots[index];
    if (pad === undefined || pad === null) return 0;
    return pad.axes[PAD_AXIS_NAMES.indexOf(axis)] ?? 0;
  }

  /** The pad that a setter changes. It must be connected. */
  private connectedPad(index: number): FakeGamepad {
    assertIndex(index);
    const pad = this.slots[index];
    if (pad === undefined || pad === null) {
      throw new TestingError("bad-argument", `pad ${index} is not connected: call connect(${index}) first`);
    }
    return pad;
  }

  /** Makes room in the slot list for an index, filling the new places with empty slots. */
  private ensureSlot(index: number): void {
    while (this.slots.length <= index) this.slots.push(null);
  }
}

/**
 * Reads the pads that `navigator.getGamepads()` reports now. Empty slots and pads that say they are
 * not connected are left out. Values are copied, so the result does not change later. Returns an empty
 * list where the browser has no gamepad API.
 */
export function readNavigatorPads(): NavigatorPad[] {
  const navigatorValue: unknown = Reflect.get(globalThis, "navigator");
  if (typeof navigatorValue !== "object" || navigatorValue === null) return [];
  const getGamepads: unknown = Reflect.get(navigatorValue, "getGamepads");
  if (typeof getGamepads !== "function") return [];
  let list: unknown;
  try {
    list = Reflect.apply(getGamepads, navigatorValue, []);
  } catch {
    return [];
  }
  if (!Array.isArray(list)) return [];
  const pads: NavigatorPad[] = [];
  for (const entry of list as unknown[]) {
    if (typeof entry !== "object" || entry === null) continue;
    if (Reflect.get(entry, "connected") === false) continue;
    const index: unknown = Reflect.get(entry, "index");
    if (typeof index !== "number" || !Number.isInteger(index) || index < 0) continue;
    const id: unknown = Reflect.get(entry, "id");
    pads.push({
      index,
      id: typeof id === "string" ? id : "",
      buttons: readButtons(Reflect.get(entry, "buttons")),
      axes: readAxes(Reflect.get(entry, "axes")),
    });
  }
  return pads;
}

/** The installed providers, in the order they were installed. The last one supplies the pads. */
const installedProviders: FakeGamepads[] = [];

/** What the navigator held before the hook was put on it, so that the last removal can put it back. */
interface NavigatorHook {
  readonly target: object;
  readonly previous: PropertyDescriptor | undefined;
  readonly created: boolean;
}

/** The hook that is on the navigator now, or null while no provider is installed. */
let navigatorHook: NavigatorHook | null = null;

/** Puts the hook on the navigator: its getGamepads returns the pads of the last installed provider. */
function attachNavigatorHook(): void {
  const { target, created } = navigatorObject();
  const previous = Object.getOwnPropertyDescriptor(target, "getGamepads");
  Object.defineProperty(target, "getGamepads", {
    value: (): Array<FakeGamepad | null> => {
      const top = installedProviders[installedProviders.length - 1];
      return top === undefined ? [] : top.padSlots();
    },
    configurable: true,
    writable: true,
    enumerable: false,
  });
  navigatorHook = { target, previous, created };
}

/** Takes the hook off the navigator, and puts back what was there before it. */
function detachNavigatorHook(): void {
  const hook = navigatorHook;
  if (hook === null) return;
  if (hook.previous !== undefined) Object.defineProperty(hook.target, "getGamepads", hook.previous);
  else Reflect.deleteProperty(hook.target, "getGamepads");
  if (hook.created) Reflect.deleteProperty(globalThis, "navigator");
  navigatorHook = null;
}

/** The navigator object to patch, and whether this call had to create it. */
function navigatorObject(): { target: object; created: boolean } {
  const existing: unknown = Reflect.get(globalThis, "navigator");
  if (typeof existing === "object" && existing !== null) return { target: existing, created: false };
  const created: object = {};
  Object.defineProperty(globalThis, "navigator", { value: created, configurable: true, writable: true });
  return { target: created, created: true };
}

/** Copies a list of buttons from a browser pad. A missing or odd value reads as released. */
function readButtons(value: unknown): PadButtonState[] {
  if (!Array.isArray(value)) return [];
  return (value as unknown[]).map((entry) => {
    const pressed = typeof entry === "object" && entry !== null && Reflect.get(entry, "pressed") === true;
    const raw: unknown = typeof entry === "object" && entry !== null ? Reflect.get(entry, "value") : 0;
    const level = typeof raw === "number" && Number.isFinite(raw) ? Math.min(1, Math.max(0, raw)) : 0;
    return { pressed, value: level };
  });
}

/** Copies a list of axes from a browser pad. A missing or odd value reads as rest. */
function readAxes(value: unknown): number[] {
  if (!Array.isArray(value)) return [];
  return (value as unknown[]).map((entry) =>
    typeof entry === "number" && Number.isFinite(entry) ? Math.min(1, Math.max(-1, entry)) : 0,
  );
}

/** Throws unless the pad index is a whole number in the range the provider holds. */
function assertIndex(index: number): void {
  if (!Number.isInteger(index) || index < 0 || index > MAX_PAD_INDEX) {
    throw new TestingError("bad-argument", `a pad index is a whole number from 0 to ${MAX_PAD_INDEX}, got ${index}`);
  }
}

/** A value from 0 to 1, or a bad-argument error that names the control. */
function clampedUnit(value: number, what: string): number {
  if (!Number.isFinite(value) || value < 0 || value > 1) {
    throw new TestingError("bad-argument", `${what} needs a value from 0 to 1, got ${value}`);
  }
  return value;
}

/** A value from -1 to 1, or a bad-argument error that names the control. */
function clampedSigned(value: number, what: string): number {
  if (!Number.isFinite(value) || value < -1 || value > 1) {
    throw new TestingError("bad-argument", `${what} needs a value from -1 to 1, got ${value}`);
  }
  return value;
}
