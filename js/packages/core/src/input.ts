// Action input for games. Keys, mouse buttons, the mouse wheel and gamepads are bound to named
// actions, and game code asks about the actions instead of the raw inputs.
//
// The model follows crates/velvet-input where it applies:
// - an action is a name; the game types its names through the Action parameter;
// - a Binding ties a control to an action, with a scale (default 1) and, for a 2D action
//   such as "move", an axisComponent ("x" or "y") that makes the binding feed one component;
// - a digital action is active when its value reaches the threshold (0.5 by default), and a
//   2D action is active when the length of its vector does.
//
// Events update the state as they arrive, between frames. In each frame the game reads
// `pressed` (active since the last endFrame), `held` (active now) and `released` (inactive
// since the last endFrame). Call endFrame() once per frame, after the reads: it clears the
// edges and the wheel and pointer deltas, and then reads the gamepads. A pad's change shows
// up in the next frame, just as a key event does.
//
// A wheel tick is a one-frame pulse. Its action is pressed and held in the frame the tick
// arrives, and released in the next frame.
//
// Gamepads use the standard mapping (see input-pads.ts). A pad control reads a button, an axis
// or a stick direction. A button is active when its level reaches the control's threshold, and
// an axis or a stick has a deadzone of its own. `value` gives the analog strength of an action
// from -1 to 1, so a trigger pulled halfway reads 0.5 even though the action is not yet active.
//
// Slots. Every binding has a slot: a number that names its place in the action's list. Slots keep
// their numbers when one is rebound or cleared, so a saved file can name them (see
// input-store.ts). Rebinding (`rebind`) puts a control into a slot, and it applies a conflict
// policy when the control is already used by another action in the same context.
//
// Keys are KeyboardEvent.code values, which name physical keys whatever the layout. Pointer
// events cover mouse, pen and touch; a touch counts as the left button.
//
// Scope and handled events. A map listens on the target it is attached to: window for the whole
// page, or an element, which then sees only the events inside it. A key held inside such an
// element is released when focus leaves it, because its key up would go elsewhere.
//
// A press is a keydown, a pointerdown or a wheel tick. A press does not count when:
//   - its target is a control: an input, textarea, select, button, [contenteditable] element or
//     [role="menuitem"]. A control keeps its own keys: Enter on a focused button clicks it, and a
//     text field receives its letters. The captureControls option lifts this for one map;
//   - its defaultPrevented flag is already set when the map sees it. Only a preventDefault that
//     ran before the map counts, which is the case for a listener on the element the event went
//     to. A listener on window that runs after the map does not affect it;
//   - markHandled(event) was called for it, which is the way to use an event without preventDefault.
// Releases (keyup, pointerup, pointercancel) and blur always count, so a press that counted is
// always released. Pointer moves always update the pointer position, whatever their target.
//
// consume(action) hides an action for the rest of the frame, so that one part of the game can
// take an action that no later part should see.

import { EventEmitter, type Unsubscribe } from "./events";
import { startCapture, type CaptureSettings } from "./input-capture";
import { controlKey, isHandled } from "./input-controls";
import {
  DEFAULT_PAD_THRESHOLD,
  buttonLevel,
  clampUnit,
  padValue,
  readGamepads,
  rumblePads,
  type PadReading,
  type RumbleOptions,
} from "./input-pads";
import { findConflictGroups, makeStored, planRebind, type StoredBinding } from "./input-rebind";
import { createFrameScheduler, type FrameScheduler } from "./loop";

export { markHandled } from "./input-controls";
export type { RumbleOptions } from "./input-pads";

/** A KeyboardEvent.code value: a physical key such as "KeyW", "Space" or "ArrowUp". */
export type KeyCode = string;

/** Mouse buttons by name. The DOM button numbers 0, 1 and 2 are left, middle and right. */
export type MouseButtonName = "left" | "middle" | "right";

/** Wheel directions: "up" and "down" for the vertical wheel, "left" and "right" for horizontal scroll. */
export type WheelDirection = "up" | "down" | "left" | "right";

/** Buttons of the standard gamepad mapping, by name. */
export type PadButtonName =
  | "a"
  | "b"
  | "x"
  | "y"
  | "leftShoulder"
  | "rightShoulder"
  | "leftTrigger"
  | "rightTrigger"
  | "select"
  | "start"
  | "leftStick"
  | "rightStick"
  | "dpadUp"
  | "dpadDown"
  | "dpadLeft"
  | "dpadRight";

/** The axes of the standard gamepad mapping: each stick has an x and a y. */
export type PadAxisName = "leftX" | "leftY" | "rightX" | "rightY";

