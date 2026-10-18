// A rebinding port for a UI: the slots of an action, the capture of the next input into one slot,
// and the clearing and resetting of bindings, as plain data and functions.
//
// The shape matches the `RebindPort` that @velvet/ui's createRebindPanel takes. It is built here,
// with the same member names and types, so that the UI package needs no import from core. A game
// does `createRebindPanel({ port: createRebindPort(map), ... })`.

import { describeBinding } from "./input-controls";
import type { FrameScheduler } from "./loop";
import type { ActionMap, Binding, BindingDisplay, ConflictPolicy, InputDevice, RebindConflict, RebindStatus } from "./input";

/** One slot of an action, as a UI shows it. `display` is null for an empty slot. */
export interface RebindPortSlot {
  readonly slot: number;
  readonly display: BindingDisplay | null;
}

/** What a UI asks of one capture. */
export interface RebindCaptureRequest {
  readonly devices: readonly InputDevice[];
  readonly timeoutMs: number;
  readonly policy: ConflictPolicy;
  readonly signal: AbortSignal;
}

/** The result of one capture that was bound. `display` is the input that was captured. */
export interface RebindPortOutcome<Action extends string> {
  readonly status: RebindStatus;
  readonly conflicts: readonly RebindConflict<Action>[];
  readonly display: BindingDisplay;
}

/** The rebinding operations that a UI drives. */
export interface RebindPort<Action extends string> {
  /** The slots of an action, in order. */
  slots(action: Action): readonly RebindPortSlot[];
  /**
   * Waits for the next input and binds it to the slot. Resolves with null when the capture was
   * cancelled, timed out or aborted, and with the outcome otherwise.
   */
  capture(action: Action, slot: number, request: RebindCaptureRequest): Promise<RebindPortOutcome<Action> | null>;
  /** Empties a slot. */
  clear(action: Action, slot: number): void;
  /** Restores the defaults of one action. */
  resetAction(action: Action): void;
  /** Restores the defaults of every action. */
  resetAll(): void;
}

/** Options for {@link createRebindPort}: where a capture listens and where it polls the gamepads. */
export interface RebindPortOptions {
  /** Where the gamepads are polled. Default: the browser's frames. */
  readonly scheduler?: FrameScheduler;
  /** Where keys and mouse buttons are listened to. Default: window. */
  readonly target?: EventTarget | null;
}

/**
 * A rebinding port over an action map. A captured input keeps the axis component and the context of
 * the slot it goes into. A captured axis takes the direction of the push as its scale, and any other
 * input keeps the slot's scale (1 for an empty slot).
 */
export function createRebindPort<Action extends string>(
  map: ActionMap<Action>,
  options: RebindPortOptions = {},
): RebindPort<Action> {
  return {
    slots(action) {
      return map.slotsOf(action).map((binding, slot) => ({
        slot,
        display: binding === null ? null : describeBinding(binding),
      }));
    },
    async capture(action, slot, request) {
      const captured = await map.captureNext({
        devices: request.devices,
        timeoutMs: request.timeoutMs,
        signal: request.signal,
        scheduler: options.scheduler,
        target: options.target,
      });
      if (captured === null) return null;
      const current = map.slotsOf(action)[slot] ?? null;
      const binding: Binding<Action> = {
        action,
        control: captured.control,
        scale: captured.sign ?? current?.scale ?? 1,
        axisComponent: current?.axisComponent,
        context: current?.context,
      };
      const outcome = map.rebind(action, slot, binding, { policy: request.policy });
      return { status: outcome.status, conflicts: outcome.conflicts, display: describeBinding(binding) };
    },
    clear(action, slot) {
      map.clearSlot(action, slot);
    },
    resetAction(action) {
      map.resetAction(action);
    },
    resetAll() {
      map.resetAll();
    },
  };
}
