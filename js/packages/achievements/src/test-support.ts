// Helpers shared by the tests of this package. Not part of the package's API.

import { readFileSync } from "node:fs";
import { expect } from "vitest";
import type { SettingsBackend, StorageLike } from "@velvet/core";
import { AchievementError, type AchievementErrorCode } from "./error";

/** The folder of the shared achievement fixtures, found from this file (src, achievements, packages, js, root). */
const FIXTURES = new URL("../../../../docs/formats/fixtures/achievements/", import.meta.url);

/** Reads a file of the shared fixtures, by its path inside the folder (for example `invalid/cycle.json`). */
export function readFixture(path: string): string {
  return readFileSync(new URL(path, FIXTURES), "utf8");
}

/** A copy of a JSON value, so that a stored value does not change when the caller changes its own. */
function copyJson(value: unknown): unknown {
  return value === undefined ? undefined : (JSON.parse(JSON.stringify(value)) as unknown);
}

/**
 * A storage backend in memory. It keeps a copy of each value, as a real storage does, counts its writes, and can be
 * made to refuse reads or writes.
 */
export class MemoryBackend implements SettingsBackend {
  private readonly values = new Map<string, unknown>();
  /** How many values the backend has taken. */
  writes = 0;
  /** When true, `get` rejects. */
  failReads = false;
  /** When true, `set` rejects. */
  failWrites = false;

  async get(key: string): Promise<unknown> {
    if (this.failReads) throw new Error("the storage refused the read");
    return copyJson(this.values.get(key));
  }

  async set(key: string, value: unknown): Promise<void> {
    if (this.failWrites) throw new Error("the storage refused the write");
    this.writes += 1;
    this.values.set(key, copyJson(value));
  }

  /** A copy of the value stored under a key, or undefined when nothing is stored. */
  peek(key: string): unknown {
    return copyJson(this.values.get(key));
  }

  /** Stores a value under a key, as an earlier run would have left it. It does not count as a write. */
  put(key: string, value: unknown): void {
    this.values.set(key, copyJson(value));
  }
}

/** An in-memory stand-in for the Web Storage methods that the localStorage backend uses. */
export class MemoryStorage implements StorageLike {
  private readonly items = new Map<string, string>();

  get length(): number {
    return this.items.size;
  }

  key(index: number): string | null {
    return [...this.items.keys()][index] ?? null;
  }

  getItem(key: string): string | null {
    return this.items.get(key) ?? null;
  }

  setItem(key: string, value: string): void {
    this.items.set(key, value);
  }

  removeItem(key: string): void {
    this.items.delete(key);
  }

  /** The text stored under a key, or undefined. */
  text(key: string): string | undefined {
    return this.items.get(key);
  }
}

/** A clock that the test moves by hand. `read` is the function that the store takes. */
export function manualClock(start = 1000): { now: number; read: () => number } {
  const clock = {
    now: start,
    read: (): number => clock.now,
  };
  return clock;
}

/**
 * Asserts that `call` throws an AchievementError whose code is `code`.
 * @param call - the call that should throw.
 * @param code - the code that the error must carry.
 */
export function expectCode(call: () => unknown, code: AchievementErrorCode): void {
  let thrown: unknown;
  try {
    call();
  } catch (error) {
    thrown = error;
  }
  expect(thrown, `expected a ${code} error`).toBeInstanceOf(AchievementError);
  expect(thrown).toMatchObject({ code });
}
