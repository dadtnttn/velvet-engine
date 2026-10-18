// The achievements of one game at run time: counters, unlocks, the goals and their cascades, progress
// notices, events, and the saved progress. Every change is applied to the state before any listener
// runs, so a listener always sees the whole result of the call that sent it. The rules are in
// docs/formats/ACHIEVEMENTS.md (Runtime rules, Saved progress).

import { EventEmitter, chooseSettingsBackend } from "@velvet/core";
import type { ChosenSettingsBackend, Listener, SettingsBackend, SettingsBackendKind, Unsubscribe } from "@velvet/core";
import { readAchievements, parseAchievements } from "./definitions";
import { AchievementError } from "./error";
import { ACHIEVEMENTS_KEY, ACHIEVEMENTS_STORAGE_PREFIX, DEFAULT_DEBOUNCE_MS } from "./format";
import { goalMet, goalProgress, type GoalReader } from "./goals";
import { buildProgress, readProgress } from "./progress";
import type {
  Achievements,
  AchievementCatalog,
  AchievementDefinition,
  AchievementEntry,
  AchievementEvents,
  AchievementLoad,
  AchievementProgress,
  AchievementsOptions,
  AchievementUnlock,
  CounterDefinition,
  ListOptions,
  ProgressDocument,
  ProgressNotice,
  SetCounterOptions,
  UnlockOptions,
} from "./types";

/** The largest safe whole number: the limit of a counter that has no maximum. */
const MAX_SAFE = Number.MAX_SAFE_INTEGER;

/** The options of a store, with the defaults filled in. */
interface ResolvedOptions {
  readonly key: string;
  readonly debounceMs: number;
  readonly clock: () => number;
  readonly dev: boolean;
  readonly onUnlock: ((unlock: AchievementUnlock) => void) | undefined;
}

/** Throws when a count or a value is not a safe whole number from 0. */
function requireCount(value: number, name: string): void {
  if (!Number.isSafeInteger(value) || value < 0) {
    throw new AchievementError("argument", `${name} must be a safe whole number from 0, got ${value}`);
  }
}

/** Clamps a counter value to the counter's maximum, when it has one. */
function clampCounter(value: number, max: number | null): number {
  return Math.min(value, max ?? MAX_SAFE);
}

/** Adds an id to the list under a key, making the list when it is new. The lists keep the order of the file. */
function addTo(map: Map<string, string[]>, key: string, id: string): void {
  const list = map.get(key);
  if (list === undefined) map.set(key, [id]);
  else list.push(id);
}

/** The points of a set of achievements. */
function sumPoints(achievements: readonly AchievementDefinition[]): number {
  return achievements.reduce((total, def) => total + def.points, 0);
}

/**
 * The gallery order: the groups come in the order of their first achievement in the file; inside a group,
 * the achievements come by `order`, and then by their place in the file.
 */
function galleryOrder(achievements: readonly AchievementDefinition[]): readonly AchievementDefinition[] {
  const groupRank = new Map<string, number>();
  for (const def of achievements) {
    if (!groupRank.has(def.group)) groupRank.set(def.group, groupRank.size);
  }
  return achievements
    .map((def, index) => ({ def, index, rank: groupRank.get(def.group) ?? 0 }))
    .sort((a, b) => a.rank - b.rank || a.def.order - b.def.order || a.index - b.index)
    .map((entry) => entry.def);
}

