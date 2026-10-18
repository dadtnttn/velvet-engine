// The saved progress: how the runtime writes it, and how it reads a saved value back. A damaged save
// is used whole or not at all, and never stops the game: the reader reports its issues and the store
// starts from the progress it has.

import type { AchievementIssue, AchievementErrorCode } from "./error";
import { ACHIEVEMENTS_FORMAT, ACHIEVEMENTS_VERSION } from "./format";
import { isRecord } from "./guards";
import type { AchievementCatalog, ProgressDocument } from "./types";

/** What a read of the saved value found. */
export type ProgressRead =
  /** Nothing was stored. */
  | { readonly status: "fresh" }
  /** The saved value could not be used. Nothing from it is applied. */
  | { readonly status: "damaged"; readonly issues: readonly AchievementIssue[] }
  /**
   * The saved value was read. `counters` holds the persisted counters that the definitions declare, and
   * `unlocked` the unlock times of the declared achievements. Unknown names were dropped.
   */
  | {
      readonly status: "loaded";
      readonly counters: ReadonlyMap<string, number>;
      readonly unlocked: ReadonlyMap<string, number>;
    };

/** Makes the read of a damaged save. A problem of the whole document has no path. */
function damaged(code: AchievementErrorCode, message: string, path?: string): ProgressRead {
  const issue: AchievementIssue = path === undefined ? { code, message } : { code, message, path };
  return { status: "damaged", issues: [issue] };
}

/**
 * Reads a saved value. A missing value is fresh. Anything else must be a progress document of version 1 with
 * both maps: a counter value must be a safe whole number from 0, and an unlock time a finite number. Any problem
 * makes the whole save damaged.
 * @param raw - the value that the storage gave back.
 * @param catalog - the definitions, which say what the names mean.
 */
export function readProgress(raw: unknown, catalog: AchievementCatalog): ProgressRead {
  if (raw === undefined || raw === null) return { status: "fresh" };
  if (!isRecord(raw)) return damaged("structure", "the saved progress is not an object");
  if (raw.format !== ACHIEVEMENTS_FORMAT) return damaged("format", `the saved format is not "${ACHIEVEMENTS_FORMAT}"`, "format");
  if (raw.kind !== "progress") return damaged("format", 'the saved kind is not "progress"', "kind");
  if (raw.version !== ACHIEVEMENTS_VERSION) {
    return damaged("version", `the saved version is not ${ACHIEVEMENTS_VERSION}`, "version");
  }
  const counters = raw.counters;
  const unlocked = raw.unlocked;
  const issues: AchievementIssue[] = [];
  if (!isRecord(counters)) issues.push({ code: "structure", message: "counters must be an object", path: "counters" });
  if (!isRecord(unlocked)) issues.push({ code: "structure", message: "unlocked must be an object", path: "unlocked" });
  if (!isRecord(counters) || !isRecord(unlocked)) return { status: "damaged", issues };

  const values = new Map<string, number>();
  for (const def of catalog.counters) {
    if (!def.persist || !Object.hasOwn(counters, def.id)) continue;
    const value = counters[def.id];
    const path = `counters.${def.id}`;
    if (typeof value !== "number") {
      issues.push({ code: "structure", message: "a counter value must be a number", path });
    } else if (!Number.isSafeInteger(value) || value < 0) {
      issues.push({ code: "value", message: `${value} is not a counter value: a whole number from 0`, path });
    } else {
      values.set(def.id, value);
    }
  }

  const times = new Map<string, number>();
  for (const def of catalog.achievements) {
    if (!Object.hasOwn(unlocked, def.id)) continue;
    const time = unlocked[def.id];
    const path = `unlocked.${def.id}`;
    if (typeof time !== "number" || !Number.isFinite(time)) {
      issues.push({ code: "structure", message: "an unlock time must be a finite number", path });
    } else {
      times.set(def.id, time);
    }
  }

  if (issues.length > 0) return { status: "damaged", issues };
  return { status: "loaded", counters: values, unlocked: times };
}

/**
 * Builds the document that is saved: every persisted counter, even at 0, and every unlock, both in the order of
 * the definitions.
 * @param catalog - the definitions.
 * @param counters - the value of each counter that was changed in this run or loaded.
 * @param unlocked - the unlock time of each unlocked achievement.
 */
export function buildProgress(
  catalog: AchievementCatalog,
  counters: ReadonlyMap<string, number>,
  unlocked: ReadonlyMap<string, number>,
): ProgressDocument {
  const counterValues: Record<string, number> = {};
  for (const def of catalog.counters) {
    if (def.persist) counterValues[def.id] = counters.get(def.id) ?? 0;
  }
  const unlockTimes: Record<string, number> = {};
  for (const def of catalog.achievements) {
    const time = unlocked.get(def.id);
    if (time !== undefined) unlockTimes[def.id] = time;
  }
  return { format: ACHIEVEMENTS_FORMAT, version: ACHIEVEMENTS_VERSION, kind: "progress", counters: counterValues, unlocked: unlockTimes };
}
