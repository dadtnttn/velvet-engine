// Reads a definitions document into a checked AchievementCatalog. Every rule of
// docs/formats/ACHIEVEMENTS.md is checked here. The envelope (format, kind and version) is checked
// first, and a problem there is fatal. Every other problem is collected with its path, so one run
// shows all of them, and the first one gives the error code.

import { AchievementError, type AchievementErrorCode, type AchievementIssue } from "./error";
import {
  ACHIEVEMENTS_FORMAT,
  ACHIEVEMENTS_VERSION,
  DEFAULT_GROUP,
  MAX_KEY_LENGTH,
  MAX_ORDER,
  MAX_POINTS,
  isArtId,
  isId,
  isKeyText,
} from "./format";
import { isRecord } from "./guards";
import type { AchievementCatalog, AchievementDefinition, AchievementGoal, CounterDefinition } from "./types";

/** The largest safe whole number: the limit of counter values, of `max` and of `at_least`. */
const MAX_SAFE = Number.MAX_SAFE_INTEGER;

/** The four kinds of goal. */
const GOAL_KINDS = ["counter", "all", "any", "manual"] as const;

/** One kind of goal, as the form of the goal names it. */
type GoalKind = (typeof GOAL_KINDS)[number];

/** Reads a field into a value, or returns undefined after it has added an issue. */
type FieldReader<Value> = (raw: unknown, path: string, issues: AchievementIssue[]) => Value | undefined;

/** The counters that the file declares: the maximum of each, by id, for the goals that name them. */
interface CounterScope {
  readonly maxOf: ReadonlyMap<string, number | null>;
}

/** A reference from an achievement to another one, for the search for loops. */
interface Edge {
  readonly to: string;
  readonly path: string;
}

/** Makes an issue that has a path. */
function issueAt(code: AchievementErrorCode, message: string, path: string): AchievementIssue {
  return { code, message, path };
}

/** Builds the error of a refused document, from all its issues. */
function refusal(issues: readonly AchievementIssue[]): AchievementError {
  const first = issues[0];
  const count = issues.length === 1 ? "1 problem" : `${issues.length} problems`;
  return new AchievementError(
    first?.code ?? "structure",
    `the achievements have ${count}; the first: ${first?.message ?? "unknown"}`,
    issues,
  );
}

/** Reads an id. A missing id, a value that is not a string and a string that breaks the pattern each give an issue. */
function readId(raw: unknown, path: string, issues: AchievementIssue[]): string | undefined {
  if (raw === undefined) {
    issues.push(issueAt("structure", `${path} is required`, path));
    return undefined;
  }
  if (typeof raw !== "string") {
    issues.push(issueAt("structure", `${path} must be an id: a string`, path));
    return undefined;
  }
  if (!isId(raw)) {
    issues.push(issueAt("bad-id", `"${raw}" is not an id: a lowercase letter, then up to 63 lowercase letters, digits or "_"`, path));
    return undefined;
  }
  return raw;
}

/** Reads an art id. */
function readArtId(raw: unknown, path: string, issues: AchievementIssue[]): string | undefined {
  if (typeof raw !== "string") {
    issues.push(issueAt("structure", `${path} must be an art id: a string`, path));
    return undefined;
  }
  if (!isArtId(raw)) {
    issues.push(issueAt("bad-id", `"${raw}" is not an art id: lowercase segments joined by dots, at most 128 characters`, path));
    return undefined;
  }
  return raw;
}

/** Reads an i18n key: text of 1 to 200 characters. */
function readKey(raw: unknown, path: string, issues: AchievementIssue[]): string | undefined {
  if (typeof raw !== "string") {
    issues.push(issueAt("structure", `${path} must be an i18n key: a string`, path));
    return undefined;
  }
  if (!isKeyText(raw)) {
    issues.push(issueAt("value", `${path} must be an i18n key: text of 1 to ${MAX_KEY_LENGTH} characters`, path));
    return undefined;
  }
  return raw;
}

/** Reads a field that must be present, then reads it with `read`. */
function readRequired<Value>(raw: unknown, path: string, issues: AchievementIssue[], read: FieldReader<Value>): Value | undefined {
  if (raw === undefined) {
    issues.push(issueAt("structure", `${path} is required`, path));
    return undefined;
  }
  return read(raw, path, issues);
}