/** The store that {@link createAchievements} makes. Not exported: the game only sees the interface. */
class AchievementStore implements Achievements {
  readonly backendKind: SettingsBackendKind;
  readonly definitions: AchievementCatalog;
  private readonly key: string;
  private readonly backend: SettingsBackend;
  private readonly debounceMs: number;
  private readonly clock: () => number;
  private readonly dev: boolean;
  private readonly onUnlock: ((unlock: AchievementUnlock) => void) | undefined;
  private readonly events = new EventEmitter<AchievementEvents>();
  private readonly counterDefs = new Map<string, CounterDefinition>();
  private readonly achievementDefs = new Map<string, AchievementDefinition>();
  /** The achievements whose `all` or `any` goal lists each achievement, in the order of the file. */
  private readonly dependents = new Map<string, string[]>();
  /** The achievements with a counter goal, by counter, in the order of the file. */
  private readonly counterGoals = new Map<string, string[]>();
  /** The achievements that are not manual, in the order of the file. The candidates of a load. */
  private readonly automatic: readonly string[];
  /** The achievements in gallery order. */
  private readonly sorted: readonly AchievementDefinition[];
  /** The value of each counter that was changed or loaded. A counter that was never changed reads as 0. */
  private readonly counters = new Map<string, number>();
  /** The unlock time of each unlocked achievement. */
  private readonly unlockedAt = new Map<string, number>();
  /** How many progress steps of each achievement have been announced. */
  private readonly reported = new Map<string, number>();
  private readonly reader: GoalReader;
  private loading: Promise<AchievementLoad> | null = null;
  private timer: ReturnType<typeof setTimeout> | null = null;
  /** The writes in order: each waits for the one before, so the last state set is the last state written. */
  private writing: Promise<void> = Promise.resolve();

  constructor(catalog: AchievementCatalog, chosen: ChosenSettingsBackend, options: ResolvedOptions) {
    this.definitions = catalog;
    this.backendKind = chosen.kind;
    this.backend = chosen.backend;
    this.key = options.key;
    this.debounceMs = options.debounceMs;
    this.clock = options.clock;
    this.dev = options.dev;
    this.onUnlock = options.onUnlock;
    for (const def of catalog.counters) this.counterDefs.set(def.id, def);
    for (const def of catalog.achievements) {
      this.achievementDefs.set(def.id, def);
      const goal = def.goal;
      if (goal.kind === "counter") {
        addTo(this.counterGoals, goal.counter, def.id);
      } else if (goal.kind === "all" || goal.kind === "any") {
        for (const target of goal.of) addTo(this.dependents, target, def.id);
      }
    }
    this.automatic = catalog.achievements.filter((def) => def.goal.kind !== "manual").map((def) => def.id);
    this.sorted = galleryOrder(catalog.achievements);
    this.reader = {
      counterValue: (counter) => this.counterValue(counter),
      isUnlocked: (id) => this.unlockedAt.has(id),
    };
  }

  get unlockedCount(): number {
    return this.unlockedAt.size;
  }

  get totalCount(): number {
    return this.definitions.achievements.length;
  }

  get points(): number {
    return sumPoints(this.definitions.achievements.filter((def) => this.unlockedAt.has(def.id)));
  }

  get totalPoints(): number {
    return sumPoints(this.definitions.achievements);
  }

  increment(counter: string, by = 1): number {
    const def = this.counterDef(counter);
    requireCount(by, "by");
    return this.applyCounter(counter, clampCounter(this.counterValue(counter) + by, def.max));
  }

  setCounter(counter: string, value: number, options: SetCounterOptions = {}): number {
    const def = this.counterDef(counter);
    requireCount(value, "value");
    const target = clampCounter(value, def.max);
    if (target < this.counterValue(counter) && options.allowDecrease !== true) return this.counterValue(counter);
    return this.applyCounter(counter, target);
  }

  counter(counter: string): number {
    this.counterDef(counter);
    return this.counterValue(counter);
  }

  unlock(id: string, options: UnlockOptions = {}): boolean {
    const def = this.achievementDef(id);
    const force = options.force === true;
    if (force && !this.dev) throw new AchievementError("dev-only", `unlock("${id}", { force }) needs the dev option`);
    if (def.goal.kind !== "manual" && !force) {
      throw new AchievementError("not-manual", `"${id}" is unlocked by its goal, not by unlock()`);
    }
    if (this.unlockedAt.has(id)) return false;
    const now = this.readTime();
    this.unlockedAt.set(id, now);
    const unlocks: AchievementUnlock[] = [{ id, time: now }, ...this.cascade(this.dependents.get(id) ?? [], now)];
    this.finish(unlocks, true, true);
    return true;
  }

