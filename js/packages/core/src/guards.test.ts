// Tests for the runtime type guards.

import { describe, expect, it } from "vitest";
import { isPositiveInteger, isRecord } from "./guards";

describe("isRecord", () => {
  it("accepts plain objects and rejects arrays, null and primitives", () => {
    expect(isRecord({})).toBe(true);
    expect(isRecord({ a: 1 })).toBe(true);
    expect(isRecord([])).toBe(false);
    expect(isRecord(null)).toBe(false);
    expect(isRecord("text")).toBe(false);
    expect(isRecord(3)).toBe(false);
    expect(isRecord(undefined)).toBe(false);
  });
});

describe("isPositiveInteger", () => {
  it("accepts whole numbers from 1 upwards", () => {
    expect(isPositiveInteger(1)).toBe(true);
    expect(isPositiveInteger(42)).toBe(true);
  });

  it("rejects zero, fractions, negatives, non-finite numbers and other types", () => {
    for (const value of [0, -1, 1.5, Number.NaN, Number.POSITIVE_INFINITY, "2", null]) {
      expect(isPositiveInteger(value)).toBe(false);
    }
  });
});