/** The two sticks. */
export type PadStickName = "left" | "right";

/** The four directions of a stick, for bindings that read a stick as four buttons. */
export type PadDirection = "up" | "down" | "left" | "right";

/** The devices that the map tracks: the keyboard, the mouse (pointer and wheel) and gamepads. */
export type InputDevice = "keyboard" | "mouse" | "gamepad";

/**
 * A gamepad control. `pad` is the index of one pad, or undefined for any connected pad. `threshold`
 * is the level at which the control counts as active (0.5 by default). `deadzone` applies to axes
 * and sticks only (0.2 by default).
 */
export type PadControl =
  | { readonly type: "pad-button"; readonly button: PadButtonName; readonly pad?: number; readonly threshold?: number }
  | {
      readonly type: "pad-axis";
      readonly axis: PadAxisName;
      readonly pad?: number;
      readonly deadzone?: number;
      readonly threshold?: number;
    }
  | {
      readonly type: "pad-stick";
      readonly stick: PadStickName;
      readonly direction: PadDirection;
      readonly pad?: number;
      readonly deadzone?: number;
      readonly threshold?: number;
    };

/** A physical control that a binding listens to. */
export type Control =
  | { readonly type: "key"; readonly key: KeyCode }
  | { readonly type: "mouse"; readonly button: MouseButtonName }
  | { readonly type: "wheel"; readonly direction: WheelDirection }
  | PadControl;

/** Connects one control to one action. */
export interface Binding<Action extends string = string> {
  /** The action that the binding feeds. */
  readonly action: Action;
  /** The physical control. */
  readonly control: Control;
  /** How much the control contributes. Default 1. A negative value pulls an axis the other way. */
  readonly scale?: number;
  /** Turns the action into a 2D axis: this binding feeds its "x" or its "y" component. */
  readonly axisComponent?: "x" | "y";
  /**
   * The slot that the binding fills: a whole number from 0 up, unique among the action's bindings.
   * Default: the next number after the action's highest slot.
   */
  readonly slot?: number;
  /**
   * The context of the binding, for conflict checks (see {@link ConflictPolicy}). Two actions
   * conflict only when they share a context. Default "default".
   */
  readonly context?: string;
}

/** A 2D vector, used for axis values, wheel deltas and pointer positions. */
export interface Vector2 {
  readonly x: number;
  readonly y: number;
}

/** Options for {@link ActionMap}. */
export interface ActionMapOptions {
  /** The least value, or vector length, at which an action counts as active. Default 0.5. */
  threshold?: number;
  /**
   * Counts presses whose target is a control, such as a button or a text field. Default false,
   * which leaves a control's own keys to the control. Turn it on for keys that work while a control
   * has the focus, such as a game's function keys.
   */
  captureControls?: boolean;
}

/**
 * What a rebinding does when its control is already bound to another action in the same context:
 * - "swap": the other action takes the control that this slot held (or loses the control when the
 *   slot was empty);
 * - "refuse": nothing changes;
 * - "allow": both actions keep the control.
 */
export type ConflictPolicy = "swap" | "refuse" | "allow";

/** A slot that a rebinding found in conflict. */
export interface RebindConflict<Action extends string = string> {
  readonly action: Action;
  readonly slot: number;
}

/** How a rebinding ended: "bound" (no conflict), "swapped", "allowed" or "refused" (nothing changed). */
export type RebindStatus = "bound" | "swapped" | "allowed" | "refused";

/** The result of {@link ActionMap.rebind}. */
export interface RebindOutcome<Action extends string = string> {
  readonly status: RebindStatus;
  /** The slots that held the same control in the same context. Empty when there was no conflict. */
  readonly conflicts: readonly RebindConflict<Action>[];
}

/** Bindings that share a context and a control, for {@link ActionMap.conflicts}. */
export interface ConflictGroup<Action extends string = string> {
  readonly context: string;
  /** The conflict key of the control (see `conflictKeyOf`). */
  readonly control: string;
  readonly entries: readonly RebindConflict<Action>[];
}

/**
 * How a UI shows a binding: the device, a short code, and the i18n key id of its name. The key ids
 * are stable (`input.key.Space`, `input.pad.a`, `input.mouse.left`), and the game supplies their
 * texts. `negative` is set on an axis that is inverted (a negative scale).
 */
export interface BindingDisplay {
  readonly device: InputDevice;
  readonly code: string;
  readonly labelKey: string;
  readonly negative?: boolean;
}

