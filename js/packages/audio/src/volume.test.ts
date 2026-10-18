// Tests for the volume curve: the gain of a slider position, its decibels, and the rule that gains
// multiply, so that levels add up in decibels.

import { describe, expect, it } from "vitest";
import { decibelsToGain, volumeToDecibels, volumeToGain } from "./volume";

describe("volumeToGain", () => {
  it("is silence at zero and unity at one", () => {
    expect(volumeToGain(0)).toBe(0);
    expect(volumeToGain(1)).toBe(1);
  });

  it("is the volume cubed, which is 60 dB per decade", () => {
    for (const volume of [0.05, 0.1, 0.25, 0.5, 0.8, 0.99]) {
      expect(volumeToGain(volume)).toBeCloseTo(volume ** 3, 12);
    }
    expect(volumeToGain(0.5)).toBeCloseTo(0.125, 12);
  });

  it("rises with the volume", () => {
    let previous = -1;
    for (let step = 0; step <= 100; step += 1) {
      const gain = volumeToGain(step / 100);
      expect(gain).toBeGreaterThanOrEqual(previous);
      previous = gain;
    }
  });

  it("multiplies: the gain of two volumes is the gain of their product", () => {
    expect(volumeToGain(0.5) * volumeToGain(0.8)).toBeCloseTo(volumeToGain(0.4), 12);
  });
});

describe("volumeToDecibels", () => {
  it("gives -Infinity for silence and 0 dB for unity", () => {
    expect(volumeToDecibels(0)).toBe(Number.NEGATIVE_INFINITY);
    expect(volumeToDecibels(1)).toBe(0);
  });

  it("gives -60 dB at 0.1 and about -18 dB at 0.5", () => {
    expect(volumeToDecibels(0.1)).toBeCloseTo(-60, 9);
    expect(volumeToDecibels(0.5)).toBeCloseTo(-18.0618, 3);
  });

  it("adds up the decibels of two levels that multiply", () => {
    const sum = volumeToDecibels(0.5) + volumeToDecibels(0.8);
    expect(volumeToDecibels(0.4)).toBeCloseTo(sum, 9);
  });
});

describe("decibelsToGain", () => {
  it("is unity at 0 dB and silence at -Infinity", () => {
    expect(decibelsToGain(0)).toBe(1);
    expect(decibelsToGain(Number.NEGATIVE_INFINITY)).toBe(0);
  });

  it("is about a quarter at -12 dB, the default ducking", () => {
    expect(decibelsToGain(-12)).toBeCloseTo(0.2512, 3);
  });

  it("refuses NaN and +Infinity", () => {
    expect(() => decibelsToGain(Number.NaN)).toThrow(RangeError);
    expect(() => decibelsToGain(Number.POSITIVE_INFINITY)).toThrow(RangeError);
  });
});

describe("volume range", () => {
  it("refuses a volume outside 0 to 1, and a value that is not a number", () => {
    for (const volume of [-0.1, 1.1, Number.NaN, Number.POSITIVE_INFINITY]) {
      expect(() => volumeToGain(volume)).toThrow(RangeError);
      expect(() => volumeToDecibels(volume)).toThrow(RangeError);
    }
  });
});
