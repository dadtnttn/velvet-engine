// Tests for the easing functions: the endpoints, the shapes (monotonic, bounded, or overshooting),
// the symmetry of the in-out curves, cubic-bezier against the CSS keywords, the stepped easings, and
// the lookup by name.

import { describe, expect, it } from "vitest";
import {
  backIn,
  backInOut,
  backOut,
  bounceInOut,
  bounceOut,
  circIn,
  circOut,
  cubicBezier,
  cubicInOut,
  cubicOut,
  ease,
  elasticIn,
  elasticOut,
  expoIn,
  expoOut,
  isEasingName,
  linear,
  quadIn,
  quadOut,
  quartOut,
  quintIn,
  sineInOut,
  steps,
  type EasingName,
} from "./easing";

/** Every built-in easing name. */
const NAMES: readonly EasingName[] = [
  "linear",
  "quadIn", "quadOut", "quadInOut",
  "cubicIn", "cubicOut", "cubicInOut",
  "quartIn", "quartOut", "quartInOut",
  "quintIn", "quintOut", "quintInOut",
  "sineIn", "sineOut", "sineInOut",
  "expoIn", "expoOut", "expoInOut",
  "circIn", "circOut", "circInOut",
  "backIn", "backOut", "backInOut",
  "elasticIn", "elasticOut", "elasticInOut",
  "bounceIn", "bounceOut", "bounceInOut",
];

/** The easings whose values stay between 0 and 1. */
const BOUNDED: readonly EasingName[] = NAMES.filter(
  (name) => !name.startsWith("back") && !name.startsWith("elastic"),
);

/** The easings that never go down as time goes up. Bounce goes down between its bounces, so it is left out. */
const MONOTONIC: readonly EasingName[] = NAMES.filter(
  (name) => !name.startsWith("back") && !name.startsWith("elastic") && !name.startsWith("bounce"),
);

/** Evaluates an easing at `count + 1` evenly spaced points from 0 to 1. */
function samples(fn: (t: number) => number, count = 400): number[] {
  return Array.from({ length: count + 1 }, (_, index) => fn(index / count));
}

/**
 * Reference cubic-bezier: finds the curve parameter whose x is `t` by plain bisection, with no
 * Newton steps, then reads y. Slow, but simple enough to trust, so the real solver is checked
 * against it.
 */
function referenceBezier(x1: number, y1: number, x2: number, y2: number, t: number): number {
  const at = (s: number, p1: number, p2: number): number =>
    3 * (1 - s) ** 2 * s * p1 + 3 * (1 - s) * s ** 2 * p2 + s ** 3;
  let low = 0;
  let high = 1;
  for (let index = 0; index < 200; index += 1) {
    const middle = (low + high) / 2;
    if (at(middle, x1, x2) < t) low = middle;
    else high = middle;
  }
  return at((low + high) / 2, y1, y2);
}

describe("easing endpoints", () => {
  it.each(NAMES)("%s starts at 0 and ends at 1", (name) => {
    const fn = ease(name);
    expect(fn(0)).toBeCloseTo(0, 12);
    expect(fn(1)).toBeCloseTo(1, 12);
  });
});

describe("easing shapes", () => {
  it.each(MONOTONIC)("%s never goes down", (name) => {
    const values = samples(ease(name));
    for (let index = 1; index < values.length; index += 1) {
      expect(values[index]).toBeGreaterThanOrEqual((values[index - 1] ?? 0) - 1e-12);
    }
  });

  it.each(BOUNDED)("%s stays between 0 and 1", (name) => {
    for (const value of samples(ease(name))) {
      expect(value).toBeGreaterThanOrEqual(-1e-9);
      expect(value).toBeLessThanOrEqual(1 + 1e-9);
    }
  });

  it("back out overshoots past 1, and back in dips below 0", () => {
    expect(Math.max(...samples(backOut))).toBeGreaterThan(1.05);
    expect(Math.min(...samples(backIn))).toBeLessThan(-0.05);
  });

  it("back in and out does both", () => {
    expect(Math.min(...samples(backInOut))).toBeLessThan(0);
    expect(Math.max(...samples(backInOut))).toBeGreaterThan(1);
  });

  it("elastic out wobbles past 1, and elastic in dips below 0", () => {
    expect(Math.max(...samples(elasticOut))).toBeGreaterThan(1);
    expect(Math.min(...samples(elasticIn))).toBeLessThan(0);
  });

  it.each([
    ["quadInOut", ease("quadInOut")],
    ["cubicInOut", cubicInOut],
    ["sineInOut", sineInOut],
    ["circInOut", ease("circInOut")],
    ["bounceInOut", bounceInOut],
    ["backInOut", backInOut],
    ["expoInOut", ease("expoInOut")],
  ] as const)("%s is point symmetric: f(t) + f(1 - t) = 1", (_name, fn) => {
    for (const t of [0.05, 0.2, 0.37, 0.5, 0.71, 0.93]) {
      expect(fn(t) + fn(1 - t)).toBeCloseTo(1, 9);
    }
  });
});