/** The input that a capture took. `sign` is set for an axis: +1 or -1, the way the stick moved. */
export interface CapturedInput {
  readonly device: InputDevice;
  readonly control: Control;
  readonly sign?: 1 | -1;
}

/** Options for {@link ActionMap.captureNext}. */
export interface CaptureOptions {
  /** The devices that may answer. Default: keyboard, mouse (buttons only) and gamepad. */
  readonly devices?: readonly InputDevice[];
  /** Milliseconds before the capture gives up and resolves with null. Default 8000. Infinity never gives up. */
  readonly timeoutMs?: number;
  /** Keys that cancel the capture. Default ["Escape"]. A cancelled capture resolves with null. */
  readonly cancelKeys?: readonly KeyCode[];
  /** Aborting the signal cancels the capture. */
  readonly signal?: AbortSignal;
  /** Where keys and mouse buttons are listened to. Default: window, or none where there is no window. */
  readonly target?: EventTarget | null;
  /** Where gamepads are polled, one request per frame. Default: the browser's frames, or timers. */
  readonly scheduler?: FrameScheduler;
}

/** A gamepad that was connected or disconnected. */
export interface GamepadChange {
  readonly type: "connected" | "disconnected";
  readonly index: number;
  readonly id: string;
}

/** A connected gamepad. */
export interface GamepadInfo {
  readonly index: number;
  readonly id: string;
}

/** The events of an action map. */
interface InputEvents {
  device: InputDevice;
  gamepad: GamepadChange;
}

const DEFAULT_THRESHOLD = 0.5;

/** The default timeout of a capture, in milliseconds. */
const DEFAULT_CAPTURE_TIMEOUT_MS = 8000;

/** The default cancel key of a capture. */
const DEFAULT_CANCEL_KEYS: readonly KeyCode[] = ["Escape"];

/** Every device, which a capture listens to by default. */
const ALL_DEVICES: readonly InputDevice[] = ["keyboard", "mouse", "gamepad"];

/** The policy of a rebinding that names none. */
const DEFAULT_POLICY: ConflictPolicy = "swap";

/** Button names for PointerEvent.button values. */
const BUTTON_NAMES: Readonly<Record<number, MouseButtonName>> = { 0: "left", 1: "middle", 2: "right" };

/** WheelEvent.deltaMode values for lines and pages. Pixels are 0. */
const DELTA_MODE_LINE = 1;
const DELTA_MODE_PAGE = 2;

/** Approximate pixels per wheel line and per wheel page, used when a browser reports lines or pages. */
const PIXELS_PER_LINE = 16;
const PIXELS_PER_PAGE = 100;

/** The stick or trigger level from which a change of device counts as use of the gamepad. */
const DEVICE_PUSH_LEVEL = 0.5;

/**
 * The controls that keep their own keys. A press whose target is one of these, or is inside one,
 * does not count, unless the map was made with captureControls.
 */
const CONTROL_SELECTOR =
  'input, textarea, select, button, [contenteditable]:not([contenteditable="false"]), [role="menuitem"]';

/** A binding in the form the map keeps it: the stored binding, see input-rebind.ts. */
type Stored<Action extends string> = StoredBinding<Action>;

/** The value of an action at this moment. */
interface Sample {
  readonly active: boolean;
  readonly vector: Vector2;
}

/** The listeners that one attach call added to a target, kept so that detach can remove them. */
interface Attachment {
  readonly target: EventTarget;
  readonly listeners: Array<[string, EventListener]>;
}

/**
 * Maps physical controls to named actions and tracks the per-frame state of those actions.
 *
 * @typeParam Action - the action names, usually a union of string literals.
 */
export class ActionMap<Action extends string = string> {
  private readonly threshold: number;
  private readonly captureControls: boolean;
  private bindings: Stored<Action>[] = [];
  /** The bindings that the map was made with. `resetAction` and `resetAll` restore them. */
  private defaults: Stored<Action>[] = [];
  private readonly downControls = new Set<string>();
  private readonly pulseControls = new Set<string>();
  private readonly pressedActions = new Set<Action>();
  private readonly releasedActions = new Set<Action>();
  /** The actions that the game consumed in this frame. They read as idle until endFrame. */
  private readonly consumedActions = new Set<Action>();
  private wheelX = 0;
  private wheelY = 0;
  private pointerX = 0;
  private pointerY = 0;
  private pointerDeltaX = 0;
  private pointerDeltaY = 0;
  private hasPointer = false;
  private attachment: Attachment | null = null;
  /** The gamepads read at the last endFrame. */
  private pads: readonly PadReading[] = [];
  private device: InputDevice = "keyboard";
  private readonly events = new EventEmitter<InputEvents>();
  /** Cancels the capture in progress, if there is one. */
  private stopCapture: (() => void) | null = null;

