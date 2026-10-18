// The public types of @velvet/achievements. The data objects use the names of the format
// (docs/formats/ACHIEVEMENTS.md), in snake_case; the methods of the runtime use camelCase.

import type { Listener, SettingsBackend, SettingsBackendKind, StorageLike, Unsubscribe } from "@velvet/core";
import type { AchievementIssue } from "./error";

/** A counter, as the definitions declare it. */
export interface CounterDefinition {
  /** The id of the counter. */
  readonly id: string;
  /** The highest value, or null when the counter has no limit. */
  readonly max: number | null;
  /** Whether the value is saved between runs. */
  readonly persist: boolean;
}

/**
 * The goal of an achievement. `kind` tells the four forms apart. In `all` and `any`, `of` lists the
 * achievements in the order of the file.
 */
export type AchievementGoal =
  | { readonly kind: "counter"; readonly counter: string; readonly at_least: number }
  | { readonly kind: "all"; readonly of: readonly string[] }
  | { readonly kind: "any"; readonly of: readonly string[]; readonly count: number }
  | { readonly kind: "manual" };

/** An achievement, as the definitions declare it, with the defaults filled in. */
export interface AchievementDefinition {
  /** The id of the achievement. */
  readonly id: string;
  /** The i18n key of the name. */
  readonly name: string;
  /** The i18n key of the description. */
  readonly description: string;
  /** The art id of the picture. */
  readonly icon: string;
  /** Whether the achievement is masked until it is unlocked. */
  readonly hidden: boolean;
  /** The i18n key of the hint of a masked achievement, or null. */
  readonly hint: string | null;
  /** The points that the achievement is worth. */
  readonly points: number;
  /** The gallery group. */
  readonly group: string;
  /** The place inside the group. */
  readonly order: number;
  /** The fractions of the goal that send a progress event, in increasing order. */
  readonly progress_steps: readonly number[];
  /** What unlocks the achievement. */
  readonly goal: AchievementGoal;
}

/** The definitions of a game, checked and in their normal form. */
export interface AchievementCatalog {
  readonly format: "velvet-achievements";
  readonly version: 1;
  readonly kind: "definitions";
  /** The counters, in the order of the file. */
  readonly counters: readonly CounterDefinition[];
  /** The achievements, in the order of the file. */
  readonly achievements: readonly AchievementDefinition[];
}

/** Progress toward a goal. `fraction` is `current / target`. */
export interface AchievementProgress {
  readonly current: number;
  readonly target: number;
  readonly fraction: number;
}

/** One entry of `list()`: an achievement with its display state. */
export interface AchievementEntry {
  readonly id: string;
  readonly group: string;
  readonly order: number;
  readonly points: number;
  readonly hidden: boolean;
  /** True when the achievement is unlocked. */
  readonly unlocked: boolean;
  /** The unlock time, or null while it is locked. */
  readonly unlocked_at: number | null;
  /** True for a hidden achievement that is still locked: the game shows it as "?". */
  readonly masked: boolean;
  /** The name key, or null while masked. */
  readonly name: string | null;
  /** The description key, or null while masked. */
  readonly description: string | null;
  /** The art id, or null while masked. */
  readonly icon: string | null;
  /** The hint key of a masked achievement that has one, otherwise null. */
  readonly hint: string | null;
  /** The progress, or null while masked. */
  readonly progress: AchievementProgress | null;
}

/** The payload of `unlocked`, and of the `onUnlock` hook. */
export interface AchievementUnlock {
  /** The achievement that unlocked. */
  readonly id: string;
  /** The clock's time of the unlock, in milliseconds. */
  readonly time: number;
}

/** The payload of `progress`: a locked achievement reached a step of its `progress_steps`. */
export interface ProgressNotice {
  /** The achievement. */
  readonly id: string;
  /** The step that was reached, as a fraction from 0 to 1. */
  readonly step: number;
}

/** The events of the runtime, by name. `changed` has no payload. */
export interface AchievementEvents {
  readonly unlocked: AchievementUnlock;
  readonly progress: ProgressNotice;
  readonly changed: void;
}

/** What `load()` found in the storage. */
export type LoadStatus = "fresh" | "loaded" | "damaged" | "unavailable";

/** The result of `load()`: the status, and the issues of a damaged save. */
export interface AchievementLoad {
  readonly status: LoadStatus;
  /** The problems of a damaged save, in the order of the document. Empty for the other statuses. */
  readonly issues: readonly AchievementIssue[];
}

/** The saved progress, as the runtime writes it. */
export interface ProgressDocument {
  readonly format: "velvet-achievements";
  readonly version: 1;
  readonly kind: "progress";
  /** The value of every counter with `persist: true`, by id. */
  readonly counters: Readonly<Record<string, number>>;
  /** The unlock time of every unlocked achievement, by id. */
  readonly unlocked: Readonly<Record<string, number>>;
}

/** Options of `setCounter`. */
export interface SetCounterOptions {
  /** Lets the value go down. Default false: a lower value changes nothing. */
  readonly allowDecrease?: boolean;
}

