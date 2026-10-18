// Runs the velvet-achievements conformance suite (docs/formats/fixtures/achievements): the normal form of the
// valid definitions, the refused documents with their codes and paths, and the scripted cases. The suite is
// shared with other readers, so each case states its results in the format's own terms: the events of each
// step in order, the codes of the refusals, and the values that a check reads.

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { AchievementError } from "./error";
import { createAchievements, parseAchievements } from "./index";
import { MemoryBackend, manualClock, readFixture } from "./test-support";
import type { AchievementEntry, AchievementLoad, AchievementUnlock, Achievements } from "./types";

/** The key that the runs save under: the default of the store. */
const KEY = "achievements";

/** A refused document: its file, its code, and a path that one of its issues has. */
interface InvalidCase {
  readonly file: string;
  readonly code: string;
  readonly path?: string;
}

/** The fields that a normal-form entry must have. Other fields are not checked. */
interface NormalCase {
  readonly id: string;
  readonly fields: Readonly<Record<string, unknown>>;
}

/** One event that a step sends, as the suite writes it. */
type LoggedEvent =
  | { readonly type: "unlocked"; readonly id: string; readonly time: number }
  | { readonly type: "progress"; readonly id: string; readonly step: number }
  | { readonly type: "changed" };

/** One step of a case. `do` says what happens; the other fields say with what, and what must follow. */
interface Step {
  readonly do: "tick" | "increment" | "set_counter" | "unlock" | "reset_all" | "load" | "restart" | "save" | "wait" | "check";
  readonly counter?: string;
  readonly by?: number;
  readonly value?: number;
  readonly allow_decrease?: boolean;
  readonly id?: string;
  readonly force?: boolean;
  readonly now?: number;
  readonly saved?: unknown;
  readonly ms?: number;
  readonly events?: readonly LoggedEvent[];
  readonly error?: string;
  readonly returns?: unknown;
  readonly status?: string;
  readonly codes?: readonly string[];
  readonly counters?: Readonly<Record<string, number>>;
  readonly unlocked?: Readonly<Record<string, number>>;
  readonly progress?: Readonly<Record<string, { current: number; target: number; fraction: number }>>;
  readonly points?: number;
  readonly unlocked_count?: number;
  readonly total_count?: number;
  readonly total_points?: number;
  readonly list?: readonly Readonly<Record<string, unknown>>[];
  readonly include_hidden?: boolean;
  readonly writes?: number;
  readonly stored?: unknown;
}

/** A scripted run: its steps, and whether it is a dev build. */
interface Case {
  readonly name: string;
  readonly dev?: boolean;
  readonly steps: readonly Step[];
}

/** The whole suite file. */
interface Suite {
  readonly definitions: string;
  readonly normal: {
    readonly counters: readonly NormalCase[];
    readonly achievements: readonly NormalCase[];
  };
  readonly invalid: readonly InvalidCase[];
  readonly cases: readonly Case[];
}

const suite = JSON.parse(readFixture("cases.json")) as Suite;
const catalogText = readFixture(suite.definitions);
const catalog = parseAchievements(catalogText);

/** Reads one field of a value that the suite compares by its fields. */
function fieldOf(item: unknown, name: string): unknown {
  return (item as Record<string, unknown>)[name];
}

/** Checks that an item has the listed fields, with the listed values. */
function expectFields(item: unknown, fields: Readonly<Record<string, unknown>>): void {
  for (const [name, value] of Object.entries(fields)) {
    expect(fieldOf(item, name), `field ${name}`).toEqual(value);
  }
}

/** Throws the refusal that a call gives. Fails the test when the call is not refused. */
function refusalOf(call: () => unknown): AchievementError {
  let thrown: unknown;
  try {
    call();
  } catch (error) {
    thrown = error;
  }
  expect(thrown, "expected a refusal").toBeInstanceOf(AchievementError);
  return thrown as AchievementError;
}

/** The unlocks in a list of events, as the hook sees them. */
function unlocksOf(events: readonly LoggedEvent[]): AchievementUnlock[] {
  const unlocks: AchievementUnlock[] = [];
  for (const event of events) {
    if (event.type === "unlocked") unlocks.push({ id: event.id, time: event.time });
  }
  return unlocks;
}