describe("known values", () => {
  it("matches the textbook values at one half", () => {
    expect(quadIn(0.5)).toBeCloseTo(0.25, 12);
    expect(quadOut(0.5)).toBeCloseTo(0.75, 12);
    expect(cubicOut(0.5)).toBeCloseTo(0.875, 12);
    expect(quartOut(0.5)).toBeCloseTo(0.9375, 12);
    expect(quintIn(0.5)).toBeCloseTo(1 / 32, 12);
    expect(sineInOut(0.5)).toBeCloseTo(0.5, 12);
    expect(expoIn(0.5)).toBeCloseTo(1 / 32, 12);
    expect(expoOut(0.5)).toBeCloseTo(31 / 32, 12);
    expect(circIn(0.5)).toBeCloseTo(1 - Math.sqrt(0.75), 12);
    expect(circOut(0.5)).toBeCloseTo(Math.sqrt(0.75), 12);
    expect(backOut(0.5)).toBeCloseTo(1.0877, 4);
    expect(bounceOut(0.5)).toBeCloseTo(0.765625, 12);
    expect(bounceOut(1)).toBeCloseTo(1, 12);
  });

  it("linear is the identity", () => {
    expect(linear(0.3)).toBe(0.3);
  });
});

describe("cubicBezier", () => {
  it("is linear for the curve from (0, 0) to (1, 1)", () => {
    const fn = cubicBezier(0, 0, 1, 1);
    for (const t of [0.1, 0.25, 0.5, 0.8]) expect(fn(t)).toBeCloseTo(t, 6);
  });

  // The CSS keywords, checked against the reference solver at many points.
  const KEYWORDS: Array<[string, [number, number, number, number]]> = [
    ["ease", [0.25, 0.1, 0.25, 1]],
    ["ease-in", [0.42, 0, 1, 1]],
    ["ease-out", [0, 0, 0.58, 1]],
    ["ease-in-out", [0.42, 0, 0.58, 1]],
  ];
  it.each(KEYWORDS)("%s agrees with a bisection reference", (_name, [x1, y1, x2, y2]) => {
    const fn = cubicBezier(x1, y1, x2, y2);
    for (let index = 1; index < 50; index += 1) {
      const t = index / 50;
      expect(fn(t)).toBeCloseTo(referenceBezier(x1, y1, x2, y2, t), 6);
    }
  });

  it("gives the known CSS ease value at one half", () => {
    expect(cubicBezier(0.25, 0.1, 0.25, 1)(0.5)).toBeCloseTo(0.8024, 3);
  });

  it("is point symmetric for ease-in-out", () => {
    const fn = cubicBezier(0.42, 0, 0.58, 1);
    expect(fn(0.5)).toBeCloseTo(0.5, 9);
    for (const t of [0.1, 0.3, 0.45]) expect(fn(t) + fn(1 - t)).toBeCloseTo(1, 7);
  });

  it("keeps the endpoints exact, and lets y leave [0, 1] when the curve says so", () => {
    const fn = cubicBezier(0.3, -0.5, 0.7, 1.5);
    expect(fn(0)).toBe(0);
    expect(fn(1)).toBe(1);
    expect(Math.min(...samples(fn))).toBeLessThan(0);
    expect(Math.max(...samples(fn))).toBeGreaterThan(1);
  });

  it("refuses an x coordinate outside [0, 1], and a value that is not finite", () => {
    expect(() => cubicBezier(-0.1, 0, 0.5, 1)).toThrow(RangeError);
    expect(() => cubicBezier(0.2, 0, 1.2, 1)).toThrow(RangeError);
    expect(() => cubicBezier(0.2, Number.NaN, 0.5, 1)).toThrow(RangeError);
  });
});

describe("steps", () => {
  it("jump-end holds each level until its step ends, and reaches 1 at the end", () => {
    const fn = steps(4, "jump-end");
    expect(fn(0)).toBe(0);
    expect(fn(0.24)).toBe(0);
    expect(fn(0.25)).toBe(0.25);
    expect(fn(0.99)).toBe(0.75);
    expect(fn(1)).toBe(1);
  });

  it("jump-start starts at the first level, and holds 1 from the last step", () => {
    const fn = steps(4, "jump-start");
    expect(fn(0)).toBe(0.25);
    expect(fn(0.24)).toBe(0.25);
    expect(fn(0.25)).toBe(0.5);
    expect(fn(0.99)).toBe(1);
    expect(fn(1)).toBe(1);
  });

  it("jump-none has n levels from 0 to 1, with no jump at either end", () => {
    const fn = steps(4, "jump-none");
    expect(fn(0)).toBe(0);
    expect(fn(0.25)).toBeCloseTo(1 / 3, 12);
    expect(fn(0.5)).toBeCloseTo(2 / 3, 12);
    expect(fn(0.75)).toBe(1);
    expect(fn(0.99)).toBe(1);
  });

  it("jump-both holds a level at both ends, with n + 1 levels in all", () => {
    const fn = steps(4, "jump-both");
    expect(fn(0)).toBeCloseTo(0.2, 12);
    expect(fn(0.25)).toBeCloseTo(0.4, 12);
    expect(fn(0.5)).toBeCloseTo(0.6, 12);
    expect(fn(0.75)).toBeCloseTo(0.8, 12);
    expect(fn(0.99)).toBeCloseTo(0.8, 12);
    expect(fn(1)).toBe(1);
  });

  it("refuses a count below 1, a fraction count, and jump-none with one step", () => {
    expect(() => steps(0)).toThrow(RangeError);
    expect(() => steps(2.5)).toThrow(RangeError);
    expect(() => steps(1, "jump-none")).toThrow(RangeError);
  });
});

describe("ease lookup", () => {
  it("returns the named easing", () => {
    expect(ease("quadOut")(0.3)).toBe(quadOut(0.3));
    expect(ease("linear")(0.3)).toBe(0.3);
  });

  it("refuses a name that is not built in", () => {
    expect(() => ease("wobble")).toThrow(RangeError);
  });

  it("tells built-in names from others", () => {
    expect(isEasingName("cubicIn")).toBe(true);
    expect(isEasingName("steps")).toBe(false);
    expect(isEasingName("toString")).toBe(false);
  });
});
