// Tests for the frame statistics: the percentiles, the summary, the ring of recent values, and the
// sampler's counters, which must not count a stall as a frame.

import { describe, expect, it } from "vitest";
import { FRAME_WINDOW, FrameSampler, Ring, percentile, summarize } from "./stats";

describe("percentile", () => {
  const hundred = Array.from({ length: 100 }, (_, index) => index + 1);

  it("picks the value at the nearest rank", () => {
    expect(percentile(hundred, 50)).toBe(50);
    expect(percentile(hundred, 95)).toBe(95);
    expect(percentile(hundred, 99)).toBe(99);
    expect(percentile(hundred, 100)).toBe(100);
  });

  it("rounds the rank up, and never goes below the first value", () => {
    expect(percentile([10, 20, 30, 40], 50)).toBe(20);
    expect(percentile([10, 20, 30, 40], 95)).toBe(40);
    expect(percentile([10, 20, 30, 40], 0)).toBe(10);
  });

  it("is NaN when there are no values", () => {
    expect(percentile([], 50)).toBeNaN();
  });
});

describe("summarize", () => {
  it("is null for an empty list", () => {
    expect(summarize([])).toBeNull();
  });

  it("gives the count, mean, median, 95th percentile and maximum, and leaves the input in order", () => {
    const values = [30, 10, 20];
    expect(summarize(values)).toEqual({ count: 3, mean: 20, p50: 20, p95: 30, max: 30 });
    expect(values).toEqual([30, 10, 20]);
  });
});

describe("Ring", () => {
  it("keeps the newest values, oldest first, and drops the oldest when full", () => {
    const ring = new Ring(3);
    for (const value of [1, 2, 3]) ring.push(value);
    expect(ring.toArray()).toEqual([1, 2, 3]);
    ring.push(4);
    expect(ring.toArray()).toEqual([2, 3, 4]);
    ring.push(5);
    ring.push(6);
    ring.push(7);
    expect(ring.toArray()).toEqual([5, 6, 7]);
  });

  it("is empty after clear", () => {
    const ring = new Ring(2);
    ring.push(1);
    ring.clear();
    expect(ring.toArray()).toEqual([]);
  });
});

describe("FrameSampler", () => {
  it("measures the time between frames, and the first frame only sets the origin", () => {
    const sampler = new FrameSampler();
    sampler.onFrame(0);
    expect(sampler.stats().frames).toBe(0);
    sampler.onFrame(16);
    const stats = sampler.stats();
    expect(stats.frames).toBe(1);
    expect(stats.lastMs).toBe(16);
    expect(stats.fps).toBeCloseTo(62.5, 9);
  });

  it("counts frames over 16.7 ms and over 33 ms", () => {
    const sampler = new FrameSampler();
    for (const time of [0, 16, 33, 67]) sampler.onFrame(time);
    // Intervals: 16, 17, 34. The 17 and the 34 are long at 60 Hz; only the 34 is long at 30 Hz.
    const stats = sampler.stats();
    expect(stats.longFrames16).toBe(2);
    expect(stats.longFrames33).toBe(1);
  });

  it("does not count a stall of more than a second as a frame", () => {
    const sampler = new FrameSampler();
    sampler.onFrame(0);
    sampler.onFrame(5000);
    expect(sampler.stats().frames).toBe(0);
    sampler.onFrame(5016);
    expect(sampler.stats().frames).toBe(1);
    expect(sampler.stats().longFrames16).toBe(0);
  });

  it("gives percentiles over the recent window only, while the frame count keeps growing", () => {
    const sampler = new FrameSampler();
    let time = 0;
    sampler.onFrame(time);
    for (let index = 0; index < 10; index += 1) {
      time += 100;
      sampler.onFrame(time);
    }
    for (let index = 0; index < FRAME_WINDOW; index += 1) {
      time += 10;
      sampler.onFrame(time);
    }
    const stats = sampler.stats();
    expect(stats.frames).toBe(10 + FRAME_WINDOW);
    expect(stats.maxMs).toBe(10);
    expect(stats.p95Ms).toBe(10);
    expect(stats.longFrames33).toBe(10);
  });

  it("averages the update and render time of loop frames, and has none before the loop reports", () => {
    const sampler = new FrameSampler();
    expect(sampler.stats().updateMs).toBeNull();
    expect(sampler.stats().renderMs).toBeNull();
    sampler.onLoopFrame(2, 4);
    sampler.onLoopFrame(4, 6);
    expect(sampler.stats().updateMs).toBe(3);
    expect(sampler.stats().renderMs).toBe(5);
  });

  it("clears its history and counters on reset", () => {
    const sampler = new FrameSampler();
    for (const time of [0, 40, 80]) sampler.onFrame(time);
    sampler.onLoopFrame(1, 1);
    sampler.reset();
    const stats = sampler.stats();
    expect(stats.frames).toBe(0);
    expect(stats.longFrames33).toBe(0);
    expect(stats.updateMs).toBeNull();
    expect(sampler.recentIntervals()).toEqual([]);
  });
});
