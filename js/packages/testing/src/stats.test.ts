import { describe, expect, it } from "vitest";
import { percentile, summarize } from "./stats";

describe("summarize", () => {
  it("gives nulls for an empty list", () => {
    expect(summarize([])).toEqual({ count: 0, min: null, max: null, mean: null, p50: null, p90: null, p99: null });
  });

  it("gives the nearest-rank percentiles of 1 to 100", () => {
    const values = Array.from({ length: 100 }, (_, index) => index + 1);
    expect(summarize(values)).toEqual({ count: 100, min: 1, max: 100, mean: 50.5, p50: 50, p90: 90, p99: 99 });
  });

  it("sorts its input and leaves NaN out", () => {
    const summary = summarize([3, Number.NaN, 1, 2]);
    expect(summary.count).toBe(3);
    expect(summary.min).toBe(1);
    expect(summary.max).toBe(3);
    expect(summary.p50).toBe(2);
  });
});

describe("percentile", () => {
  it("gives the first value at 0 and the last at 100", () => {
    expect(percentile([5, 6, 7], 0)).toBe(5);
    expect(percentile([5, 6, 7], 100)).toBe(7);
    expect(percentile([], 50)).toBeNull();
  });
});
