// A seeded random generator for bots, soak runs and fake games. The same seed always gives the same
// sequence, so a run that fails can be run again with its seed. It is not meant for security.

import { TestingError } from "./errors";

/** A seeded generator. Every method draws from the same sequence. */
export interface Random {
  /** The next number, from 0 up to but not including 1. */
  next(): number;
  /** A whole number from `min` to `max`, both included. */
  int(min: number, max: number): number;
  /** True with the given chance, from 0 (never) to 1 (always). */
  chance(probability: number): boolean;
  /**
   * One item of a non-empty list.
   * @throws TestingError with code "bad-argument" when the list is empty.
   */
  pick<Item>(items: readonly Item[]): Item;
}

/** The constant that spreads a seed's bits, taken from the golden ratio. */
const GOLDEN = 0x9e3779b9;

/** The step that the generator adds to its state on every draw. */
const STEP = 0x6d2b79f5;

/** 2^32, the size of the low half of a 64-bit seed. */
const TWO_POW_32 = 4294967296;

/**
 * Creates a generator from a seed: a safe whole number, or a text of decimal digits (for seeds above
 * 2^53, as the cards package writes them).
 * @throws TestingError with code "bad-argument" for any other seed.
 */
export function createRandom(seed: number | string): Random {
  let state = initialState(seed);
  const next = (): number => {
    state = (state + STEP) | 0;
    let mixed = Math.imul(state ^ (state >>> 15), 1 | state);
    mixed = (mixed + Math.imul(mixed ^ (mixed >>> 7), 61 | mixed)) ^ mixed;
    return ((mixed ^ (mixed >>> 14)) >>> 0) / TWO_POW_32;
  };
  return {
    next,
    int(min, max) {
      if (!Number.isInteger(min) || !Number.isInteger(max) || max < min) {
        throw new TestingError("bad-argument", `int needs whole numbers with min <= max, got ${min} and ${max}`);
      }
      return min + Math.floor(next() * (max - min + 1));
    },
    chance(probability) {
      if (!(probability >= 0 && probability <= 1)) {
        throw new TestingError("bad-argument", `chance needs a probability from 0 to 1, got ${probability}`);
      }
      return next() < probability;
    },
    pick<Item>(items: readonly Item[]): Item {
      if (items.length === 0) throw new TestingError("bad-argument", "pick needs a non-empty list");
      const index = Math.min(items.length - 1, Math.floor(next() * items.length));
      return items[index] as Item;
    },
  };
}

/** The first 32-bit state of the generator, mixed from the two halves of the seed. */
function initialState(seed: number | string): number {
  const { low, high } = splitSeed(seed);
  return mix32((low ^ mix32((high + GOLDEN) | 0)) | 0);
}

/** Splits a seed into its low and high 32 bits. A text seed is folded into 64 bits first. */
function splitSeed(seed: number | string): { low: number; high: number } {
  if (typeof seed === "number") {
    if (!Number.isSafeInteger(seed) || seed < 0) {
      throw new TestingError("bad-argument", `a numeric seed must be a safe whole number from 0 up, got ${seed}`);
    }
    return { low: seed % TWO_POW_32, high: Math.floor(seed / TWO_POW_32) };
  }
  if (!/^\d{1,40}$/.test(seed)) {
    throw new TestingError("bad-argument", `a text seed must be 1 to 40 decimal digits, got ${JSON.stringify(seed)}`);
  }
  const mask = (1n << 64n) - 1n;
  let folded = 0n;
  for (const digit of seed) folded = (folded * 10n + BigInt(digit)) & mask;
  return { low: Number(folded & 0xffffffffn), high: Number(folded >> 32n) };
}

/** A 32-bit finaliser that spreads every bit of the input over the output. */
function mix32(value: number): number {
  let mixed = value >>> 0;
  mixed = Math.imul(mixed ^ (mixed >>> 16), 0x85ebca6b);
  mixed = Math.imul(mixed ^ (mixed >>> 13), 0xc2b2ae35);
  return (mixed ^ (mixed >>> 16)) >>> 0;
}