/** Reads a whole number from `min` to `max`. */
function readWhole(raw: unknown, path: string, min: number, max: number, issues: AchievementIssue[]): number | undefined {
  if (typeof raw !== "number") {
    issues.push(issueAt("structure", `${path} must be a whole number`, path));
    return undefined;
  }
  if (!Number.isInteger(raw) || raw < min || raw > max) {
    issues.push(issueAt("value", `${raw} is not a whole number from ${min} to ${max}`, path));
    return undefined;
  }
  return raw;
}

/** Reads a true-or-false field. */
function readFlag(raw: unknown, path: string, issues: AchievementIssue[]): boolean | undefined {
  if (typeof raw !== "boolean") {
    issues.push(issueAt("structure", `${path} must be true or false`, path));
    return undefined;
  }
  return raw;
}

/**
 * Reads the progress steps: fractions above 0 and below 1, in strictly increasing order. A missing list is
 * empty. Returns undefined, with issues, when a step is wrong.
 */
function readSteps(raw: unknown, path: string, issues: AchievementIssue[]): number[] | undefined {
  if (raw === undefined) return [];
  if (!Array.isArray(raw)) {
    issues.push(issueAt("structure", `${path} must be a list of numbers`, path));
    return undefined;
  }
  const items: readonly unknown[] = raw;
  const steps: number[] = [];
  let valid = true;
  for (const [index, item] of items.entries()) {
    const itemPath = `${path}[${index}]`;
    if (typeof item !== "number") {
      issues.push(issueAt("structure", `${itemPath} must be a number`, itemPath));
      valid = false;
    } else if (!(item > 0 && item < 1)) {
      issues.push(issueAt("value", `${item} is not a step: use a number above 0 and below 1`, itemPath));
      valid = false;
    } else {
      const previous = steps[steps.length - 1];
      if (previous !== undefined && item <= previous) {
        issues.push(issueAt("value", `the steps must increase: ${item} does not come after ${previous}`, itemPath));
        valid = false;
      } else {
        steps.push(item);
      }
    }
  }
  return valid ? steps : undefined;
}

/** Reads the counters. A counter whose id is valid is declared even when another of its fields is wrong. */
function readCounters(raw: unknown, issues: AchievementIssue[]): { defs: CounterDefinition[]; scope: CounterScope } {
  const defs: CounterDefinition[] = [];
  const maxOf = new Map<string, number | null>();
  if (raw === undefined) return { defs, scope: { maxOf } };
  if (!Array.isArray(raw)) {
    issues.push(issueAt("structure", "counters must be an array of counters", "counters"));
    return { defs, scope: { maxOf } };
  }
  const items: readonly unknown[] = raw;
  for (const [index, item] of items.entries()) {
    const path = `counters[${index}]`;
    if (!isRecord(item)) {
      issues.push(issueAt("structure", "a counter must be an object", path));
      continue;
    }
    const id = readId(item.id, `${path}.id`, issues);
    const duplicate = id !== undefined && maxOf.has(id);
    if (duplicate) issues.push(issueAt("duplicate", `"${id}" is declared twice`, `${path}.id`));
    const max = item.max === undefined ? null : readWhole(item.max, `${path}.max`, 1, MAX_SAFE, issues);
    let persist: boolean | undefined;
    if (item.persist === undefined) {
      issues.push(issueAt("structure", `${path}.persist is required: true or false`, `${path}.persist`));
    } else {
      persist = readFlag(item.persist, `${path}.persist`, issues);
    }
    if (id !== undefined && !duplicate) maxOf.set(id, max ?? null);
    if (id !== undefined && !duplicate && max !== undefined && persist !== undefined) {
      defs.push({ id, max, persist });
    }
  }
  return { defs, scope: { maxOf } };
}

/** The ids of the achievements that the file declares, valid ones only. Used to check the references. */
function declaredIds(raw: unknown): Set<string> {
  const ids = new Set<string>();
  if (!Array.isArray(raw)) return ids;
  const items: readonly unknown[] = raw;
  for (const item of items) {
    if (!isRecord(item)) continue;
    const id = item.id;
    if (isId(id)) ids.add(id);
  }
  return ids;
}

