// Tests for timelines: sequences and parallel children, labels and relative positions, nesting,
// scrubbing to any time, playing and reversing, the time scale, calls and completion callbacks, and
// the rules about which playbacks may join a timeline. Every playback here has its own test ticker,
// so no test shares time with another.

import { describe, expect, it } from "vitest";
import { timeline } from "./timeline";
import { Ticker } from "./ticker";
import { tween, type Tween } from "./tween";

/** A number tween from 0 to `to` that lasts `seconds`, on `ticker`. */
function line(ticker: Ticker, seconds: number, to = 10): Tween<number> {
  return tween({ from: 0, to, duration: seconds, ticker });
}

describe("sequences and parallel children", () => {
  it("plays its children one after the other, and seeks each to the right state", () => {
    const ticker = new Ticker();
    const first = line(ticker, 1);
    const second = tween({ from: 0, to: 100, duration: 2, ticker });
    const sequence = timeline({ ticker }).add(first).add(second);
    expect(sequence.duration).toBe(3);

    sequence.seek(0.5);
    expect(first.value).toBeCloseTo(5, 12);
    expect(second.value).toBe(0);
    sequence.seek(1.5);
    expect(first.value).toBe(10);
    expect(second.value).toBeCloseTo(25, 12);
    sequence.seek(3);
    expect(first.value).toBe(10);
    expect(second.value).toBe(100);
    sequence.seek(0);
    expect(first.value).toBe(0);
    expect(second.value).toBe(0);
  });

  it("runs children placed at 0 together", () => {
    const ticker = new Ticker();
    const first = line(ticker, 1);
    const second = tween({ from: 0, to: 100, duration: 2, ticker });
    const parallel = timeline({ ticker }).add(first, 0).add(second, 0);
    expect(parallel.duration).toBe(2);
    parallel.seek(1);
    expect(first.value).toBe(10);
    expect(second.value).toBeCloseTo(50, 12);
  });

  it("holds only the timeline on its ticker, not the children that it took", () => {
    const ticker = new Ticker();
    timeline({ ticker }).add(line(ticker, 1)).add(line(ticker, 1));
    expect(ticker.size).toBe(1);
  });
});

describe("positions", () => {
  it("places a child at a label, by name", () => {
    const ticker = new Ticker();
    const child = line(ticker, 1);
    const sequence = timeline({ ticker }).label("mid", 1).add(child, "mid");
    expect(sequence.timeOf("mid")).toBe(1);
    sequence.seek(0.5);
    expect(child.value).toBe(0);
    sequence.seek(2);
    expect(child.value).toBe(10);
  });

  it("places a child after the end with +=, and before it with -=", () => {
    const ticker = new Ticker();
    const first = line(ticker, 1);
    const second = tween({ from: 0, to: 100, duration: 2, ticker });
    const third = line(ticker, 1);
    const sequence = timeline({ ticker }).add(first).add(second, "+=0.5").add(third, "-=1");
    expect(sequence.duration).toBe(3.5);
    sequence.seek(3);
    expect(second.value).toBeCloseTo(75, 12);
    expect(third.value).toBeCloseTo(5, 12);
  });

  it("starts with the previous child using <, and ends with it using >, with shifts", () => {
    const ticker = new Ticker();
    const first = line(ticker, 1);
    const second = line(ticker, 2);
    const third = line(ticker, 1);
    const fourth = line(ticker, 1);
    const fifth = line(ticker, 1);
    const sequence = timeline({ ticker })
      .add(first)
      .add(second)
      .add(third, "<")
      .add(fourth, ">")
      .add(fifth, "<0.5");
    // second starts at 1, third starts with it at 1, fourth starts when third ends (2),
    // and fifth starts half a second after fourth (2.5).
    expect(sequence.duration).toBe(3.5);
    sequence.seek(1.5);
    expect(third.value).toBeCloseTo(5, 12);
    sequence.seek(2.25);
    expect(fourth.value).toBeCloseTo(2.5, 12);
    sequence.seek(3.5);
    expect(fifth.value).toBe(10);
  });

  it("refuses an unknown label, a position before the start, and names that look like positions", () => {
    const ticker = new Ticker();
    const sequence = timeline({ ticker });
    expect(() => sequence.add(line(ticker, 1), "nope")).toThrow(RangeError);
    expect(() => sequence.add(line(ticker, 1), -1)).toThrow(RangeError);
    expect(() => sequence.add(line(ticker, 1), "-=5")).toThrow(RangeError);
    expect(() => sequence.label("<bad")).toThrow(RangeError);
    expect(() => sequence.label("1.5")).toThrow(RangeError);
    sequence.label("mid", 1);
    expect(() => sequence.label("mid", 2)).toThrow(RangeError);
  });
});

