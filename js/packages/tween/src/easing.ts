// Easing functions: maps from progress in [0, 1] to eased progress.
//
// Every easing takes `t` in [0, 1] and returns the eased value. The endpoints are exact:
// `ease(0) === 0` and `ease(1) === 1` for every easing here. The back, elastic and bounce
// families overshoot or wobble in the middle, so their results may leave [0, 1]; the others
// stay inside it. Use `steps` and `cubicBezier` for CSS-style timing curves.

/** An easing: progress in, eased progress out. */
export type EasingFunction = (t: number) => number;

/** Names of the built-in easings, for data files. `ease` looks them up. */
export type EasingName =
  | "linear"
  | "quadIn"
  | "quadOut"
  | "quadInOut"
  | "cubicIn"
  | "cubicOut"
  | "cubicInOut"
  | "quartIn"
  | "quartOut"
  | "quartInOut"
  | "quintIn"
  | "quintOut"
  | "quintInOut"
  | "sineIn"
  | "sineOut"
  | "sineInOut"
  | "expoIn"
  | "expoOut"
  | "expoInOut"
  | "circIn"
  | "circOut"
  | "circInOut"
  | "backIn"
  | "backOut"
  | "backInOut"
  | "elasticIn"
  | "elasticOut"
  | "elasticInOut"
  | "bounceIn"
  | "bounceOut"
  | "bounceInOut";

/** Overshoot amount of the back easings: the standard 1.70158 used by most motion systems. */
const BACK_OVERSHOOT = 1.70158;

/** Period and amplitude constants of the elastic easings. */
const ELASTIC_PERIOD = (2 * Math.PI) / 3;
const ELASTIC_PERIOD_HALF = (2 * Math.PI) / 4.5;

/** Constants of the bounce easing: the four parabolas of a bouncing ball. */
const BOUNCE_N = 7.5625;
const BOUNCE_D = 2.75;

/** Linear progress: no easing. */
export function linear(t: number): number {
  return t;
}

/** Power of `t`: the shared body of the quad, cubic, quart and quint families. */
function powerIn(power: number): EasingFunction {
  return (t) => t ** power;
}

/** Mirror of a power-in curve, so that it starts fast and stops slowly. */
function powerOut(power: number): EasingFunction {
  return (t) => 1 - (1 - t) ** power;
}

/** Power-in on the first half and power-out on the second half, so it starts and stops slowly. */
function powerInOut(power: number): EasingFunction {
  return (t) => (t < 0.5 ? 2 ** (power - 1) * t ** power : 1 - (-2 * t + 2) ** power / 2);
}

/** Quadratic ease in: t squared. */
export const quadIn: EasingFunction = powerIn(2);
/** Quadratic ease out. */
export const quadOut: EasingFunction = powerOut(2);
/** Quadratic ease in and out. */
export const quadInOut: EasingFunction = powerInOut(2);

/** Cubic ease in: t cubed. */
export const cubicIn: EasingFunction = powerIn(3);
/** Cubic ease out. */
export const cubicOut: EasingFunction = powerOut(3);
/** Cubic ease in and out. */
export const cubicInOut: EasingFunction = powerInOut(3);

/** Quartic ease in: t to the fourth. */
export const quartIn: EasingFunction = powerIn(4);
/** Quartic ease out. */
export const quartOut: EasingFunction = powerOut(4);
/** Quartic ease in and out. */
export const quartInOut: EasingFunction = powerInOut(4);

/** Quintic ease in: t to the fifth. */
export const quintIn: EasingFunction = powerIn(5);
/** Quintic ease out. */
export const quintOut: EasingFunction = powerOut(5);
/** Quintic ease in and out. */
export const quintInOut: EasingFunction = powerInOut(5);

/** Sine ease in: a quarter of a cosine wave. */
export function sineIn(t: number): number {
  return 1 - Math.cos((t * Math.PI) / 2);
}