  /**
   * @param bindings - the bindings to start with. They are the defaults that `resetAction` restores.
   * @param options - see {@link ActionMapOptions}.
   * @throws RangeError when the threshold is not a positive finite number, or a binding is not valid.
   */
  constructor(bindings: readonly Binding<Action>[] = [], options: ActionMapOptions = {}) {
    const threshold = options.threshold ?? DEFAULT_THRESHOLD;
    if (!Number.isFinite(threshold) || threshold <= 0) {
      throw new RangeError(`threshold must be a positive number, got ${threshold}`);
    }
    this.threshold = threshold;
    this.captureControls = options.captureControls ?? false;
    for (const binding of bindings) this.bind(binding);
    this.defaults = [...this.bindings];
  }

  /**
   * Adds a binding in the next free slot of its action, or in the slot it names. When its control is
   * already down, the action becomes active at once and counts as pressed.
   * @throws RangeError when the scale is not a finite number, the control is not valid, or the slot
   * is taken.
   */
  bind(binding: Binding<Action>): void {
    const slot = binding.slot ?? this.nextSlot(binding.action);
    assertSlot(slot);
    if (binding.slot !== undefined && this.bindings.some((entry) => entry.action === binding.action && entry.slot === slot)) {
      throw new RangeError(`slot ${slot} of "${binding.action}" already holds a binding`);
    }
    const stored = makeStored(binding, slot);
    const before = this.snapshot([binding.action]);
    this.bindings = [...this.bindings, stored];
    this.recordEdges(before);
  }

  /** Removes every binding of `action`. An action that was active becomes released. */
  unbind(action: Action): void {
    const before = this.snapshot([action]);
    this.bindings = this.bindings.filter((binding) => binding.action !== action);
    this.recordEdges(before);
  }

  /**
   * Listens for keyboard, pointer and wheel events on `target`: usually `window`, or an element to
   * see only the events inside it (see the rule at the top of this file). Attaching again moves the
   * listeners: the previous target is detached first.
   */
  attach(target: EventTarget): void {
    this.detach();
    const listeners: Array<[string, EventListener]> = [
      ["keydown", (event) => this.onKey(event, true)],
      ["keyup", (event) => this.onKey(event, false)],
      ["pointerdown", (event) => this.onPointerButton(event, true)],
      ["pointerup", (event) => this.onPointerButton(event, false)],
      ["pointermove", (event) => this.onPointerMove(event)],
      ["pointercancel", () => this.releaseWhere(isMouseControl)],
      ["wheel", (event) => this.onWheel(event)],
      ["blur", () => this.releaseWhere(() => true)],
    ];
    if (isNode(target)) {
      // Focus that leaves a scoping element takes its held keys with it: their key ups go elsewhere.
      const scope: Node = target;
      listeners.push(["focusout", (event) => this.onFocusOut(event, scope)]);
    }
    for (const [type, listener] of listeners) target.addEventListener(type, listener);
    this.attachment = { target, listeners };
  }

  /** Stops listening, and releases every control that is down so that no action stays stuck. */
  detach(): void {
    const attachment = this.attachment;
    if (attachment === null) return;
    for (const [type, listener] of attachment.listeners) {
      attachment.target.removeEventListener(type, listener);
    }
    this.attachment = null;
    this.releaseWhere(() => true);
  }

  /** True on the frame in which the action became active (since the last endFrame). */
  pressed(action: Action): boolean {
    return !this.consumedActions.has(action) && this.pressedActions.has(action);
  }

  /** True while the action is active. */
  held(action: Action): boolean {
    return !this.consumedActions.has(action) && this.isActive(action);
  }

  /** True on the frame in which the action stopped being active (since the last endFrame). */
  released(action: Action): boolean {
    return !this.consumedActions.has(action) && this.releasedActions.has(action);
  }

  /** The value of a digital action in [-1, 1], or the x component of a 2D action. Zero once consumed. */
  axis1(action: Action): number {
    return this.consumedActions.has(action) ? 0 : this.sample(action).vector.x;
  }

  /** The value of a 2D action. For a digital action this is `{ x: value, y: 0 }`. Zero once consumed. */
  axis2(action: Action): Vector2 {
    if (this.consumedActions.has(action)) return { x: 0, y: 0 };
    const { x, y } = this.sample(action).vector;
    return { x, y };
  }