describe("nesting and ownership", () => {
  it("plays a nested timeline as one child, and seeks its children inside it", () => {
    const ticker = new Ticker();
    const first = line(ticker, 1);
    const second = line(ticker, 1);
    const inner = timeline({ticker}).add(first).add(second);
    const outer = timeline({ ticker }).add(line(ticker, 1)).add(inner);
    expect(outer.duration).toBe(3);
    outer.seek(2.5);
    expect(first.value).toBe(10);
    expect(second.value).toBeCloseTo(5, 12);
  });

  it("measures a nested timeline again when children are added to it after it joined", () => {
    const ticker = new Ticker();
    const inner = timeline({ ticker });
    const outer = timeline({ ticker }).add(inner);
    expect(outer.duration).toBe(0);
    inner.add(line(ticker, 2));
    expect(outer.duration).toBe(2);
    outer.seek(1);
    expect(inner.time).toBe(1);
  });

  it("refuses to hold itself, or a timeline that holds it", () => {
    const ticker = new Ticker();
    const outer = timeline({ ticker });
    const inner = timeline({ ticker });
    outer.add(inner);
    expect(() => outer.add(outer)).toThrow(RangeError);
    expect(() => inner.add(outer)).toThrow(RangeError);
  });

  it("lets a child join one timeline only, and refuses a cancelled one", () => {
    const ticker = new Ticker();
    const child = line(ticker, 1);
    timeline({ ticker }).add(child);
    expect(() => timeline({ ticker }).add(child)).toThrow(RangeError);
    const cancelled = line(ticker, 1);
    cancelled.cancel();
    expect(() => timeline({ ticker }).add(cancelled)).toThrow(RangeError);
  });

  it("stops drawing a child that was cancelled after it joined", () => {
    const ticker = new Ticker();
    const child = line(ticker, 1);
    const sequence = timeline({ ticker }).add(child);
    sequence.seek(0.5);
    expect(child.value).toBeCloseTo(5, 12);
    child.cancel();
    sequence.seek(1);
    expect(child.value).toBeCloseTo(5, 12);
  });
});

describe("scrubbing", () => {
  it("gives the same state at a time, whatever the order of the seeks", () => {
    const ticker = new Ticker();
    const first = line(ticker, 1);
    const second = tween({ from: 0, to: 100, duration: 2, ticker });
    const sequence = timeline({ ticker }).add(first).add(second);
    for (const time of [2.2, 0.4, 1.9, 0, 3, 1.1, 0.4]) {
      sequence.seek(time);
      expect(first.value).toBeCloseTo(10 * Math.min(1, time), 9);
      expect(second.value).toBeCloseTo(50 * Math.min(2, Math.max(0, time - 1)), 9);
    }
  });
});

describe("playing, reversing and time scale", () => {
  it("plays forward through its children, and completes once at the end", async () => {
    const ticker = new Ticker();
    const first = line(ticker, 1);
    const second = tween({ from: 0, to: 100, duration: 2, ticker });
    let completions = 0;
    const sequence = timeline({
      ticker,
      onComplete: () => {
        completions += 1;
      },
    })
      .add(first)
      .add(second);
    ticker.advance(1.5);
    expect(first.value).toBe(10);
    expect(second.value).toBeCloseTo(25, 12);
    ticker.advance(1.5);
    expect(second.value).toBe(100);
    expect(completions).toBe(1);
    expect(sequence.isDone).toBe(true);
    await expect(sequence.finished).resolves.toBe("complete");
  });

  it("reverses from the end and completes at zero", () => {
    const ticker = new Ticker();
    const first = line(ticker, 1);
    const second = tween({ from: 0, to: 100, duration: 2, ticker });
    const sequence = timeline({ ticker }).add(first).add(second);
    sequence.seek(sequence.duration);
    sequence.reverse();
    ticker.advance(1.5);
    expect(first.value).toBe(10);
    expect(second.value).toBeCloseTo(25, 12);
    ticker.advance(1.5);
    expect(first.value).toBe(0);
    expect(second.value).toBe(0);
    expect(sequence.isDone).toBe(true);
  });

  it("time scale plays the whole timeline faster or slower", () => {
    const ticker = new Ticker();
    const child = line(ticker, 1);
    const sequence = timeline({ ticker, timeScale: 2 }).add(child);
    ticker.advance(0.25);
    expect(sequence.time).toBeCloseTo(0.5, 12);
    expect(child.value).toBeCloseTo(5, 12);
  });
});

describe("the view of a timeline", () => {
  it("lists its items with their starts and durations, and its labels", () => {
    const ticker = new Ticker();
    const first = line(ticker, 1);
    const second = line(ticker, 2);
    const sequence = timeline({ ticker }).label("mid", 0.5).add(first).add(second, "+=0.5").call(() => undefined, "mid");
    expect(sequence.spans()).toEqual([
      { kind: "child", start: 0, duration: 1, child: first },
      { kind: "child", start: 1.5, duration: 2, child: second },
      { kind: "call", start: 0.5, duration: 0, child: undefined },
    ]);
    expect(sequence.labelTimes()).toEqual(new Map([["mid", 0.5]]));
  });
});

describe("calls and completion callbacks", () => {
  it("runs a call when the playhead passes it, in either direction, and not on a seek", () => {
    const ticker = new Ticker();
    const log: string[] = [];
    const sequence = timeline({ ticker }).add(line(ticker, 1)).call(() => log.push("half"), 0.5);
    ticker.advance(0.25);
    expect(log).toEqual([]);
    ticker.advance(0.5);
    expect(log).toEqual(["half"]);
    sequence.seek(0);
    expect(log).toEqual(["half"]);
    sequence.seek(1);
    sequence.reverse();
    ticker.advance(0.75);
    expect(log).toEqual(["half", "half"]);
  });

  it("runs a child's onComplete when the timeline plays past the child's end", () => {
    const ticker = new Ticker();
    let finishedCount = 0;
    const child = tween({
      from: 0,
      to: 10,
      duration: 1,
      ticker,
      onComplete: () => {
        finishedCount += 1;
      },
    });
    timeline({ ticker }).add(child);
    ticker.advance(1.5);
    expect(finishedCount).toBe(1);
  });

  it("has the duration of its last item, calls included", () => {
    const ticker = new Ticker();
    const sequence = timeline({ ticker }).add(line(ticker, 1)).call(() => undefined, 2.5);
    expect(sequence.duration).toBe(2.5);
  });
});
