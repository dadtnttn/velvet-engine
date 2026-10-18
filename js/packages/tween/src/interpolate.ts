// Interpolation between two values of one kind: numbers, arrays of numbers, objects of numbers,
// and colours (see colour.ts). `interpolate` returns a function from progress to value. A tween
// passes its eased progress to that function. The two ends come back exactly as given, so a tween
// finishes on its `to` value, not on a value that a rounding error puts a little away from it.
// Progress outside [0, 1] (the back and elastic easings go there) extrapolates.

import { formatColour, mixColours, parseColour } from "./colour";

/** The values a tween can animate: a number, a colour text, an array of numbers, or an object of numbers. */
export type TweenValue = number | string | readonly number[] | Readonly<Record<string, number>>;

/** Linear mix of two numbers, exact at both ends. */
function lerp(from: number, to: number, t: number): number {
  if (t === 0) return from;
  if (t === 1) return to;
  return from + (to - from) * t;
}

/** True for an array of numbers. */
function isNumberList(value: TweenValue): value is readonly number[] {
  return Array.isArray(value);
}

/** A short name for the kind of a value, for error messages. */
function kindOf(value: TweenValue): string {
  if (typeof value === "number") return "a number";
  if (typeof value === "string") return "a colour text";
  if (isNumberList(value)) return "an array";
  return "an object";
}

/**
 * Returns the function that gives the value at progress `t`, from `from` at 0 to `to` at 1. Both
 * values must be the same kind: two numbers, two colour texts, two arrays of the same length, or
 * two objects with the same keys (the values of arrays and objects are numbers).
 * @throws TypeError when the two values are of different kinds.
 * @throws RangeError when a number is not finite, a colour is not in a supported form, or the
 * arrays or the objects differ in size or keys.
 */
export function interpolate(from: TweenValue, to: TweenValue): (t: number) => TweenValue {
  if (typeof from === "number") {
    if (typeof to !== "number") throw kindMismatch(from, to);
    if (!Number.isFinite(from) || !Number.isFinite(to)) {
      throw new RangeError(`numbers must be finite, got ${from} and ${to}`);
    }
    return (t) => lerp(from, to, t);
  }

  if (typeof from === "string") {
    if (typeof to !== "string") throw kindMismatch(from, to);
    const start = parseColour(from);
    const end = parseColour(to);
    if (start === null || end === null) {
      throw new RangeError(
        `colours must be #rgb, #rgba, #rrggbb, #rrggbbaa, rgb() or rgba(), got "${from}" and "${to}"`,
      );
    }
    const withAlpha = start.hasAlpha || end.hasAlpha;
    return (t) => (t === 0 ? from : t === 1 ? to : formatColour(mixColours(start, end, t), withAlpha));
  }

  if (isNumberList(from)) {
    if (!isNumberList(to)) throw kindMismatch(from, to);
    if (from.length !== to.length) {
      throw new RangeError(`arrays must have the same length, got ${from.length} and ${to.length}`);
    }
    if (!from.every(Number.isFinite) || !to.every(Number.isFinite)) {
      throw new RangeError("array values must be finite numbers");
    }
    // Copies at creation, so that later changes to the caller's arrays do not change the tween.
    const start = [...from];
    const end = [...to];
    return (t) => (t === 0 ? [...start] : t === 1 ? [...end] : start.map((value, index) => lerp(value, end[index] ?? value, t)));
  }

  if (typeof to !== "object" || isNumberList(to)) throw kindMismatch(from, to);
  const keys = Object.keys(from);
  const endKeys = Object.keys(to);
  if (keys.length !== endKeys.length || keys.some((key) => !Object.hasOwn(to, key))) {
    throw new RangeError("objects must have the same keys");
  }
  for (const key of keys) {
    const start = from[key];
    const end = to[key];
    if (start === undefined || end === undefined || !Number.isFinite(start) || !Number.isFinite(end)) {
      throw new RangeError(`object value "${key}" must be a finite number`);
    }
  }
  const start: Record<string, number> = { ...from };
  const end: Record<string, number> = { ...to };
  return (t) => {
    const out: Record<string, number> = {};
    for (const key of keys) out[key] = lerp(start[key] ?? 0, end[key] ?? 0, t);
    return out;
  };
}

/** The error for values of two different kinds. */
function kindMismatch(from: TweenValue, to: TweenValue): TypeError {
  return new TypeError(`cannot tween ${kindOf(from)} to ${kindOf(to)}: both ends must be the same kind`);
}