/** The unlocked achievements of a store, with their unlock times. */
function unlockedMap(store: Achievements): Record<string, number | null> {
  const map: Record<string, number | null> = {};
  for (const entry of store.list()) {
    if (entry.unlocked) map[entry.id] = entry.unlocked_at;
  }
  return map;
}

/** Checks the whole list, in order, with the fields that each entry gives. */
function checkList(entries: readonly AchievementEntry[], expected: readonly Readonly<Record<string, unknown>>[], label: string): void {
  expect(entries.length, `${label}: the length of the list`).toBe(expected.length);
  for (const [index, fields] of expected.entries()) {
    const entry = entries[index];
    for (const [name, value] of Object.entries(fields)) {
      expect(fieldOf(entry, name), `${label}: entry ${index} (${String(fields.id)}) field ${name}`).toEqual(value);
    }
  }
}

/** One run of a case: a storage, a clock, the store that is in use, and what the store has sent since the step began. */
class Run {
  readonly backend = new MemoryBackend();
  readonly clock = manualClock(1000);
  readonly dev: boolean;
  events: LoggedEvent[] = [];
  hooked: AchievementUnlock[] = [];
  store: Achievements;

  constructor(dev: boolean) {
    this.dev = dev;
    this.store = this.open();
  }

  /** Makes a store on this run's storage and clock, and records what it sends. */
  open(): Achievements {
    const store = createAchievements({
      definitions: catalogText,
      backend: this.backend,
      clock: this.clock.read,
      dev: this.dev,
      onUnlock: (unlock) => {
        this.hooked.push(unlock);
      },
    });
    store.on("unlocked", (unlock) => {
      this.events.push({ type: "unlocked", id: unlock.id, time: unlock.time });
    });
    store.on("progress", (notice) => {
      this.events.push({ type: "progress", id: notice.id, step: notice.step });
    });
    store.on("changed", () => {
      this.events.push({ type: "changed" });
    });
    return store;
  }
}

/** The value that a step needs, or a failure when the suite is wrong. */
function need<Value>(value: Value | undefined, what: string): Value {
  if (value === undefined) throw new Error(`the suite is missing ${what}`);
  return value;
}

/** Lets the pending promises run, so that a save that a timer started has written. */
async function flushPromises(): Promise<void> {
  for (let index = 0; index < 100; index += 1) await Promise.resolve();
}

/** Runs one step, and returns what the call gave. */
async function perform(run: Run, step: Step): Promise<unknown> {
  switch (step.do) {
    case "tick":
      run.clock.now = need(step.now, "now");
      return undefined;
    case "increment":
      return run.store.increment(need(step.counter, "counter"), step.by);
    case "set_counter":
      return run.store.setCounter(need(step.counter, "counter"), need(step.value, "value"), {
        allowDecrease: step.allow_decrease === true,
      });
    case "unlock":
      return run.store.unlock(need(step.id, "id"), { force: step.force === true });
    case "reset_all":
      run.store.resetAll();
      return undefined;
    case "load":
      if (step.saved !== undefined) run.backend.put(KEY, step.saved);
      return run.store.load();
    case "restart":
      run.store = run.open();
      return run.store.load();
    case "save":
      return run.store.save();
    case "wait":
      await vi.advanceTimersByTimeAsync(need(step.ms, "ms"));
      await flushPromises();
      return undefined;
    case "check":
      return undefined;
  }
}

/** Checks what a load or a restart found. */
function checkLoad(result: AchievementLoad, step: Step, label: string): void {
  if (step.status !== undefined) expect(result.status, `${label}: status`).toBe(step.status);
  if (step.codes !== undefined) expect(result.issues.map((issue) => issue.code), `${label}: codes`).toEqual(step.codes);
}