/** Reads a counter goal: a declared counter and an `at_least` that the counter can reach. */
function readCounterGoal(raw: Record<string, unknown>, path: string, scope: CounterScope, issues: AchievementIssue[]): AchievementGoal | undefined {
  const counter = readId(raw.counter, `${path}.counter`, issues);
  let atLeast: number | undefined;
  if (raw.at_least === undefined) {
    issues.push(issueAt("structure", `${path}.at_least is required`, `${path}.at_least`));
  } else {
    atLeast = readWhole(raw.at_least, `${path}.at_least`, 1, MAX_SAFE, issues);
  }
  if (counter === undefined) return undefined;
  if (!scope.maxOf.has(counter)) {
    issues.push(issueAt("unknown-counter", `"${counter}" is not a counter of the file`, `${path}.counter`));
    return undefined;
  }
  if (atLeast === undefined) return undefined;
  const max = scope.maxOf.get(counter) ?? null;
  if (max !== null && atLeast > max) {
    issues.push(issueAt("value", `${atLeast} is above the counter's maximum of ${max}, so the goal could never be met`, `${path}.at_least`));
    return undefined;
  }
  return { kind: "counter", counter, at_least: atLeast };
}

/** Reads an `all` or an `any` goal: a list of distinct declared achievements, and for `any` a count. */
function readListGoal(
  kind: "all" | "any",
  raw: Record<string, unknown>,
  path: string,
  declared: ReadonlySet<string>,
  issues: AchievementIssue[],
): AchievementGoal | undefined {
  const listPath = `${path}.${kind}`;
  const list = raw[kind];
  if (!Array.isArray(list)) {
    issues.push(issueAt("structure", `${listPath} must be an array of achievement ids`, listPath));
    return undefined;
  }
  const items: readonly unknown[] = list;
  if (items.length === 0) {
    issues.push(issueAt("value", `${listPath} needs at least one achievement`, listPath));
    return undefined;
  }
  let count: number | undefined;
  if (kind === "any") {
    if (raw.count === undefined) {
      issues.push(issueAt("structure", `${path}.count is required: how many of the list must be unlocked`, `${path}.count`));
    } else {
      count = readWhole(raw.count, `${path}.count`, 1, items.length, issues);
    }
  }
  const of: string[] = [];
  let valid = true;
  for (const [index, item] of items.entries()) {
    const itemPath = `${listPath}[${index}]`;
    if (typeof item !== "string") {
      issues.push(issueAt("structure", `${itemPath} must be an achievement id`, itemPath));
      valid = false;
    } else if (!isId(item)) {
      issues.push(issueAt("bad-id", `"${item}" is not an id`, itemPath));
      valid = false;
    } else if (!declared.has(item)) {
      issues.push(issueAt("unknown-achievement", `"${item}" is not an achievement of the file`, itemPath));
      valid = false;
    } else if (of.includes(item)) {
      issues.push(issueAt("duplicate", `"${item}" is listed twice`, itemPath));
      valid = false;
    } else {
      of.push(item);
    }
  }
  if (!valid) return undefined;
  if (kind === "all") return { kind: "all", of };
  if (count === undefined) return undefined;
  return { kind: "any", of, count };
}

/** Reads a manual goal: the flag must be `true`. */
function readManualGoal(raw: Record<string, unknown>, path: string, issues: AchievementIssue[]): AchievementGoal | undefined {
  const manual = raw.manual;
  if (typeof manual !== "boolean") {
    issues.push(issueAt("structure", `${path}.manual must be true`, `${path}.manual`));
    return undefined;
  }
  if (!manual) {
    issues.push(issueAt("value", `${path}.manual must be true: the game unlocks a manual achievement`, `${path}.manual`));
    return undefined;
  }
  return { kind: "manual" };
}

