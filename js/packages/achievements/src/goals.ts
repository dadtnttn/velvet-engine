// The rules of the goals: when a goal is met, and how far it has got. The functions read a running
// store through the small GoalReader interface, so they hold no state of their own.

import type { AchievementGoal } from "./types";

/** What a goal reads from a running store. */
export interface GoalReader {
  /** The value of a counter: 0 when it was never changed. */
  counterValue(counter: string): number;
  /** True when the achievement is unlocked. */
  isUnlocked(id: string): boolean;
}

/** The number of the listed achievements that are unlocked. */
function countUnlocked(ids: readonly string[], reader: GoalReader): number {
  let count = 0;
  for (const id of ids) {
    if (reader.isUnlocked(id)) count += 1;
  }
  return count;
}

/**
 * True when the goal is met now. A counter goal is met at its `at_least`; an `all` goal when every listed
 * achievement is unlocked; an `any` goal when `count` listed achievements are. A manual goal is never met
 * by a check: only the game unlocks it.
 */
export function goalMet(goal: AchievementGoal, reader: GoalReader): boolean {
  switch (goal.kind) {
    case "counter":
      return reader.counterValue(goal.counter) >= goal.at_least;
    case "all":
      return goal.of.every((id) => reader.isUnlocked(id));
    case "any":
      return countUnlocked(goal.of, reader) >= goal.count;
    case "manual":
      return false;
  }
}

/**
 * How far a goal has got: `current` out of `target`. A counter's `current` stops at its `at_least`; an `all`
 * goal counts its unlocked parts; an `any` goal counts them up to `count`; a manual goal is 0 of 1 until the
 * game unlocks it.
 */
export function goalProgress(goal: AchievementGoal, reader: GoalReader): { readonly current: number; readonly target: number } {
  switch (goal.kind) {
    case "counter":
      return { current: Math.min(reader.counterValue(goal.counter), goal.at_least), target: goal.at_least };
    case "all":
      return { current: countUnlocked(goal.of, reader), target: goal.of.length };
    case "any":
      return { current: Math.min(countUnlocked(goal.of, reader), goal.count), target: goal.count };
    case "manual":
      return { current: 0, target: 1 };
  }
}