  /**
   * The analog strength of an action, from -1 to 1, whether or not it is active. Each binding gives
   * its control's level times its scale: a key gives 1 while it is down, a trigger gives its pull,
   * and a stick gives its push (negative for an inverted binding). The strongest value by magnitude
   * wins. For a 2D action this is the strongest component; use `axis2` for the vector. Zero once
   * consumed.
   */
  value(action: Action): number {
    if (this.consumedActions.has(action)) return 0;
    let strongest = 0;
    for (const binding of this.bindings) {
      if (binding.action !== action) continue;
      const { analog } = this.read(binding);
      if (Math.abs(analog) > Math.abs(strongest)) strongest = analog;
    }
    return clampUnit(strongest);
  }

  /**
   * Marks an action as handled for the rest of this frame. Until endFrame, `pressed`, `held` and
   * `released` read false, and the axis values read zero. The control state is kept, so a key that
   * is still down is still held in the next frame. Use it when one part of the game has acted on an
   * action, so that no later part acts on it again.
   */
  consume(action: Action): void {
    this.consumedActions.add(action);
  }

  /**
   * The wheel movement since the last endFrame, in pixels. As in the DOM, y is positive when
   * the wheel moves down.
   */
  wheelDelta(): Vector2 {
    return { x: this.wheelX, y: this.wheelY };
  }

  /** The latest pointer position in client (CSS) pixels. Zero before the first pointer event. */
  pointer(): Vector2 {
    return { x: this.pointerX, y: this.pointerY };
  }

  /** The pointer movement since the last endFrame, in client pixels. */
  pointerDelta(): Vector2 {
    return { x: this.pointerDeltaX, y: this.pointerDeltaY };
  }

  /**
   * Clears the edges, the consumed actions, and the wheel and pointer deltas, then reads the
   * gamepads. Call it once per frame, after the frame has read the input.
   */
  endFrame(): void {
    this.pressedActions.clear();
    this.releasedActions.clear();
    this.consumedActions.clear();
    this.wheelX = 0;
    this.wheelY = 0;
    this.pointerDeltaX = 0;
    this.pointerDeltaY = 0;
    // Wheel pulses end here. Their release is recorded after the clears above, so the next
    // frame reports it as released.
    for (const id of [...this.pulseControls]) {
      this.pulseControls.delete(id);
      this.setControl(id, false);
    }
    this.poll();
  }

  /** The device that the player used last: "keyboard" until something else is used. */
  lastDevice(): InputDevice {
    return this.device;
  }

  /**
   * Runs `listener` when the device that the player uses changes, with the new device. It is not
   * called for each event, only when the device changes.
   * @returns a function that removes the listener.
   */
  onDeviceChange(listener: (device: InputDevice) => void): Unsubscribe {
    return this.events.on("device", listener);
  }

  /** The gamepads connected at the last endFrame. */
  connectedGamepads(): GamepadInfo[] {
    return this.pads.map((pad) => ({ index: pad.index, id: pad.id }));
  }

  /**
   * Runs `listener` when a gamepad is connected or disconnected. The check happens at endFrame, so a
   * change is seen within a frame.
   * @returns a function that removes the listener.
   */
  onGamepadChange(listener: (change: GamepadChange) => void): Unsubscribe {
    return this.events.on("gamepad", listener);
  }

  /**
   * Runs a rumble effect on a gamepad. Resolves to true when the browser played it, and to false
   * when no connected pad can vibrate, or the browser refuses. It never rejects.
   */
  rumble(options: RumbleOptions = {}): Promise<boolean> {
    return rumblePads(this.pads, options);
  }

  /** The names of the actions that the map knows: those with a binding now, or in its defaults. */
  actions(): Action[] {
    const seen = new Set<Action>();
    for (const entry of [...this.defaults, ...this.bindings]) seen.add(entry.action);
    return [...seen];
  }

  /**
   * The slots of an action, by number. Slot 0 is first. An empty slot is null, and a slot past the
   * last binding is not in the list.
   */
  slotsOf(action: Action): (Binding<Action> | null)[] {
    const entries = this.bindings.filter((entry) => entry.action === action);
    const count = entries.reduce((most, entry) => Math.max(most, entry.slot + 1), 0);
    const slots: (Binding<Action> | null)[] = Array.from({ length: count }, () => null);
    for (const entry of entries) slots[entry.slot] = toBinding(entry);
    return slots;
  }