  isUnlocked(id: string): boolean {
    this.achievementDef(id);
    return this.unlockedAt.has(id);
  }

  progress(id: string): AchievementProgress {
    const def = this.achievementDef(id);
    const { current, target } = goalProgress(def.goal, this.reader);
    if (this.unlockedAt.has(id)) return { current: target, target, fraction: 1 };
    return { current, target, fraction: current / target };
  }

  list(options: ListOptions = {}): AchievementEntry[] {
    const includeHidden = options.includeHidden ?? true;
    const entries: AchievementEntry[] = [];
    for (const def of this.sorted) {
      const unlockedAt = this.unlockedAt.get(def.id) ?? null;
      const masked = def.hidden && unlockedAt === null;
      if (masked && !includeHidden) continue;
      entries.push({
        id: def.id,
        group: def.group,
        order: def.order,
        points: def.points,
        hidden: def.hidden,
        unlocked: unlockedAt !== null,
        unlocked_at: unlockedAt,
        masked,
        name: masked ? null : def.name,
        description: masked ? null : def.description,
        icon: masked ? null : def.icon,
        hint: masked ? def.hint : null,
        progress: masked ? null : this.progress(def.id),
      });
    }
    return entries;
  }

  load(): Promise<AchievementLoad> {
    // A read that the storage refused is tried again by the next load or save, so a storage that comes
    // back later still gives its progress.
    this.loading ??= this.readSaved().then((result) => {
      if (result.status === "unavailable") this.loading = null;
      return result;
    });
    return this.loading;
  }

  async save(): Promise<boolean> {
    this.cancelTimer();
    // A save must not replace progress it has not read, so it waits for the load. While the storage
    // refuses the read, nothing is written: the progress it holds may be better than this run's.
    const loaded = await this.load().catch(() => null);
    if (loaded?.status === "unavailable") return false;
    const document = buildProgress(this.definitions, this.counters, this.unlockedAt);
    const write = this.writing.then(() => this.writeDocument(document));
    this.writing = write.then(() => undefined);
    return write;
  }

  resetAll(): void {
    if (!this.dev) throw new AchievementError("dev-only", "resetAll() needs the dev option");
    this.counters.clear();
    this.unlockedAt.clear();
    this.reported.clear();
    this.finish([], true, true);
  }

  on<Type extends keyof AchievementEvents>(type: Type, listener: Listener<AchievementEvents[Type]>): Unsubscribe {
    return this.events.on(type, listener);
  }

  once<Type extends keyof AchievementEvents>(type: Type, listener: Listener<AchievementEvents[Type]>): Unsubscribe {
    return this.events.once(type, listener);
  }

  off<Type extends keyof AchievementEvents>(type: Type, listener: Listener<AchievementEvents[Type]>): void {
    this.events.off(type, listener);
  }

  /** The definition of a counter. @throws AchievementError with "unknown-counter". */
  private counterDef(counter: string): CounterDefinition {
    const def = this.counterDefs.get(counter);
    if (def === undefined) throw new AchievementError("unknown-counter", `"${counter}" is not a counter of the definitions`);
    return def;
  }

  /** The definition of an achievement. @throws AchievementError with "unknown-achievement". */
  private achievementDef(id: string): AchievementDefinition {
    const def = this.achievementDefs.get(id);
    if (def === undefined) throw new AchievementError("unknown-achievement", `"${id}" is not an achievement of the definitions`);
    return def;
  }

  /** The value of a counter: 0 when it was never changed. */
  private counterValue(counter: string): number {
    return this.counters.get(counter) ?? 0;
  }

  /** The clock's time, which must be a finite number. Read before any state changes, so a bad clock changes nothing. */
  private readTime(): number {
    const time = this.clock();
    if (typeof time !== "number" || !Number.isFinite(time)) {
      throw new AchievementError("argument", "the clock must give a finite time in milliseconds");
    }
    return time;
  }

