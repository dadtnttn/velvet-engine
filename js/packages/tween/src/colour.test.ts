// Tests for colours and for interpolation between values: parsing the supported colour forms,
// mixing in OKLab, writing colours back as CSS text, and the interpolation of numbers, arrays,
// objects and colours, including extrapolation past the ends.

import { describe, expect, it } from "vitest";
import { formatColour, mixColours, parseColour, type Colour } from "./colour";
import { interpolate } from "./interpolate";

const BLACK: Colour = { red: 0, green: 0, blue: 0, alpha: 1, hasAlpha: false };
const WHITE: Colour = { red: 255, green: 255, blue: 255, alpha: 1, hasAlpha: false };
const CLEAR_BLACK: Colour = { red: 0, green: 0, blue: 0, alpha: 0, hasAlpha: true };

describe("parseColour", () => {
  it("reads the hex forms, including the short ones", () => {
    expect(parseColour("#f80")).toEqual({ red: 255, green: 136, blue: 0, alpha: 1, hasAlpha: false });
    expect(parseColour("#102030")).toEqual({ red: 16, green: 32, blue: 48, alpha: 1, hasAlpha: false });
    const solidHalf = parseColour("#ff000080");
    expect(solidHalf?.hasAlpha).toBe(true);
    expect(solidHalf?.alpha).toBeCloseTo(128 / 255, 12);
    expect(parseColour("#f008")?.alpha).toBeCloseTo(136 / 255, 12);
  });

  it("reads rgb and rgba, with commas, spaces, a slash and percentages", () => {
    expect(parseColour("rgb(10, 20, 30)")).toEqual({ red: 10, green: 20, blue: 30, alpha: 1, hasAlpha: false });
    expect(parseColour("rgba(10 20 30 / 50%)")).toEqual({
      red: 10,
      green: 20,
      blue: 30,
      alpha: 0.5,
      hasAlpha: true,
    });
    const percent = parseColour("rgb(100%, 0%, 50%)");
    expect(percent?.red).toBeCloseTo(255, 9);
    expect(percent?.green).toBe(0);
    expect(percent?.blue).toBeCloseTo(127.5, 9);
  });

  it("returns null for text that is not a supported colour", () => {
    for (const text of ["red", "#12", "#ggg", "rgb(1, 2)", "rgb(a, b, c)", "hsl(0, 100%, 50%)"]) {
      expect(parseColour(text)).toBeNull();
    }
  });
});

describe("mixColours", () => {
  it("returns the ends at 0 and 1", () => {
    const start = mixColours(BLACK, WHITE, 0);
    const end = mixColours(BLACK, WHITE, 1);
    // The published OKLab matrices round-trip to within about 1e-4 of a channel, which is invisible.
    expect(start.red).toBeCloseTo(0, 3);
    expect(end.red).toBeCloseTo(255, 3);
    expect(end.blue).toBeCloseTo(255, 3);
  });

  it("gives a grey halfway between black and white, lighter than the plain sRGB middle of 128", () => {
    // OKLab lightness is linear, so the middle of black and white is lightness one half, which
    // is about 99 in sRGB. A plain sRGB mix would give 128.
    const middle = mixColours(BLACK, WHITE, 0.5);
    expect(middle.red).toBeCloseTo(middle.green, 2);
    expect(middle.green).toBeCloseTo(middle.blue, 2);
    expect(middle.red).toBeGreaterThanOrEqual(98);
    expect(middle.red).toBeLessThanOrEqual(100);
  });

  it("mixes the alpha linearly, and keeps alpha when either end has it", () => {
    const half = mixColours(CLEAR_BLACK, { ...WHITE, alpha: 1, hasAlpha: false }, 0.5);
    expect(half.alpha).toBeCloseTo(0.5, 12);
    expect(half.hasAlpha).toBe(true);
  });

  it("passes between red and blue without going grey in the middle", () => {
    const red: Colour = { red: 255, green: 0, blue: 0, alpha: 1, hasAlpha: false };
    const blue: Colour = { red: 0, green: 0, blue: 255, alpha: 1, hasAlpha: false };
    const middle = mixColours(red, blue, 0.5);
    expect(middle.red).toBeGreaterThan(100);
    expect(middle.blue).toBeGreaterThan(100);
    // A grey middle would have its channels within a few units of each other.
    const channels = [middle.red, middle.green, middle.blue];
    expect(Math.max(...channels) - Math.min(...channels)).toBeGreaterThan(50);
  });
});

describe("formatColour", () => {
  it("writes rgb, or rgba when asked, with whole channels", () => {
    expect(formatColour({ red: 127.6, green: 0, blue: 300, alpha: 1, hasAlpha: false }, false)).toBe(
      "rgb(128, 0, 255)",
    );
    expect(formatColour({ red: 0, green: 0, blue: 0, alpha: 0.5, hasAlpha: true }, true)).toBe(
      "rgba(0, 0, 0, 0.5)",
    );
  });
});

describe("interpolate", () => {
  it("interpolates numbers, exact at the ends and extrapolated past them", () => {
    const fn = interpolate(0, 10);
    expect(fn(0)).toBe(0);
    expect(fn(1)).toBe(10);
    expect(fn(0.5)).toBe(5);
    expect(fn(1.5)).toBe(15);
  });

  it("interpolates colour texts, and returns the ends as they were given", () => {
    const fn = interpolate("#fff", "#000");
    expect(fn(0)).toBe("#fff");
    expect(fn(1)).toBe("#000");
    expect(fn(0.5)).toMatch(/^rgb\(\d+, \d+, \d+\)$/);
  });

  it("writes rgba when either colour has an alpha", () => {
    const fn = interpolate("rgba(0, 0, 0, 0)", "rgba(255, 255, 255, 1)");
    expect(fn(0.5)).toMatch(/, 0\.5\)$/);
  });

  it("interpolates arrays, returning a new array each time", () => {
    const from = [0, 10];
    const fn = interpolate(from, [10, 20]);
    expect(fn(0.5)).toEqual([5, 15]);
    expect(fn(0)).toEqual([0, 10]);
    expect(fn(0)).not.toBe(from);
  });

  it("interpolates objects of numbers with the same keys", () => {
    expect(interpolate({ x: 0, y: 4 }, { x: 2, y: 0 })(0.5)).toEqual({ x: 1, y: 2 });
  });

  it("refuses ends of different kinds or sizes, and colours it cannot read", () => {
    expect(() => interpolate(0, "#fff")).toThrow(TypeError);
    expect(() => interpolate([0], [0, 1])).toThrow(RangeError);
    expect(() => interpolate({ a: 0 }, { b: 1 })).toThrow(RangeError);
    expect(() => interpolate("red", "blue")).toThrow(RangeError);
    expect(() => interpolate(Number.NaN, 1)).toThrow(RangeError);
  });
});