  /**
   * Puts a control into a slot of an action, replacing what the slot held. The conflict policy
   * decides what happens when the control is already bound to another action in the same context:
   * "swap" (the default), "refuse" or "allow" (see {@link ConflictPolicy}). The slot may be past the
   * last one, which adds it.
   * @throws RangeError when the slot is not a whole number from 0 up, the binding is for another
   * action, or its control or scale is not valid.
   */
  rebind(
    action: Action,
    slot: number,
    binding: Binding<Action>,
    options: { readonly policy?: ConflictPolicy } = {},
  ): RebindOutcome<Action> {
    assertSlot(slot);
    if (binding.action !== action) {
      throw new RangeError(`the binding is for "${binding.action}", not "${action}"`);
    }
    const plan = planRebind(this.bindings, makeStored(binding, slot), options.policy ?? DEFAULT_POLICY);
    this.replaceBindings(plan.entries);
    return plan.outcome;
  }

  /** Empties a slot. Does nothing when the slot is already empty. */
  clearSlot(action: Action, slot: number): void {
    assertSlot(slot);
    if (!this.bindings.some((entry) => entry.action === action && entry.slot === slot)) return;
    this.replaceBindings(this.bindings.filter((entry) => !(entry.action === action && entry.slot === slot)));
  }

  /**
   * Restores the default bindings of one action: its slots go back to the bindings that the map was
   * made with. Conflicts that the defaults bring back are not resolved; see `conflicts`.
   */
  resetAction(action: Action): void {
    this.replaceBindings([
      ...this.bindings.filter((entry) => entry.action !== action),
      ...this.defaults.filter((entry) => entry.action === action),
    ]);
  }

  /** Restores the default bindings of every action. */
  resetAll(): void {
    this.replaceBindings([...this.defaults]);
  }

  /** The groups of bindings that conflict: two or more actions on one control in one context. */
  conflicts(): ConflictGroup<Action>[] {
    return findConflictGroups(this.bindings);
  }

  /**
   * Waits for the next key, mouse button or gamepad input, for a rebinding screen. The capture takes
   * the input before the page and the game see it, so the key that is being bound does not also act.
   * Resolves with the input, or with null when the capture is cancelled (by a cancel key, by
   * `cancelCapture` or by the signal), or times out. Starting a capture cancels the one in progress.
   * @throws RangeError when the timeout is not above 0.
   */
  captureNext(options: CaptureOptions = {}): Promise<CapturedInput | null> {
    const timeoutMs = options.timeoutMs ?? DEFAULT_CAPTURE_TIMEOUT_MS;
    if (!(timeoutMs > 0)) throw new RangeError(`timeoutMs must be above 0, got ${timeoutMs}`);
    this.cancelCapture();
    const settings: CaptureSettings = {
      devices: options.devices ?? ALL_DEVICES,
      timeoutMs,
      cancelKeys: options.cancelKeys ?? DEFAULT_CANCEL_KEYS,
      signal: options.signal,
      target: options.target === undefined ? defaultTarget() : options.target,
      scheduler: options.scheduler ?? createFrameScheduler(),
    };
    const capture = startCapture(settings);
    this.stopCapture = capture.cancel;
    return capture.result.then((value) => {
      if (this.stopCapture === capture.cancel) this.stopCapture = null;
      return value;
    });
  }

  /** Cancels the capture in progress. Its promise resolves with null. Does nothing when none runs. */
  cancelCapture(): void {
    const stop = this.stopCapture;
    this.stopCapture = null;
    stop?.();
  }

  private onKey(event: Event, down: boolean): void {
    if (down) this.setDevice("keyboard");
    // A keydown that does not count is dropped. A keyup always counts, so no key stays down.
    if (down && this.skipsPress(event)) return;
    this.setControl(controlKey({ type: "key", key: (event as KeyboardEvent).code }), down);
  }

  private onPointerButton(event: Event, down: boolean): void {
    const pointer = event as PointerEvent;
    this.moveTo(pointer.clientX, pointer.clientY);
    if (down) this.setDevice("mouse");
    if (down && this.skipsPress(event)) return;
    const name = BUTTON_NAMES[pointer.button];
    if (name !== undefined) this.setControl(controlKey({ type: "mouse", button: name }), down);
  }

  private onPointerMove(event: Event): void {
    const pointer = event as PointerEvent;
    this.moveTo(pointer.clientX, pointer.clientY);
  }

  private moveTo(x: number, y: number): void {
    if (this.hasPointer) {
      this.pointerDeltaX += x - this.pointerX;
      this.pointerDeltaY += y - this.pointerY;
    }
    this.pointerX = x;
    this.pointerY = y;
    this.hasPointer = true;
  }

