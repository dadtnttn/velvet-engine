// A slider: a value between a minimum and a maximum, which moves in steps from the minimum. Left and Down
// lower it, Right and Up raise it, and Home and End go to the two ends. A press or a drag on the track sets it
// under the pointer. The element has role "slider", so a screen reader gets the name, the range, the value and
// the text that `format` gives.
//
// The fill is drawn with a transform and the thumb with a left offset, so a change of value moves no other
// layout. Hover and focus change colour only.

import { createElement, releaseFocus } from "./dom";

/** Options of {@link createSlider}. */
export interface SliderOptions {
  /** The name that a screen reader gives the slider, such as "Music volume". */
  readonly label?: string;
  /** The smallest value. */
  readonly min: number;
  /**
   * The largest value. The slider never goes above it. When the steps do not land on it, the largest value is
   * the last step that is not above it.
   */
  readonly max: number;
  /** The distance between two values. Default 1. */
  readonly step?: number;
  /** The starting value. Default `min`. It is snapped to the steps and kept in the range. */
  readonly value?: number;
  /** The text beside the track for a value. Default: the number as it is. */
  readonly format?: (value: number) => string;
  /** Runs with the new value when the player changes it, by a key or a pointer. A silent `setValue` does not run it. */
  readonly onChange?: (value: number) => void;
  /** True disables the slider: it leaves the Tab order, and the keys and the pointer do nothing. Default false. */
  readonly disabled?: boolean;
}

/** A slider. Create it with {@link createSlider}. */
export interface Slider {
  /** The element. Put it in the page, or in the stage's root. */
  readonly el: HTMLElement;
  /** The current value. */
  readonly value: number;
  /**
   * Sets the value. It is snapped to the steps and kept in the range. Unless `silent` is true, `onChange` runs
   * when the value changes.
   * @throws RangeError when the value is not a finite number.
   */
  setValue(value: number, options?: { readonly silent?: boolean }): void;
  /** Runs with the new value when the player changes it. It can be set after the slider is made. */
  onChange: ((value: number) => void) | undefined;
  /** Disables or enables the slider. A slider that is disabled gives up the focus. */
  setDisabled(disabled: boolean): void;
  /** Removes the element and its listeners. */
  destroy(): void;
}

/** The difference that still counts as the same step, to absorb the error of a division of decimals. */
const STEP_EPSILON = 1e-9;

/** The most decimal places that a value is rounded to. */
const MAX_DECIMALS = 20;

/**
 * Creates a slider.
 * @param options - the range, the step and the starting value, see {@link SliderOptions}.
 * @throws RangeError when the range, the step or the starting value is not valid.
 */