/** Options of `unlock`. */
export interface UnlockOptions {
  /** Unlocks an achievement whose goal is not manual. Only in a dev build. Default false. */
  readonly force?: boolean;
}

/** Options of `list`. */
export interface ListOptions {
  /** Whether masked achievements are listed. Default true. */
  readonly includeHidden?: boolean;
}

/** Options of {@link createAchievements}. */
export interface AchievementsOptions {
  /**
   * The definitions: the JSON text of the file, or the file already parsed (for example a JSON import). Either
   * way it is checked, and a refused file throws an AchievementError.
   */
  readonly definitions: string | object;
  /** A storage backend with `get` and `set`. It is used before the bridge and the web storage. */
  readonly backend?: SettingsBackend;
  /** The bridge whose settings hold the progress. Default: the bridge of Velvet Desktop, when it is there. `null` skips it. */
  readonly bridge?: { readonly settings: SettingsBackend } | null;
  /** The web storage that holds the progress in a browser. Default: localStorage, when it is allowed. `null` skips it. */
  readonly storage?: StorageLike | null;
  /** The key of the saved progress. Default "achievements". Two games on one storage need different keys. */
  readonly key?: string;
  /** Milliseconds that a change waits before it is saved. Default 250. */
  readonly debounceMs?: number;
  /** The clock: a function that returns the time of an unlock, in milliseconds. Default `Date.now`. */
  readonly clock?: () => number;
  /** True in a dev build: enables `unlock(id, { force })` and `resetAll()`. Default false. */
  readonly dev?: boolean;
  /**
   * The platform hook. Called once for each unlock that happens in this run, before its `unlocked` event. It is the
   * place where a Steam or other bridge reports the unlock. It is not called for the unlocks that a load restores.
   */
  readonly onUnlock?: (unlock: AchievementUnlock) => void;
}

/** The runtime of one game's achievements. Create it with {@link createAchievements}. */
export interface Achievements {
  /** Where the progress is kept: the bridge, localStorage, a custom backend, or memory. */
  readonly backendKind: SettingsBackendKind;
  /** The definitions in their normal form. */
  readonly definitions: AchievementCatalog;
  /**
   * Adds `by` to a counter, clamped to its maximum. Returns the new value.
   * @throws AchievementError with "unknown-counter", or "argument" when `by` is not a safe whole number from 0.
   */
  increment(counter: string, by?: number): number;
  /**
   * Sets a counter. A lower value changes nothing unless `allowDecrease` is set. Returns the new value.
   * @throws AchievementError with "unknown-counter", or "argument" when `value` is not a safe whole number from 0.
   */
  setCounter(counter: string, value: number, options?: SetCounterOptions): number;
  /**
   * The value of a counter: 0 when it was never changed.
   * @throws AchievementError with "unknown-counter".
   */
  counter(counter: string): number;
  /**
   * Unlocks an achievement. Returns true when this call unlocked it, false when it was unlocked already.
   * @throws AchievementError with "unknown-achievement", "not-manual", or "dev-only" (for `force`).
   */
  unlock(id: string, options?: UnlockOptions): boolean;
  /**
   * True when the achievement is unlocked.
   * @throws AchievementError with "unknown-achievement".
   */
  isUnlocked(id: string): boolean;
  /**
   * The progress toward the goal of an achievement. Masked achievements give their numbers too: the game decides
   * what to show.
   * @throws AchievementError with "unknown-achievement".
   */
  progress(id: string): AchievementProgress;
  /** The achievements in gallery order, with their display state. Reads only: it changes nothing. */
  list(options?: ListOptions): AchievementEntry[];
  /** The number of unlocked achievements. */
  readonly unlockedCount: number;
  /** The number of achievements. */
  readonly totalCount: number;
  /** The points of the unlocked achievements. */
  readonly points: number;
  /** The points of all the achievements, hidden ones included. */
  readonly totalPoints: number;
  /**
   * Reads the saved progress once, and merges it with the progress of this run. Repeated calls give the same
   * result. Rejects only when a listener, the `onUnlock` hook or the clock throws while the load is applied.
   */
  load(): Promise<AchievementLoad>;
  /**
   * Writes the progress now, and cancels a pending save. Waits for a pending load first. Resolves true when the
   * storage took the document, and false when it refused. Never rejects.
   */
  save(): Promise<boolean>;
  /**
   * Clears every counter and unlock, and saves the empty progress after the debounce. For dev builds only.
   * @throws AchievementError with "dev-only" outside a dev build.
   */
  resetAll(): void;
  /** Adds a listener for an event. Returns a function that removes it. */
  on<Type extends keyof AchievementEvents>(type: Type, listener: Listener<AchievementEvents[Type]>): Unsubscribe;
  /** Adds a listener that runs once, at the next event of its type. Returns a function that removes it. */
  once<Type extends keyof AchievementEvents>(type: Type, listener: Listener<AchievementEvents[Type]>): Unsubscribe;
  /** Removes a listener. Does nothing when it is not there. */
  off<Type extends keyof AchievementEvents>(type: Type, listener: Listener<AchievementEvents[Type]>): void;
}