/** Reads a goal. It has exactly one of the four kinds. */
function readGoal(
  raw: unknown,
  path: string,
  scope: CounterScope,
  declared: ReadonlySet<string>,
  issues: AchievementIssue[],
): AchievementGoal | undefined {
  if (raw === undefined) {
    issues.push(issueAt("structure", `${path} is required`, path));
    return undefined;
  }
  if (!isRecord(raw)) {
    issues.push(issueAt("structure", `${path} must be an object with one of counter, all, any or manual`, path));
    return undefined;
  }
  const goal: Record<string, unknown> = raw;
  const kinds = GOAL_KINDS.filter((kind) => goal[kind] !== undefined);
  if (kinds.length === 0) {
    issues.push(issueAt("structure", `${path} needs one of counter, all, any or manual`, path));
    return undefined;
  }
  if (kinds.length > 1) {
    issues.push(issueAt("value", `${path} has ${kinds.length} kinds (${kinds.join(", ")}); use one`, path));
    return undefined;
  }
  const kind: GoalKind | undefined = kinds[0];
  switch (kind) {
    case "counter":
      return readCounterGoal(goal, path, scope, issues);
    case "all":
    case "any":
      return readListGoal(kind, goal, path, declared, issues);
    case "manual":
      return readManualGoal(goal, path, issues);
    case undefined:
      return undefined;
  }
}

/** Reads the achievements, in the order of the file. */
function readAchievementList(
  raw: unknown,
  scope: CounterScope,
  declared: ReadonlySet<string>,
  issues: AchievementIssue[],
): AchievementDefinition[] {
  if (raw === undefined) return [];
  if (!Array.isArray(raw)) {
    issues.push(issueAt("structure", "achievements must be an array of achievements", "achievements"));
    return [];
  }
  const items: readonly unknown[] = raw;
  const seen = new Set<string>();
  const result: AchievementDefinition[] = [];
  for (const [index, item] of items.entries()) {
    const path = `achievements[${index}]`;
    if (!isRecord(item)) {
      issues.push(issueAt("structure", "an achievement must be an object", path));
      continue;
    }
    const id = readId(item.id, `${path}.id`, issues);
    const duplicate = id !== undefined && seen.has(id);
    if (duplicate) issues.push(issueAt("duplicate", `"${id}" is declared twice`, `${path}.id`));
    if (id !== undefined) seen.add(id);
    const name = readRequired(item.name, `${path}.name`, issues, readKey);
    const description = readRequired(item.description, `${path}.description`, issues, readKey);
    const icon = readRequired(item.icon, `${path}.icon`, issues, readArtId);
    const hidden = item.hidden === undefined ? false : readFlag(item.hidden, `${path}.hidden`, issues);
    const hint = item.hint === undefined ? null : readKey(item.hint, `${path}.hint`, issues);
    if (typeof hint === "string" && hidden === false) {
      issues.push(issueAt("value", "a hint needs hidden: true", `${path}.hint`));
    }
    const points = item.points === undefined ? 0 : readWhole(item.points, `${path}.points`, 0, MAX_POINTS, issues);
    const group = item.group === undefined ? DEFAULT_GROUP : readId(item.group, `${path}.group`, issues);
    const order = item.order === undefined ? 0 : readWhole(item.order, `${path}.order`, -MAX_ORDER, MAX_ORDER, issues);
    const steps = readSteps(item.progress_steps, `${path}.progress_steps`, issues);
    const goal = readGoal(item.goal, `${path}.goal`, scope, declared, issues);
    if (goal !== undefined && steps !== undefined && steps.length > 0) {
      if (goal.kind === "manual") {
        issues.push(issueAt("value", "a manual goal has no progress, so it takes no progress_steps", `${path}.progress_steps`));
      } else if (hidden === true) {
        issues.push(issueAt("value", "a hidden achievement sends no progress, so it takes no progress_steps", `${path}.progress_steps`));
      }
    }
    if (
      duplicate ||
      id === undefined ||
      name === undefined ||
      description === undefined ||
      icon === undefined ||
      hidden === undefined ||
      hint === undefined ||
      points === undefined ||
      group === undefined ||
      order === undefined ||
      steps === undefined ||
      goal === undefined
    ) {
      continue;
    }
    result.push({ id, name, description, icon, hidden, hint, points, group, order, progress_steps: steps, goal });
  }
  return result;
}

/**
 * Finds the loops of the `all` and `any` goals. Each loop is reported once, at the reference that closes it.
 * The search starts from the achievements in the order of the file, so the report is the same every time.
 */