  /** Sets a counter to a value that the caller has clamped, then runs the goals and ends the call. */
  private applyCounter(counter: string, value: number): number {
    if (value === this.counterValue(counter)) return value;
    const now = this.readTime();
    this.counters.set(counter, value);
    const unlocks = this.cascade(this.counterGoals.get(counter) ?? [], now);
    this.finish(unlocks, true, true);
    return value;
  }

  /**
   * Unlocks each candidate whose goal is met, and queues the achievements that depend on it. A queue runs until
   * it is empty, so an unlock reaches every dependent, in the order the cascade reaches them.
   * @returns the unlocks, in the order they happened.
   */
  private cascade(candidates: readonly string[], now: number): AchievementUnlock[] {
    const unlocks: AchievementUnlock[] = [];
    const queue = [...candidates];
    for (let index = 0; index < queue.length; index += 1) {
      const id = queue[index];
      if (id === undefined || this.unlockedAt.has(id)) continue;
      const def = this.achievementDefs.get(id);
      if (def === undefined || def.goal.kind === "manual" || !goalMet(def.goal, this.reader)) continue;
      this.unlockedAt.set(id, now);
      unlocks.push({ id, time: now });
      queue.push(...(this.dependents.get(id) ?? []));
    }
    return unlocks;
  }

  /** The number of the progress steps that an achievement's fraction has reached. */
  private reachedSteps(def: AchievementDefinition): number {
    const { current, target } = goalProgress(def.goal, this.reader);
    const fraction = current / target;
    return def.progress_steps.filter((step) => step <= fraction).length;
  }

  /**
   * Marks the steps that the current state has already reached as announced, without sending them. Used after a
   * load. A step that was announced before stays announced, even when the loaded value is lower.
   */
  private syncReported(): void {
    for (const def of this.definitions.achievements) {
      if (def.progress_steps.length === 0) continue;
      const done = this.reported.get(def.id) ?? 0;
      this.reported.set(def.id, Math.max(done, this.reachedSteps(def)));
    }
  }

  /**
   * Finds the progress steps that were reached since the last notice, in file order and step order. A hidden or
   * an unlocked achievement sends none.
   */
  private scanProgress(): ProgressNotice[] {
    const notices: ProgressNotice[] = [];
    for (const def of this.definitions.achievements) {
      if (def.progress_steps.length === 0 || def.hidden || this.unlockedAt.has(def.id)) continue;
      const reached = this.reachedSteps(def);
      const done = this.reported.get(def.id) ?? 0;
      if (reached <= done) continue;
      for (const step of def.progress_steps.slice(done, reached)) notices.push({ id: def.id, step });
      this.reported.set(def.id, reached);
    }
    return notices;
  }

  /**
   * Merges a loaded save into this run: a counter takes the larger value, and an unlock is added (the earlier time
   * wins when both have one). Returns true when a counter or an unlock changed.
   */
  private merge(counters: ReadonlyMap<string, number>, unlocked: ReadonlyMap<string, number>): boolean {
    let changed = false;
    for (const [counter, saved] of counters) {
      const def = this.counterDefs.get(counter);
      if (def === undefined) continue;
      const before = this.counterValue(counter);
      const after = clampCounter(Math.max(before, saved), def.max);
      if (after !== before) {
        this.counters.set(counter, after);
        changed = true;
      }
    }
    for (const [id, time] of unlocked) {
      const known = this.unlockedAt.get(id);
      if (known === undefined) {
        this.unlockedAt.set(id, time);
        changed = true;
      } else if (time < known) {
        this.unlockedAt.set(id, time);
      }
    }
    return changed;
  }

  /**
   * Ends a call. The state is already in place: this schedules the save when asked, finds the progress notices, and
   * announces the unlocks, the notices and the change, in that order.
   */
  private finish(unlocks: readonly AchievementUnlock[], changed: boolean, persist: boolean): void {
    if (persist) this.scheduleSave();
    this.announce(unlocks, this.scanProgress(), changed);
  }

