// Action input for games. Keys, mouse buttons and the mouse wheel are bound to named
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
// edges and the wheel and pointer deltas.
//
// A wheel tick is a one-frame pulse. Its action is pressed and held in the frame the tick
// arrives, and released in the next frame.
//
// Keys are KeyboardEvent.code values, which name physical keys whatever the layout. Pointer
// events cover mouse, pen and touch; a touch counts as the left button.

/** A KeyboardEvent.code value: a physical key such as "KeyW", "Space" or "ArrowUp". */
export type KeyCode = string;

/** Mouse buttons by name. The DOM button numbers 0, 1 and 2 are left, middle and right. */
export type MouseButtonName = "left" | "middle" | "right";

/** Wheel directions: "up" and "down" for the vertical wheel, "left" and "right" for horizontal scroll. */
export type WheelDirection = "up" | "down" | "left" | "right";

/** A physical control that a binding listens to. */
export type Control =
  | { readonly type: "key"; readonly key: KeyCode }
  | { readonly type: "mouse"; readonly button: MouseButtonName }
  | { readonly type: "wheel"; readonly direction: WheelDirection };

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
}

const DEFAULT_THRESHOLD = 0.5;

/** Button names for PointerEvent.button values. */
const BUTTON_NAMES: Readonly<Record<number, MouseButtonName>> = { 0: "left", 1: "middle", 2: "right" };

/** WheelEvent.deltaMode values for lines and pages. Pixels are 0. */
const DELTA_MODE_LINE = 1;
const DELTA_MODE_PAGE = 2;

/** Approximate pixels per wheel line and per wheel page, used when a browser reports lines or pages. */
const PIXELS_PER_LINE = 16;
const PIXELS_PER_PAGE = 100;

/** A binding in the form the map keeps it. */
interface StoredBinding<Action extends string> {
  readonly action: Action;
  readonly control: string;
  readonly scale: number;
  readonly axis: "x" | "y" | null;
}

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
  private bindings: StoredBinding<Action>[] = [];
  private readonly downControls = new Set<string>();
  private readonly pulseControls = new Set<string>();
  private readonly pressedActions = new Set<Action>();
  private readonly releasedActions = new Set<Action>();
  private wheelX = 0;
  private wheelY = 0;
  private pointerX = 0;
  private pointerY = 0;
  private pointerDeltaX = 0;
  private pointerDeltaY = 0;
  private hasPointer = false;
  private attachment: Attachment | null = null;

  /**
   * @param bindings - the bindings to start with.
   * @param options - see {@link ActionMapOptions}.
   * @throws RangeError when the threshold is not a positive finite number, or a binding has a
   * scale that is not finite.
   */
  constructor(bindings: readonly Binding<Action>[] = [], options: ActionMapOptions = {}) {
    const threshold = options.threshold ?? DEFAULT_THRESHOLD;
    if (!Number.isFinite(threshold) || threshold <= 0) {
      throw new RangeError(`threshold must be a positive number, got ${threshold}`);
    }
    this.threshold = threshold;
    for (const binding of bindings) this.bind(binding);
  }

  /**
   * Adds a binding. When its control is already down, the action becomes active at once and
   * counts as pressed.
   * @throws RangeError when the scale is not a finite number.
   */
  bind(binding: Binding<Action>): void {
    const scale = binding.scale ?? 1;
    if (!Number.isFinite(scale)) {
      throw new RangeError(`binding scale must be a finite number, got ${scale}`);
    }
    const wasActive = this.isActive(binding.action);
    this.bindings.push({
      action: binding.action,
      control: controlId(binding.control),
      scale,
      axis: binding.axisComponent ?? null,
    });
    if (!wasActive && this.isActive(binding.action)) this.pressedActions.add(binding.action);
  }

  /** Removes every binding of `action`. An action that was active becomes released. */
  unbind(action: Action): void {
    const wasActive = this.isActive(action);
    this.bindings = this.bindings.filter((binding) => binding.action !== action);
    if (wasActive && !this.isActive(action)) this.releasedActions.add(action);
  }

  /**
   * Listens for keyboard, pointer and wheel events on `target`, usually `window`. Attaching
   * again moves the listeners: the previous target is detached first.
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
    return this.pressedActions.has(action);
  }

  /** True while the action is active. */
  held(action: Action): boolean {
    return this.isActive(action);
  }

  /** True on the frame in which the action stopped being active (since the last endFrame). */
  released(action: Action): boolean {
    return this.releasedActions.has(action);
  }

  /** The value of a digital action in [-1, 1], or the x component of a 2D action. */
  axis1(action: Action): number {
    return this.sample(action).vector.x;
  }

  /** The value of a 2D action. For a digital action this is `{ x: value, y: 0 }`. */
  axis2(action: Action): Vector2 {
    const { x, y } = this.sample(action).vector;
    return { x, y };
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
   * Clears the edges and the wheel and pointer deltas. Call it once per frame, after the
   * frame has read the input.
   */
  endFrame(): void {
    this.pressedActions.clear();
    this.releasedActions.clear();
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
  }

  private onKey(event: Event, down: boolean): void {
    this.setControl(controlId({ type: "key", key: (event as KeyboardEvent).code }), down);
  }

  private onPointerButton(event: Event, down: boolean): void {
    const pointer = event as PointerEvent;
    this.moveTo(pointer.clientX, pointer.clientY);
    const name = BUTTON_NAMES[pointer.button];
    if (name !== undefined) this.setControl(controlId({ type: "mouse", button: name }), down);
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
    const id = controlId({ type: "wheel", direction });
    if (this.downControls.has(id)) return;
    this.pulseControls.add(id);
    this.setControl(id, true);
  }

  /**
   * Sets a control down or up, and records the action edges that this causes. Repeating the
   * current state does nothing, so auto-repeated key events are ignored.
   */
  private setControl(id: string, down: boolean): void {
    if (this.downControls.has(id) === down) return;
    const actions = [
      ...new Set(this.bindings.filter((binding) => binding.control === id).map((binding) => binding.action)),
    ];
    const before = actions.map((action) => ({ action, wasActive: this.isActive(action) }));
    if (down) this.downControls.add(id);
    else this.downControls.delete(id);
    for (const { action, wasActive } of before) {
      const isNowActive = this.isActive(action);
      if (!wasActive && isNowActive) this.pressedActions.add(action);
      if (wasActive && !isNowActive) this.releasedActions.add(action);
    }
  }

  /** Releases every control whose id matches the predicate. */
  private releaseWhere(predicate: (id: string) => boolean): void {
    for (const id of [...this.downControls]) {
      if (!predicate(id)) continue;
      this.pulseControls.delete(id);
      this.setControl(id, false);
    }
  }

  /** The value of an action from the controls that are down now. */
  private sample(action: Action): Sample {
    let digital = 0;
    let x = 0;
    let y = 0;
    let isAxis = false;
    for (const binding of this.bindings) {
      if (binding.action !== action || !this.downControls.has(binding.control)) continue;
      if (binding.axis === "x") {
        x += binding.scale;
        isAxis = true;
      } else if (binding.axis === "y") {
        y += binding.scale;
        isAxis = true;
      } else {
        digital = Math.max(digital, binding.scale);
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

/** The id of a control, used to track which controls are down. */
function controlId(control: Control): string {
  switch (control.type) {
    case "key":
      return `key:${control.key}`;
    case "mouse":
      return `mouse:${control.button}`;
    case "wheel":
      return `wheel:${control.direction}`;
  }
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

/** Clamps a value into [-1, 1]. */
function clampUnit(value: number): number {
  return Math.min(1, Math.max(-1, value));
}
