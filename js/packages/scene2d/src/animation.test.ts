// Tests for animations: frames change at their times (by frame rate or by per-frame durations), a loop wraps and a
// loop-free animation stops and calls back once, the playing directions give Aseprite's order (ping-pong runs to
// the end and back without repeating the end frames), a clock that is paused or slowed moves the animation the
// same way, seeking shows the frame of its time without calling back, and a sprite shows each frame.

import { GameClock } from "@velvet/core";
import { describe, expect, it, vi } from "vitest";
import { createAnimation, type PlayDirection } from "./animation";
import type { SpriteFrame } from "./sprite";
import { Sprite } from "./sprite";
import { fakeImage } from "./test-support";

/** Four frames, named by their index in the list, in a sheet of 40 pixels. */
function frames(count = 4): SpriteFrame[] {
  return Array.from({ length: count }, (_, index) => ({ name: `f${index}`, x: index * 10, y: 0, width: 10, height: 10 }));
}

/** The frame index of each step of a direction, read by seeking one second apart at one frame per second. */
function orderOf(direction: PlayDirection, count: number): number[] {
  const animation = createAnimation(frames(count), { fps: 1, direction, autoplay: false });
  const order: number[] = [];
  for (let second = 0; second < count * 2; second += 1) {
    animation.seek(second);
    order.push(animation.index);
  }
  return order;
}

describe("timing", () => {
  it("shows each frame for 1 over the frame rate", () => {
    const animation = createAnimation(frames(), { fps: 10 });
    expect(animation.index).toBe(0);
    animation.tick(0.05);
    expect(animation.index).toBe(0);
    animation.tick(0.05);
    expect(animation.index).toBe(1);
    animation.tick(0.2);
    expect(animation.index).toBe(3);
    expect(animation.time).toBeCloseTo(0.3, 9);
  });

  it("shows each frame for its own duration", () => {
    const animation = createAnimation(frames(2), { durations: [0.5, 0.1] });
    animation.tick(0.4);
    expect(animation.index).toBe(0);
    animation.tick(0.1);
    expect(animation.index).toBe(1);
  });

  it("loops, wrapping the time into the cycle", () => {
    const animation = createAnimation(frames(4), { fps: 10 });
    expect(animation.duration).toBeCloseTo(0.4, 9);
    animation.tick(0.45);
    expect(animation.index).toBe(0);
    expect(animation.time).toBeCloseTo(0.05, 9);
    expect(animation.playing).toBe(true);
  });

  it("stops on the last frame of a loop-free animation, and calls back once", () => {
    const onComplete = vi.fn();
    const animation = createAnimation(frames(3), { fps: 10, loop: false, onComplete });
    animation.tick(1);
    expect(animation.index).toBe(2);
    expect(animation.finished).toBe(true);
    expect(animation.playing).toBe(false);
    animation.tick(1);
    expect(onComplete).toHaveBeenCalledTimes(1);
  });

  it("plays again from the start when it is played after it finished", () => {
    const animation = createAnimation(frames(2), { fps: 10, loop: false });
    animation.tick(1);
    expect(animation.finished).toBe(true);
    animation.play();
    expect(animation.finished).toBe(false);
    expect(animation.index).toBe(0);
    animation.tick(0.1);
    expect(animation.index).toBe(1);
  });

  it("does not move while paused, and moves after play", () => {
    const animation = createAnimation(frames(), { fps: 10, autoplay: false });
    animation.tick(1);
    expect(animation.index).toBe(0);
    animation.play();
    animation.tick(0.1);
    expect(animation.index).toBe(1);
    animation.pause();
    animation.tick(1);
    expect(animation.index).toBe(1);
  });

  it("shows the first frame at once, and calls back for each change", () => {
    const seen: number[] = [];
    const animation = createAnimation(frames(3), { fps: 10, onFrame: (_frame, index) => seen.push(index) });
    animation.tick(0.1);
    animation.tick(0.1);
    expect(seen).toEqual([0, 1, 2]);
  });
});

describe("playing directions", () => {
  it("plays forward", () => {
    expect(orderOf("forward", 3)).toEqual([0, 1, 2, 0, 1, 2]);
  });

  it("plays in reverse", () => {
    expect(orderOf("reverse", 3)).toEqual([2, 1, 0, 2, 1, 0]);
  });

  it("plays a ping-pong to the end and back, without repeating the end frames", () => {
    expect(orderOf("pingpong", 4)).toEqual([0, 1, 2, 3, 2, 1, 0, 1]);
  });

  it("plays a reversed ping-pong from the end, the same way round", () => {
    expect(orderOf("pingpong_reverse", 4)).toEqual([3, 2, 1, 0, 1, 2, 3, 2]);
  });

  it("holds a single frame for every direction", () => {
    for (const direction of ["forward", "reverse", "pingpong", "pingpong_reverse"] as const) {
      expect(orderOf(direction, 1)).toEqual([0, 0]);
    }
  });
});

describe("the clock", () => {
  it("stops while the clock is paused", () => {
    const clock = new GameClock();
    const animation = createAnimation(frames(), { fps: 10, clock });
    clock.pause();
    animation.tick(5);
    expect(animation.index).toBe(0);
    clock.resume();
    animation.tick(0.1);
    expect(animation.index).toBe(1);
  });

  it("runs at the clock's speed", () => {
    const clock = new GameClock({ speeds: [1, 2] });
    const animation = createAnimation(frames(), { fps: 10, clock });
    clock.setSpeed(2);
    // A real step of 0.05 seconds is 0.1 seconds of game time at speed 2, which is exactly one frame.
    animation.tick(0.05);
    expect(animation.index).toBe(1);
    expect(animation.time).toBeCloseTo(0.1, 9);
  });
});

describe("seeking and the sprite", () => {
  it("seeks to a time and shows its frame, without calling back", () => {
    const onComplete = vi.fn();
    const animation = createAnimation(frames(3), { fps: 10, loop: false, onComplete, autoplay: false });
    animation.seek(0.25);
    expect(animation.index).toBe(2);
    animation.seek(99);
    expect(animation.index).toBe(2);
    expect(animation.finished).toBe(false);
    expect(onComplete).not.toHaveBeenCalled();
  });

  it("goes back to the first frame on reset", () => {
    const animation = createAnimation(frames(), { fps: 10 });
    animation.tick(0.3);
    animation.reset();
    expect(animation.index).toBe(0);
    expect(animation.time).toBe(0);
  });

  it("sets the frame of its sprite on every change, and on the first frame", () => {
    const sprite = new Sprite({ image: fakeImage(40, 10) });
    const list = frames(3);
    createAnimation(list, { fps: 10, sprite }).tick(0.1);
    expect(sprite.frame).toBe(list[1]);
  });

  it("refuses an empty list, durations of the wrong length, and a rate of zero", () => {
    expect(() => createAnimation([])).toThrow(RangeError);
    expect(() => createAnimation(frames(2), { durations: [0.1] })).toThrow(RangeError);
    expect(() => createAnimation(frames(2), { fps: 0 })).toThrow(RangeError);
    expect(() => createAnimation(frames(2), { durations: [0.1, -1] })).toThrow(RangeError);
  });
});