export function createSlider(options: SliderOptions): Slider {
  const min = options.min;
  const step = options.step ?? 1;
  checkRange(min, options.max, step);
  if (options.value !== undefined) requireFinite(options.value);
  const decimals = Math.min(MAX_DECIMALS, Math.max(decimalPlaces(min), decimalPlaces(step)));
  const count = Math.floor((options.max - min) / step + STEP_EPSILON);
  const last = roundTo(min + count * step, decimals);
  const format = options.format ?? ((value: number): string => String(value));

  const el = createElement("div", "vu-slider");
  el.setAttribute("role", "slider");
  if (options.label !== undefined && options.label !== "") el.setAttribute("aria-label", options.label);
  const track = createElement("div", "vu-slider-track");
  const fill = createElement("div", "vu-slider-fill");
  const thumb = createElement("div", "vu-slider-thumb");
  track.append(fill, thumb);
  const readout = createElement("span", "vu-slider-value");
  el.append(track, readout);

  /** The value on the steps that is nearest to `value`, kept between the first and the last step. */
  const snap = (value: number): number => {
    const clamped = Math.min(Math.max(value, min), last);
    const index = Math.round((clamped - min) / step);
    return Math.min(last, roundTo(min + index * step, decimals));
  };

  let current = snap(options.value ?? min);
  let disabled = options.disabled === true;
  /** The pointer that is dragging the thumb, or null when no drag is in progress. */
  let dragging: number | null = null;

  /** Draws the fill, the thumb, the text and the ARIA values from the current value. */
  const render = (): void => {
    const span = last - min;
    const share = span > 0 ? (current - min) / span : 0;
    fill.style.transform = `scaleX(${share})`;
    thumb.style.left = `${share * 100}%`;
    const text = format(current);
    readout.textContent = text;
    el.setAttribute("aria-valuemin", String(min));
    el.setAttribute("aria-valuemax", String(last));
    el.setAttribute("aria-valuenow", String(current));
    el.setAttribute("aria-valuetext", text);
    el.tabIndex = disabled ? -1 : 0;
    if (disabled) el.setAttribute("aria-disabled", "true");
    else el.removeAttribute("aria-disabled");
  };

  /** Sets the value from the pointer: the track's left edge is the minimum and its right edge the last step. */
  const setFromPointer = (clientX: number): void => {
    const box = track.getBoundingClientRect();
    if (!(box.width > 0)) return;
    const share = Math.min(Math.max((clientX - box.left) / box.width, 0), 1);
    setValue(min + share * (last - min));
  };

  const setValue = (value: number, setOptions: { readonly silent?: boolean } = {}): void => {
    requireFinite(value);
    const next = snap(value);
    if (next === current) return;
    current = next;
    render();
    if (setOptions.silent !== true) slider.onChange?.(current);
  };

  /** The arrows and Home and End. A key that another control has taken (prevented) is left alone. */
  const onKeyDown = (event: KeyboardEvent): void => {
    if (disabled || event.defaultPrevented) return;
    let wanted: number;
    switch (event.key) {
      case "ArrowRight":
      case "ArrowUp":
        wanted = current + step;
        break;
      case "ArrowLeft":
      case "ArrowDown":
        wanted = current - step;
        break;
      case "Home":
        wanted = min;
        break;
      case "End":
        wanted = last;
        break;
      default:
        return;
    }
    event.preventDefault();
    setValue(wanted);
  };

  const onPointerDown = (event: PointerEvent): void => {
    if (disabled || !event.isPrimary || event.button !== 0) return;
    event.preventDefault();
    el.focus({ preventScroll: true });
    dragging = event.pointerId;
    capturePointer(el, event.pointerId);
    setFromPointer(event.clientX);
  };

  const onPointerMove = (event: PointerEvent): void => {
    if (dragging === event.pointerId) setFromPointer(event.clientX);
  };

  const onPointerEnd = (event: PointerEvent): void => {
    if (dragging === event.pointerId) dragging = null;
  };

  el.addEventListener("keydown", onKeyDown);
  el.addEventListener("pointerdown", onPointerDown);
  el.addEventListener("pointermove", onPointerMove);
  el.addEventListener("pointerup", onPointerEnd);
  el.addEventListener("pointercancel", onPointerEnd);
  render();

  const slider: Slider = {
    el,
    get value(): number {
      return current;
    },
    setValue,
    onChange: options.onChange,
    setDisabled(next: boolean): void {
      disabled = next;
      if (disabled) {
        dragging = null;
        releaseFocus(el);
      }
      render();
    },
    destroy(): void {
      el.removeEventListener("keydown", onKeyDown);
      el.removeEventListener("pointerdown", onPointerDown);
      el.removeEventListener("pointermove", onPointerMove);
      el.removeEventListener("pointerup", onPointerEnd);
      el.removeEventListener("pointercancel", onPointerEnd);
      releaseFocus(el);
      el.remove();
    },
  };
  return slider;
}

/**
 * Checks the range and the step of a slider.
 * @throws RangeError when a value is not finite, the maximum is not above the minimum, or the step is not above
 *   zero or is larger than the range.
 */
function checkRange(min: number, max: number, step: number): void {
  requireFinite(min);
  requireFinite(max);
  requireFinite(step);
  if (!(max > min)) throw new RangeError(`max (${max}) must be above min (${min})`);
  if (!(step > 0)) throw new RangeError(`step must be above zero, got ${step}`);
  if (step > max - min) throw new RangeError(`step (${step}) is larger than the range from ${min} to ${max}`);
}

/** Throws a RangeError unless the value is a finite number. */
function requireFinite(value: number): void {
  if (!Number.isFinite(value)) throw new RangeError(`value must be a finite number, got ${value}`);
}

/** The number of decimal places in a number's shortest text, counting an exponent: 0.25 has two, 1e-7 has seven. */
function decimalPlaces(value: number): number {
  const [mantissa = "", exponent = "0"] = String(value).split("e");
  const fraction = mantissa.split(".")[1]?.length ?? 0;
  return Math.max(0, fraction - Number(exponent));
}

/** Rounds a number to a number of decimal places, so that a sum of steps shows its clean value. */
function roundTo(value: number, decimals: number): number {
  return Number(value.toFixed(decimals));
}

/**
 * Captures a pointer on an element, so that its moves and its release keep reaching it while the pointer is
 * outside it. A browser that refuses the capture is left alone: the element still gets the moves over it.
 */
function capturePointer(element: Element, pointerId: number): void {
  try {
    element.setPointerCapture(pointerId);
  } catch {
    // The pointer is not active, so there is nothing to capture.
  }
}