  private onWheel(event: Event): void {
    this.setDevice("mouse");
    if (this.skipsPress(event)) return;
    const wheel = event as WheelEvent;
    const scale = pixelsPerUnit(wheel.deltaMode);
    const dx = wheel.deltaX * scale;
    const dy = wheel.deltaY * scale;
    this.wheelX += dx;
    this.wheelY += dy;
    if (dy < 0) this.pulse("up");
    if (dy > 0) this.pulse("down");
    if (dx < 0) this.pulse("left");
    if (dx > 0) this.pulse("right");
  }

  /** Starts a one-frame wheel pulse. It ends at the next endFrame. */
  private pulse(direction: WheelDirection): void {
    const id = controlKey({ type: "wheel", direction });
    if (this.downControls.has(id)) return;
    this.pulseControls.add(id);
    this.setControl(id, true);
  }

  /**
   * True when a press must not count: the UI has used the event, or it came from a control that
   * keeps its own keys. The rule is at the top of this file.
   */
  private skipsPress(event: Event): boolean {
    if (event.defaultPrevented || isHandled(event)) return true;
    return !this.captureControls && isControlTarget(event.target);
  }

  /** Focus left the scoping element for a place outside it: the keys that it counted are released. */
  private onFocusOut(event: Event, scope: Node): void {
    const next = (event as FocusEvent).relatedTarget;
    if (next instanceof Node && scope.contains(next)) return;
    this.releaseWhere(() => true);
  }

  /**
   * Sets a control down or up, and records the action edges that this causes. Repeating the
   * current state does nothing, so auto-repeated key events are ignored.
   */
  private setControl(id: string, down: boolean): void {
    if (this.downControls.has(id) === down) return;
    const before = this.snapshot(this.bindings.filter((binding) => binding.id === id).map((binding) => binding.action));
    if (down) this.downControls.add(id);
    else this.downControls.delete(id);
    this.recordEdges(before);
  }

  /** Releases every control whose id matches the predicate. */
  private releaseWhere(predicate: (id: string) => boolean): void {
    for (const id of [...this.downControls]) {
      if (!predicate(id)) continue;
      this.pulseControls.delete(id);
      this.setControl(id, false);
    }
  }

  /**
   * Reads the gamepads, and records the edges of the actions that a pad drives. Connections and
   * disconnections are reported, and so is a change to the gamepad as the last device used.
   */
  private poll(): void {
    const previous = this.pads;
    const next = readGamepads();
    const padActions = new Set<Action>();
    for (const binding of this.bindings) {
      if (binding.control.type.startsWith("pad-")) padActions.add(binding.action);
    }
    const before = this.snapshot(padActions);
    this.pads = next;
    this.recordEdges(before);
    this.reportPads(previous, next);
  }

  /** Reports the gamepads that came or went, and makes a gamepad the last device when it was used. */
  private reportPads(previous: readonly PadReading[], next: readonly PadReading[]): void {
    const before = new Set(previous.map((pad) => pad.index));
    const after = new Set(next.map((pad) => pad.index));
    for (const pad of next) {
      if (!before.has(pad.index)) this.events.emit("gamepad", { type: "connected", index: pad.index, id: pad.id });
    }
    for (const pad of previous) {
      if (!after.has(pad.index)) this.events.emit("gamepad", { type: "disconnected", index: pad.index, id: pad.id });
    }
    for (const pad of next) {
      // A pad that has just connected was at rest before, so a button it reports as pressed counts.
      const old = previous.find((candidate) => candidate.index === pad.index);
      const oldButtons = old?.buttons ?? [];
      const oldAxes = old?.axes ?? [];
      const pressed = pad.buttons.some(
        (button, index) => buttonLevel(button) >= DEVICE_PUSH_LEVEL && buttonLevel(oldButtons[index]) < DEVICE_PUSH_LEVEL,
      );
      const pushed = pad.axes.some(
        (value, index) => Math.abs(value) >= DEVICE_PUSH_LEVEL && Math.abs(oldAxes[index] ?? 0) < DEVICE_PUSH_LEVEL,
      );
      if (pressed || pushed) this.setDevice("gamepad");
    }
  }

  /** Makes a device the last one used, and tells the listeners when it changed. */
  private setDevice(device: InputDevice): void {
    if (device === this.device) return;
    this.device = device;
    this.events.emit("device", device);
  }

  /** Replaces the stored bindings, and records the edges of every action that they touch. */
  private replaceBindings(next: readonly Stored<Action>[]): void {
    const touched = new Set<Action>();
    for (const entry of this.bindings) touched.add(entry.action);
    for (const entry of next) touched.add(entry.action);
    const before = this.snapshot(touched);
    this.bindings = [...next];
    this.recordEdges(before);
  }

