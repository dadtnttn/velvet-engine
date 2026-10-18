// Test doubles for the gamepad tests: synthetic pads that a stubbed navigator.getGamepads returns, and
// a frame scheduler that the test runs by hand. Only the tests import this file. The package index does
// not export it.

import { vi } from "vitest";
import type { FrameScheduler } from "./loop";

/** The number of buttons in the standard mapping. */
const STANDARD_BUTTONS = 17;

/** One button of a synthetic pad. */
export interface FakeButton {
  pressed: boolean;
  value: number;
}

/** A synthetic Gamepad, with the fields that the input code reads. Tests change it between frames. */
export interface FakePad {
  index: number;
  id: string;
  connected: boolean;
  buttons: FakeButton[];
  axes: number[];
  vibrationActuator?: unknown;
}

/** Options for {@link fakePad}. */
export interface FakePadOptions {
  /** The pad's name. */
  readonly id?: string;
  /** Button values by button index. A value above 0 is a press; a digital button reads 1. */
  readonly buttons?: Readonly<Record<number, number>>;
  /** The four axes of the standard mapping: leftX, leftY, rightX, rightY. */
  readonly axes?: readonly number[];
  /** The vibration actuator, when the pad has one. */
  readonly vibrationActuator?: unknown;
}

/** Builds a synthetic pad with the standard layout: 17 buttons and 4 axes. */
export function fakePad(index: number, options: FakePadOptions = {}): FakePad {
  const buttons: FakeButton[] = [];
  for (let button = 0; button < STANDARD_BUTTONS; button += 1) {
    const value = options.buttons?.[button] ?? 0;
    buttons.push({ pressed: value > 0, value });
  }
  return {
    index,
    id: options.id ?? `Fake pad ${index}`,
    connected: true,
    buttons,
    axes: [...(options.axes ?? [0, 0, 0, 0])],
    vibrationActuator: options.vibrationActuator,
  };
}

/** The pads that the stubbed navigator reports. A null slot is an empty place, as in the browser. */
export class FakePads {
  /** The slots of the navigator's pad list. Change them between frames. */
  slots: Array<FakePad | null> = [];

  /** Makes `navigator.getGamepads` return this object's slots. Call `vi.unstubAllGlobals` afterwards. */
  install(): void {
    vi.stubGlobal("navigator", { getGamepads: () => this.slots });
  }

  /** Replaces the connected pads with these. */
  set(pads: readonly FakePad[]): void {
    this.slots = [...pads];
  }
}

/** A frame scheduler that runs its requests only when the test calls `run`. */
export interface ManualScheduler extends FrameScheduler {
  /** Runs every requested callback once, with the given time in milliseconds. */
  run(timeMs?: number): void;
  /** The number of requests that wait to run. */
  readonly pending: number;
}

/** Builds a manual frame scheduler. */
export function manualScheduler(): ManualScheduler {
  let next = 1;
  const waiting = new Map<number, (timeMs: number) => void>();
  return {
    request(callback) {
      const handle = next;
      next += 1;
      waiting.set(handle, callback);
      return handle;
    },
    cancel(handle) {
      waiting.delete(handle);
    },
    run(timeMs = 0) {
      const callbacks = [...waiting.values()];
      waiting.clear();
      for (const callback of callbacks) callback(timeMs);
    },
    get pending() {
      return waiting.size;
    },
  };
}