/** Sine ease out. */
export function sineOut(t: number): number {
  return Math.sin((t * Math.PI) / 2);
}

/** Sine ease in and out: half a cosine wave. */
export function sineInOut(t: number): number {
  return -(Math.cos(Math.PI * t) - 1) / 2;
}

/** Exponential ease in. Exactly 0 at t = 0, so the endpoints stay exact. */
export function expoIn(t: number): number {
  return t === 0 ? 0 : 2 ** (10 * t - 10);
}

/** Exponential ease out. Exactly 1 at t = 1. */
export function expoOut(t: number): number {
  return t === 1 ? 1 : 1 - 2 ** (-10 * t);
}

/** Exponential ease in and out. */
export function expoInOut(t: number): number {
  if (t === 0) return 0;
  if (t === 1) return 1;
  return t < 0.5 ? 2 ** (20 * t - 10) / 2 : (2 - 2 ** (-20 * t + 10)) / 2;
}

/** Circular ease in: a quarter of a circle. */
export function circIn(t: number): number {
  return 1 - Math.sqrt(1 - t * t);
}

/** Circular ease out. */
export function circOut(t: number): number {
  return Math.sqrt(1 - (t - 1) ** 2);
}

/** Circular ease in and out. */
export function circInOut(t: number): number {
  return t < 0.5
    ? (1 - Math.sqrt(1 - (2 * t) ** 2)) / 2
    : (Math.sqrt(1 - (-2 * t + 2) ** 2) + 1) / 2;
}

/** Back ease in: pulls back below zero before it moves forward. */
export function backIn(t: number): number {
  const c = BACK_OVERSHOOT + 1;
  return c * t * t * t - BACK_OVERSHOOT * t * t;
}

/** Back ease out: runs past one and settles back. */
export function backOut(t: number): number {
  const c = BACK_OVERSHOOT + 1;
  return 1 + c * (t - 1) ** 3 + BACK_OVERSHOOT * (t - 1) ** 2;
}

/** Back ease in and out: pulls back at the start and overshoots at the end. */
export function backInOut(t: number): number {
  const c = BACK_OVERSHOOT * 1.525;
  return t < 0.5
    ? ((2 * t) ** 2 * ((c + 1) * 2 * t - c)) / 2
    : ((2 * t - 2) ** 2 * ((c + 1) * (t * 2 - 2) + c) + 2) / 2;
}

/** Elastic ease in: a spring that winds up and snaps to the end. */
export function elasticIn(t: number): number {
  if (t === 0) return 0;
  if (t === 1) return 1;
  return -(2 ** (10 * t - 10)) * Math.sin((t * 10 - 10.75) * ELASTIC_PERIOD);
}

/** Elastic ease out: overshoots and wobbles around one before it settles. */
export function elasticOut(t: number): number {
  if (t === 0) return 0;
  if (t === 1) return 1;
  return 2 ** (-10 * t) * Math.sin((t * 10 - 0.75) * ELASTIC_PERIOD) + 1;
}

/** Elastic ease in and out. */
export function elasticInOut(t: number): number {
  if (t === 0) return 0;
  if (t === 1) return 1;
  return t < 0.5
    ? -(2 ** (20 * t - 10) * Math.sin((20 * t - 11.125) * ELASTIC_PERIOD_HALF)) / 2
    : (2 ** (-20 * t + 10) * Math.sin((20 * t - 11.125) * ELASTIC_PERIOD_HALF)) / 2 + 1;
}

/** Bounce out: a ball that drops and bounces with shrinking heights, then lands on one. */
export function bounceOut(t: number): number {
  if (t < 1 / BOUNCE_D) {
    return BOUNCE_N * t * t;
  }
  if (t < 2 / BOUNCE_D) {
    const u = t - 1.5 / BOUNCE_D;
    return BOUNCE_N * u * u + 0.75;
  }
  if (t < 2.5 / BOUNCE_D) {
    const u = t - 2.25 / BOUNCE_D;
    return BOUNCE_N * u * u + 0.9375;
  }
  const u = t - 2.625 / BOUNCE_D;
  return BOUNCE_N * u * u + 0.984375;
}