  /**
   * Sends the events of a call. Each unlock calls the `onUnlock` hook and then sends `unlocked`; then come the
   * progress notices, then `changed`. A listener that throws does not stop the others: the first error is thrown
   * after every event has been sent.
   */
  private announce(unlocks: readonly AchievementUnlock[], notices: readonly ProgressNotice[], changed: boolean): void {
    const failures: unknown[] = [];
    const attempt = (action: () => void): void => {
      try {
        action();
      } catch (error) {
        failures.push(error);
      }
    };
    const hook = this.onUnlock;
    for (const unlock of unlocks) {
      if (hook !== undefined) attempt(() => hook(unlock));
      attempt(() => this.events.emit("unlocked", unlock));
    }
    for (const notice of notices) attempt(() => this.events.emit("progress", notice));
    if (changed) attempt(() => this.events.emit("changed"));
    if (failures.length > 0) throw failures[0];
  }

  /** Reads the saved progress and merges it, then checks every goal again. A goal that the saved counters meet unlocks now. */
  private async readSaved(): Promise<AchievementLoad> {
    let raw: unknown;
    try {
      raw = await this.backend.get(this.key);
    } catch {
      return { status: "unavailable", issues: [] };
    }
    const read = readProgress(raw, this.definitions);
    if (read.status === "fresh") return { status: "fresh", issues: [] };
    if (read.status === "damaged") return { status: "damaged", issues: read.issues };
    const merged = this.merge(read.counters, read.unlocked);
    this.syncReported();
    const unlocks = this.cascade(this.automatic, this.readTime());
    this.finish(unlocks, merged || unlocks.length > 0, unlocks.length > 0);
    return { status: "loaded", issues: [] };
  }

  /** Writes one document. Resolves false when the storage refuses it. */
  private async writeDocument(document: ProgressDocument): Promise<boolean> {
    try {
      await this.backend.set(this.key, document);
      return true;
    } catch {
      return false;
    }
  }

  /** Starts the save timer. A change that comes before it fires moves it on. */
  private scheduleSave(): void {
    this.cancelTimer();
    this.timer = setTimeout(() => {
      this.timer = null;
      void this.save();
    }, this.debounceMs);
  }

  private cancelTimer(): void {
    if (this.timer === null) return;
    clearTimeout(this.timer);
    this.timer = null;
  }
}

/**
 * Creates the achievements of a game. The progress is saved through the backend that is passed in, else the
 * bridge of Velvet Desktop, else localStorage, else memory, the same choice as the settings store makes.
 *
 * @param options - the definitions, the storage, the clock, the dev flag and the platform hook (see {@link AchievementsOptions}).
 * @throws AchievementError when the definitions are refused (the code is the first problem's).
 * @throws RangeError when `debounceMs` or `key` is not valid.
 */
export function createAchievements(options: AchievementsOptions): Achievements {
  const catalog =
    typeof options.definitions === "string" ? parseAchievements(options.definitions) : readAchievements(options.definitions);
  const debounceMs = options.debounceMs ?? DEFAULT_DEBOUNCE_MS;
  if (!Number.isFinite(debounceMs) || debounceMs < 0) {
    throw new RangeError(`debounceMs must be zero or a positive number, got ${debounceMs}`);
  }
  const key = options.key ?? ACHIEVEMENTS_KEY;
  if (key.length === 0) throw new RangeError("key must not be empty");
  const chosen = chooseSettingsBackend({
    explicit: options.backend,
    bridge: options.bridge,
    storage: options.storage,
    prefix: ACHIEVEMENTS_STORAGE_PREFIX,
  });
  return new AchievementStore(catalog, chosen, {
    key,
    debounceMs,
    clock: options.clock ?? Date.now,
    dev: options.dev === true,
    onUnlock: options.onUnlock,
  });
}