/** Checks the state that a check step names. Only the fields that the step gives are checked. */
function checkState(run: Run, step: Step, label: string): void {
  const store = run.store;
  for (const [id, value] of Object.entries(step.counters ?? {})) {
    expect(store.counter(id), `${label}: counter ${id}`).toBe(value);
  }
  if (step.unlocked !== undefined) expect(unlockedMap(store), `${label}: unlocked`).toEqual(step.unlocked);
  for (const [id, expected] of Object.entries(step.progress ?? {})) {
    expect(store.progress(id), `${label}: progress of ${id}`).toEqual(expected);
  }
  if (step.points !== undefined) expect(store.points, `${label}: points`).toBe(step.points);
  if (step.unlocked_count !== undefined) expect(store.unlockedCount, `${label}: unlocked_count`).toBe(step.unlocked_count);
  if (step.total_count !== undefined) expect(store.totalCount, `${label}: total_count`).toBe(step.total_count);
  if (step.total_points !== undefined) expect(store.totalPoints, `${label}: total_points`).toBe(step.total_points);
  if (step.list !== undefined) {
    checkList(store.list({ includeHidden: step.include_hidden ?? true }), step.list, label);
  }
  if (step.writes !== undefined) expect(run.backend.writes, `${label}: writes`).toBe(step.writes);
  if (step.stored !== undefined) expect(run.backend.peek(KEY) ?? null, `${label}: stored`).toEqual(step.stored);
}

/** Runs one case, step by step, and checks each step against what the suite says it must do. */
async function runCase(testCase: Case): Promise<void> {
  const run = new Run(testCase.dev === true);
  for (const [index, step] of testCase.steps.entries()) {
    const label = `step ${index + 1} (${step.do})`;
    run.events = [];
    run.hooked = [];
    let result: unknown;
    let thrown: unknown;
    try {
      result = await perform(run, step);
    } catch (error) {
      thrown = error;
    }
    if (step.error !== undefined) {
      expect(thrown, `${label}: expected ${step.error}`).toBeInstanceOf(AchievementError);
      expect((thrown as AchievementError).code, `${label}: code`).toBe(step.error);
      expect(run.events, `${label}: a refused call sends no events`).toEqual([]);
      expect(run.hooked, `${label}: a refused call unlocks nothing`).toEqual([]);
      continue;
    }
    if (thrown !== undefined) throw thrown;
    expect(run.hooked, `${label}: the onUnlock calls match the unlocked events`).toEqual(unlocksOf(run.events));
    if (step.events !== undefined) expect(run.events, `${label}: events`).toEqual(step.events);
    if (step.returns !== undefined) expect(result, `${label}: returns`).toEqual(step.returns);
    if (step.do === "load" || step.do === "restart") checkLoad(result as AchievementLoad, step, label);
    if (step.do === "check") checkState(run, step, label);
  }
}

describe("velvet-achievements conformance: the valid definitions", () => {
  it("keeps the counters and the achievements in the order of the file", () => {
    expect(catalog.counters.map((counter) => counter.id)).toEqual(["kills", "gold", "steps"]);
    expect(catalog.achievements.map((def) => def.id)).toEqual([
      "prologue",
      "first_blood",
      "veteran",
      "butcher",
      "purse",
      "good_company",
      "secret_ending",
      "lantern",
      "marathon",
    ]);
  });

  for (const expected of suite.normal.counters) {
    it(`counter ${expected.id} has the fields of the fixture`, () => {
      const counter = catalog.counters.find((item) => item.id === expected.id);
      expect(counter).toBeDefined();
      expectFields(counter, expected.fields);
    });
  }

  for (const expected of suite.normal.achievements) {
    it(`achievement ${expected.id} has the fields of the fixture`, () => {
      const def = catalog.achievements.find((item) => item.id === expected.id);
      expect(def).toBeDefined();
      expectFields(def, expected.fields);
    });
  }
});

describe("velvet-achievements conformance: refused documents", () => {
  for (const testCase of suite.invalid) {
    it(`${testCase.file} is refused with ${testCase.code}`, () => {
      const text = readFixture(testCase.file);
      const error = refusalOf(() => parseAchievements(text));
      expect(error.code).toBe(testCase.code);
      expect(error.issues[0]?.code).toBe(testCase.code);
      if (testCase.path !== undefined) {
        expect(error.issues.map((issue) => issue.path)).toContain(testCase.path);
      }
    });
  }
});

describe("velvet-achievements conformance: scripted cases", () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  for (const testCase of suite.cases) {
    it(testCase.name, async () => {
      await runCase(testCase);
    });
  }
});