/** Bounce in: the bounce played backwards. */
export function bounceIn(t: number): number {
  return 1 - bounceOut(1 - t);
}

/** Bounce in and out. */
export function bounceInOut(t: number): number {
  return t < 0.5 ? (1 - bounceOut(1 - 2 * t)) / 2 : (1 + bounceOut(2 * t - 1)) / 2;
}

/** Every built-in easing by name. `ease` reads from this table. */
const EASINGS: Readonly<Record<EasingName, EasingFunction>> = {
  linear,
  quadIn,
  quadOut,
  quadInOut,
  cubicIn,
  cubicOut,
  cubicInOut,
  quartIn,
  quartOut,
  quartInOut,
  quintIn,
  quintOut,
  quintInOut,
  sineIn,
  sineOut,
  sineInOut,
  expoIn,
  expoOut,
  expoInOut,
  circIn,
  circOut,
  circInOut,
  backIn,
  backOut,
  backInOut,
  elasticIn,
  elasticOut,
  elasticInOut,
  bounceIn,
  bounceOut,
  bounceInOut,
};

/** True when `name` is one of the built-in easing names. */
export function isEasingName(name: string): name is EasingName {
  return Object.prototype.hasOwnProperty.call(EASINGS, name);
}

/**
 * Looks an easing up by its name, for data files.
 * @throws RangeError when the name is not a built-in easing.
 */
export function ease(name: string): EasingFunction {
  if (!isEasingName(name)) {
    throw new RangeError(`unknown easing "${name}"`);
  }
  return EASINGS[name];
}

/**
 * A cubic Bezier curve with CSS semantics, the same curve as `cubic-bezier(x1, y1, x2, y2)`.
 * The x coordinates must lie in [0, 1], so that time only moves forward. The curve runs from
 * (0, 0) to (1, 1); the result for a time `t` is found by solving for the curve parameter
 * whose x equals `t` (Newton's method, with bisection when the slope is flat), then reading y.
 *
 * @param x1 - x of the first control point, in [0, 1].
 * @param y1 - y of the first control point, any number (CSS allows overshoot).
 * @param x2 - x of the second control point, in [0, 1].
 * @param y2 - y of the second control point, any number.
 * @throws RangeError when an x coordinate is outside [0, 1] or a value is not finite.
 */
export function cubicBezier(x1: number, y1: number, x2: number, y2: number): EasingFunction {
  for (const [name, value] of [
    ["x1", x1],
    ["y1", y1],
    ["x2", x2],
    ["y2", y2],
  ] as const) {
    if (!Number.isFinite(value)) {
      throw new RangeError(`cubicBezier ${name} must be a finite number, got ${value}`);
    }
  }
  if (x1 < 0 || x1 > 1 || x2 < 0 || x2 > 1) {
    throw new RangeError("cubicBezier x1 and x2 must lie in [0, 1]");
  }
  // The curve's coordinate as a function of the parameter s, with endpoints fixed at 0 and 1.
  const sampleX = (s: number): number => bezierCoordinate(x1, x2, s);
  const sampleY = (s: number): number => bezierCoordinate(y1, y2, s);
  // Slope of x with respect to the parameter, used by Newton's method.
  const slopeX = (s: number): number => bezierDerivative(x1, x2, s);

  return (t: number): number => {
    if (t <= 0) return 0;
    if (t >= 1) return 1;
    const s = solveParameter(t, sampleX, slopeX);
    return sampleY(s);
  };
}

/**
 * One coordinate of a cubic Bezier with endpoints 0 and 1 and control values `p1` and `p2`,
 * at the parameter `s` in [0, 1]. Expanded from the Bernstein form, as CSS does it.
 */