  /** Whether each of the actions is active now. Taken before a change, to find its edges. */
  private snapshot(actions: Iterable<Action>): Map<Action, boolean> {
    const states = new Map<Action, boolean>();
    for (const action of actions) states.set(action, this.isActive(action));
    return states;
  }

  /** Records the press and release of each action in `before` that changed since the snapshot. */
  private recordEdges(before: ReadonlyMap<Action, boolean>): void {
    for (const [action, wasActive] of before) {
      const isNowActive = this.isActive(action);
      if (!wasActive && isNowActive) this.pressedActions.add(action);
      if (wasActive && !isNowActive) this.releasedActions.add(action);
    }
  }

  /** The next slot of an action: one more than its highest slot, or 0 when it has none. */
  private nextSlot(action: Action): number {
    return this.bindings
      .filter((entry) => entry.action === action)
      .reduce((next, entry) => Math.max(next, entry.slot + 1), 0);
  }

  /**
   * The state of one binding: its analog value (the control's level times its scale), and whether
   * it is active. A key, mouse button or wheel is active while it is down, and gives 1 times its
   * scale. A pad control is active when its analog value reaches the threshold, and an axis that
   * feeds an axis component counts at every level.
   */
  private read(binding: Stored<Action>): { readonly analog: number; readonly active: boolean } {
    const { control } = binding;
    if (control.type === "key" || control.type === "mouse" || control.type === "wheel") {
      const down = this.downControls.has(binding.id);
      return { analog: down ? binding.scale : 0, active: down };
    }
    const analog = binding.scale * padValue(this.pads, control);
    return { analog, active: analog >= (control.threshold ?? DEFAULT_PAD_THRESHOLD) };
  }

  /** The value of an action from the controls that are down now. */
  private sample(action: Action): Sample {
    let digital = 0;
    let x = 0;
    let y = 0;
    let isAxis = false;
    for (const binding of this.bindings) {
      if (binding.action !== action) continue;
      const { analog, active } = this.read(binding);
      if (binding.axis !== null) {
        isAxis = true;
        // A stick that feeds an axis counts at every level; other controls count while active.
        if (active || binding.control.type === "pad-axis") {
          if (binding.axis === "x") x += analog;
          else y += analog;
        }
      } else if (active) {
        digital = Math.max(digital, analog);
      }
    }
    if (isAxis) {
      const vector = { x: clampUnit(x), y: clampUnit(y) };
      return { active: Math.hypot(vector.x, vector.y) >= this.threshold, vector };
    }
    const value = clampUnit(digital);
    return { active: value >= this.threshold, vector: { x: value, y: 0 } };
  }

  private isActive(action: Action): boolean {
    return this.sample(action).active;
  }
}

/** Throws a RangeError unless the slot is a whole number from 0 up. */
function assertSlot(slot: number): void {
  if (!Number.isInteger(slot) || slot < 0) {
    throw new RangeError(`slot must be a whole number from 0 up, got ${slot}`);
  }
}

/** The public form of a stored binding. */
function toBinding<Action extends string>(entry: Stored<Action>): Binding<Action> {
  return {
    action: entry.action,
    control: entry.control,
    scale: entry.scale,
    slot: entry.slot,
    context: entry.context,
    ...(entry.axis === null ? {} : { axisComponent: entry.axis }),
  };
}

/** True when the target is an element inside a control (the control itself counts). */
function isControlTarget(target: EventTarget | null): boolean {
  if (typeof Element === "undefined" || !(target instanceof Element)) return false;
  return target.closest(CONTROL_SELECTOR) !== null;
}

/** True for a DOM node. It is false where there is no DOM, as in a plain Node.js test. */
function isNode(target: EventTarget): target is Node {
  return typeof Node !== "undefined" && target instanceof Node;
}

/** True for the id of a mouse button control. */
function isMouseControl(id: string): boolean {
  return id.startsWith("mouse:");
}

/** The pixels that one unit of a wheel delta stands for, given its deltaMode. */
function pixelsPerUnit(deltaMode: number): number {
  if (deltaMode === DELTA_MODE_LINE) return PIXELS_PER_LINE;
  if (deltaMode === DELTA_MODE_PAGE) return PIXELS_PER_PAGE;
  return 1;
}

/** The window where a capture listens to keys and the mouse, or null where there is no window. */
function defaultTarget(): EventTarget | null {
  return typeof window === "undefined" ? null : window;
}
