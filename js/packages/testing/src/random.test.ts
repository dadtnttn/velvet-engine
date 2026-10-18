import { describe, expect, it } from "vitest";
import { TestingError } from "./errors";
import { createRandom } from "./random";

describe("createRandom", () => {
  it("gives the same sequence for the same seed, and a different one for another seed", () => {
    const first = createRandom(42);
    const again = createRandom(42);
    const other = createRandom(43);
    const draws = Array.from({ length: 5 }, () => first.next());
    expect(Array.from({ length: 5 }, () => again.next())).toEqual(draws);
    expect(Array.from({ length: 5 }, () => other.next())).not.toEqual(draws);
  });

  it("keeps next() in [0, 1)", () => {
    const random = createRandom(7);
    for (let index = 0; index < 1000; index += 1) {
      const value = random.next();
      expect(value).toBeGreaterThanOrEqual(0);
      expect(value).toBeLessThan(1);
    }
  });

  it("gives whole numbers from min to max, both included", () => {
    const random = createRandom(9);
    const seen = new Set<number>();
    for (let index = 0; index < 500; index += 1) seen.add(random.int(2, 5));
    expect([...seen].sort()).toEqual([2, 3, 4, 5]);
  });

  it("follows a chance, and picks from a list", () => {
    const random = createRandom(11);
    expect(Array.from({ length: 50 }, () => random.chance(0))).not.toContain(true);
    expect(Array.from({ length: 50 }, () => random.chance(1))).not.toContain(false);
    const picks = new Set(Array.from({ length: 200 }, () => random.pick(["a", "b", "c"])));
    expect([...picks].sort()).toEqual(["a", "b", "c"]);
  });

  it("accepts a seed of decimal digits above 2^53, and tells such seeds apart", () => {
    const big = createRandom("18446744073709551615");
    const other = createRandom("18446744073709551614");
    expect(big.next()).not.toBe(other.next());
    expect(createRandom("42").next()).toBe(createRandom(42).next());
  });

  it("refuses a seed that is not a safe whole number or decimal digits", () => {
    expect(() => createRandom(-1)).toThrow(TestingError);
    expect(() => createRandom(1.5)).toThrow(/safe whole number/);
    expect(() => createRandom("12a")).toThrow(/decimal digits/);
  });

  it("refuses an empty list, a reversed range and a bad probability", () => {
    const random = createRandom(1);
    expect(() => random.pick([])).toThrow(/non-empty list/);
    expect(() => random.int(5, 2)).toThrow(/min <= max/);
    expect(() => random.chance(1.5)).toThrow(/probability/);
  });
});