function bezierCoordinate(p1: number, p2: number, s: number): number {
  // The polynomial a*s^3 + b*s^2 + c*s, with the constant term zero because the curve starts at 0.
  const c = 3 * p1;
  const b = 3 * (p2 - p1) - c;
  const a = 1 - c - b;
  return ((a * s + b) * s + c) * s;
}

/** The derivative of {@link bezierCoordinate} with respect to `s`. */
function bezierDerivative(p1: number, p2: number, s: number): number {
  const c = 3 * p1;
  const b = 3 * (p2 - p1) - c;
  const a = 1 - c - b;
  return (3 * a * s + 2 * b) * s + c;
}

/** Newton-Raphson iterations before the solver falls back to bisection. */
const NEWTON_STEPS = 8;
/** Slope below which Newton's method is not trusted and bisection takes over. */
const MIN_SLOPE = 1e-3;
/** Accuracy of the parameter solve; well below what a rendered frame can show. */
const SOLVE_EPSILON = 1e-7;

/**
 * Finds the parameter `s` in [0, 1] where the curve's x equals `t`. Newton's method converges in
 * a few steps where the slope is steady. Where it is flat, or it does not converge, bisection
 * is used instead, which always finds the answer because x increases with s on a valid curve.
 */
function solveParameter(
  t: number,
  sampleX: (s: number) => number,
  slopeX: (s: number) => number,
): number {
  let s = t;
  for (let i = 0; i < NEWTON_STEPS; i += 1) {
    const error = sampleX(s) - t;
    if (Math.abs(error) < SOLVE_EPSILON) return s;
    const slope = slopeX(s);
    if (Math.abs(slope) < MIN_SLOPE) break;
    s -= error / slope;
  }
  if (s >= 0 && s <= 1 && Math.abs(sampleX(s) - t) < SOLVE_EPSILON) return s;

  let low = 0;
  let high = 1;
  s = t;
  while (high - low > SOLVE_EPSILON) {
    const x = sampleX(s);
    if (x < t) low = s;
    else high = s;
    s = (low + high) / 2;
  }
  return s;
}

/**
 * A stepped easing with CSS semantics: `steps(n, position)`. The output takes `n` equal levels
 * instead of rising smoothly. `position` decides where each step lands:
 * - "jump-start": the first level is reached at t = 0, so there are n levels above 0.
 * - "jump-end": the last level is held until t = 1, the default in CSS.
 * - "jump-none": the levels are held at both ends, so there are n - 1 steps and n levels.
 * - "jump-both": there are n + 1 steps, with a hold at each end.
 * - "start": the same as "jump-start", named as in CSS.
 * - "end": the same as "jump-end", named as in CSS.
 *
 * @throws RangeError when `n` is not a whole number from 1, or `jump-none` is used with `n` 1.
 */
export function steps(count: number, position: StepPosition = "jump-end"): EasingFunction {
  if (!Number.isInteger(count) || count < 1) {
    throw new RangeError(`steps count must be a whole number from 1, got ${count}`);
  }
  if (position === "jump-none" && count < 2) {
    throw new RangeError("steps with jump-none needs at least 2 steps");
  }
  return (t: number): number => {
    // The CSS spec: at progress 1 every position ends on its final level.
    if (t >= 1) return 1;
    const progress = Math.max(0, t);
    // The level index of the step that progress falls in, counted from 0 (CSS spec, section 3.2).
    const index = Math.floor(progress * count);
    switch (position) {
      case "jump-start":
      case "start":
        return Math.min(count, index + 1) / count;
      case "jump-end":
      case "end":
        return index / count;
      case "jump-none":
        return index / (count - 1);
      case "jump-both":
        return (index + 1) / (count + 1);
    }
  };
}

/** Where a stepped easing puts its steps. See {@link steps}. */
export type StepPosition = "jump-start" | "jump-end" | "jump-none" | "jump-both" | "start" | "end";