function findCycles(raw: unknown, declared: ReadonlySet<string>): AchievementIssue[] {
  if (!Array.isArray(raw)) return [];
  const items: readonly unknown[] = raw;
  const edges = new Map<string, Edge[]>();
  for (const [index, item] of items.entries()) {
    if (!isRecord(item)) continue;
    const id = item.id;
    const goal = item.goal;
    if (!isId(id) || edges.has(id) || !isRecord(goal)) continue;
    const all = goal.all;
    const any = goal.any;
    let key: "all" | "any";
    let list: readonly unknown[];
    if (Array.isArray(all)) {
      key = "all";
      list = all;
    } else if (Array.isArray(any)) {
      key = "any";
      list = any;
    } else {
      continue;
    }
    const refs: Edge[] = [];
    for (const [position, ref] of list.entries()) {
      if (typeof ref === "string" && declared.has(ref)) {
        refs.push({ to: ref, path: `achievements[${index}].goal.${key}[${position}]` });
      }
    }
    edges.set(id, refs);
  }

  const issues: AchievementIssue[] = [];
  const state = new Map<string, "open" | "done">();
  const visit = (id: string): void => {
    state.set(id, "open");
    for (const edge of edges.get(id) ?? []) {
      const seen = state.get(edge.to);
      if (seen === "open") {
        const message =
          edge.to === id ? `"${id}" lists itself in its goal` : `the goals of "${id}" and "${edge.to}" lead back to each other`;
        issues.push(issueAt("cycle", message, edge.path));
      } else if (seen === undefined) {
        visit(edge.to);
      }
    }
    state.set(id, "done");
  };
  for (const id of edges.keys()) {
    if (!state.has(id)) visit(id);
  }
  return issues;
}

/**
 * Reads a definitions document that is already parsed (for example a JSON import). Every rule of the format is
 * checked.
 * @param value - the parsed document.
 * @returns the definitions in their normal form: defaults filled in, lists in the order of the file.
 * @throws AchievementError. Its `code` is the code of the first problem; its `issues` lists every problem.
 */
export function readAchievements(value: unknown): AchievementCatalog {
  if (!isRecord(value)) {
    throw new AchievementError("structure", "the definitions must be a JSON object", [
      { code: "structure", message: "the definitions must be a JSON object" },
    ]);
  }
  if (value.format !== ACHIEVEMENTS_FORMAT) {
    throw new AchievementError("format", `the format tag is not "${ACHIEVEMENTS_FORMAT}"`, [
      issueAt("format", `the format tag is not "${ACHIEVEMENTS_FORMAT}"`, "format"),
    ]);
  }
  if (value.kind !== "definitions") {
    throw new AchievementError("format", 'the kind is not "definitions"', [issueAt("format", 'the kind is not "definitions"', "kind")]);
  }
  if (value.version !== ACHIEVEMENTS_VERSION) {
    throw new AchievementError("version", `the version is not ${ACHIEVEMENTS_VERSION}`, [
      issueAt("version", `the version is not ${ACHIEVEMENTS_VERSION}`, "version"),
    ]);
  }
  const issues: AchievementIssue[] = [];
  const { defs: counters, scope } = readCounters(value.counters, issues);
  const declared = declaredIds(value.achievements);
  const achievements = readAchievementList(value.achievements, scope, declared, issues);
  issues.push(...findCycles(value.achievements, declared));
  if (issues.length > 0) throw refusal(issues);
  return { format: ACHIEVEMENTS_FORMAT, version: ACHIEVEMENTS_VERSION, kind: "definitions", counters, achievements };
}

/**
 * Reads a definitions document from its JSON text, strictly.
 * @param text - the text of the definitions file.
 * @throws AchievementError. Its `code` is "json" for text that is not JSON, otherwise the code of the first problem.
 */
export function parseAchievements(text: string): AchievementCatalog {
  let parsed: unknown;
  try {
    parsed = JSON.parse(text);
  } catch (cause) {
    throw new AchievementError("json", "the achievements text is not valid JSON", [{ code: "json", message: "the text is not valid JSON" }], {
      cause,
    });
  }
  return readAchievements(parsed);
}
