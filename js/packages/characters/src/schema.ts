// The stat schema: the definitions that every character of a game shares (kind "schema" in
// docs/formats/CHARACTER.md). parseStatSchema checks every rule and returns the normal form.
// indexSchema builds the lookups that the runtime needs, once per schema.

import { CharacterError } from "./error";
import { expressionVariables, parseExpression, type Expression } from "./expression";
import {
  checkUnique,
  invalid,
  optionalList,
  parseJson,
  readEnvelope,
  readId,
  readInteger,
  readKey,
  readList,
  readNumber,
  readOneOf,
  readRecord,
} from "./read";
import {
  CHARACTER_FORMAT,
  CHARACTER_VERSION,
  type BaseStatDef,
  type DerivedStatDef,
  type LevelDef,
  type NeedDef,
  type ProfessionDef,
  type RelationshipRules,
  type StatDef,
  type StatSchema,
  type ThresholdDef,
  type TraitDef,
  type TraitModifier,
} from "./types";
import { indexById } from "./util";

/** Stat ids that formulas use as function names, so that a stat cannot take them. */
const RESERVED_STAT_IDS: readonly string[] = ["min", "max", "clamp", "floor"];

/**
 * Reads a stat schema document, checks every rule, and returns it in its normal form: every default
 * written out, and every list in the order the document gives.
 *
 * @param json - the text of the schema file.
 * @returns the checked schema.
 * @throws {CharacterError} with the code of the rule that broke: "json", "unsupported-document",
 *   "invalid", "bad-id", "duplicate-id", "expression", "unknown-function", "arity", "unknown-stat",
 *   "unknown-need", "division-by-zero" (a literal zero divisor), or "cycle" (derived stats in a loop).
 *
 * @example
 * const schema = parseStatSchema(await readText("schema.json"));
 */
export function parseStatSchema(json: string): StatSchema {
  const doc = readEnvelope(parseJson(json), "schema");
  const stats = readStats(optionalList(doc, "schema", "stats"));
  const needs = readNeeds(optionalList(doc, "schema", "needs"));
  const professions = readProfessions(optionalList(doc, "schema", "professions"));
  const relationships = readRelationships(doc["relationships"]);
  const traits = readTraits(optionalList(doc, "schema", "traits"), indexById(stats), indexById(needs));
  checkFormulas(stats);
  return {
    format: CHARACTER_FORMAT,
    version: CHARACTER_VERSION,
    kind: "schema",
    stats,
    needs,
    professions,
    relationships,
    traits,
  };
}

/** Reads the minimum and maximum of a stat or a need, and checks that the minimum is not above the maximum. */
function readBounds(record: Record<string, unknown>, path: string): { min: number; max: number } {
  const min = readNumber(record["min"], `${path}.min`);
  const max = readNumber(record["max"], `${path}.max`);
  if (min > max) throw invalid(`${path}: min is above max`);
  return { min, max };
}

/** Reads a finite number that must lie from `min` to `max`. */
function readInRange(value: unknown, path: string, min: number, max: number): number {
  const amount = readNumber(value, path);
  if (amount < min || amount > max) throw invalid(`${path} must be from ${min} to ${max}`);
  return amount;
}

/** Reads a stat id. The names that formulas use as functions are reserved. */
function readStatId(value: unknown, path: string): string {
  const id = readId(value, path);
  if (RESERVED_STAT_IDS.includes(id)) {
    throw new CharacterError("bad-id", `${path}: "${id}" is a function name of formulas and cannot be a stat id`);
  }
  return id;
}

/** Reads the base and derived stats. A base stat has a default; a derived stat has a formula and no default. */
function readStats(list: readonly unknown[]): StatDef[] {
  const seen = new Set<string>();
  return list.map((item, index): StatDef => {
    const path = `stats[${index}]`;
    const record = readRecord(item, path);
    const id = readStatId(record["id"], `${path}.id`);
    checkUnique(seen, id, "stats");
    const label = readKey(record["label"], `${path}.label`);
    const { min, max } = readBounds(record, path);
    if (record["formula"] !== undefined) {
      if (record["default"] !== undefined) {
        throw invalid(`${path}: a derived stat has a formula, so it has no default`);
      }
      const formula = record["formula"];
      if (typeof formula !== "string" || formula.trim() === "") {
        throw invalid(`${path}.formula must be a formula: text that is not empty`);
      }
      const derived: DerivedStatDef = { id, label, min, max, formula };
      return derived;
    }
    const start = readInRange(record["default"], `${path}.default`, min, max);
    const base: BaseStatDef = { id, label, min, max, default: start };
    return base;
  });
}

/** Reads the needs: each with a range, a default, a rate per game second, and its thresholds. */
function readNeeds(list: readonly unknown[]): NeedDef[] {
  const seen = new Set<string>();
  return list.map((item, index): NeedDef => {
    const path = `needs[${index}]`;
    const record = readRecord(item, path);
    const id = readId(record["id"], `${path}.id`);
    checkUnique(seen, id, "needs");
    const label = readKey(record["label"], `${path}.label`);
    const { min, max } = readBounds(record, path);
    const start = readInRange(record["default"], `${path}.default`, min, max);
    const rate = record["rate"] === undefined ? 0 : readNumber(record["rate"], `${path}.rate`);
    const thresholds = readThresholds(optionalList(record, path, "thresholds"), `${path}.thresholds`, min, max);
    return { id, label, min, max, default: start, rate, thresholds };
  });
}

/** Reads the thresholds of one need. Their ids are unique within the need. */
function readThresholds(list: readonly unknown[], path: string, min: number, max: number): ThresholdDef[] {
  const seen = new Set<string>();
  return list.map((item, index): ThresholdDef => {
    const where = `${path}[${index}]`;
    const record = readRecord(item, where);
    const id = readId(record["id"], `${where}.id`);
    checkUnique(seen, id, path);
    const at = readInRange(record["at"], `${where}.at`, min, max);
    const when = readOneOf(record["when"], `${where}.when`, ["below", "above"] as const);
    return { id, at, when };
  });
}

/**
 * Reads the professions and their experience curves. A curve starts at 0 and rises strictly.
 */
function readProfessions(list: readonly unknown[]): ProfessionDef[] {
  const seen = new Set<string>();
  return list.map((item, index): ProfessionDef => {
    const path = `professions[${index}]`;
    const record = readRecord(item, path);
    const id = readId(record["id"], `${path}.id`);
    checkUnique(seen, id, "professions");
    const label = readKey(record["label"], `${path}.label`);
    return { id, label, xp_thresholds: readXpThresholds(record["xp_thresholds"], `${path}.xp_thresholds`) };
  });
}

/** Reads one experience curve: the total experience at which each level starts. */
function readXpThresholds(value: unknown, path: string): number[] {
  const list = readList(value, path);
  if (list.length === 0) throw invalid(`${path} must list at least one level`);
  const thresholds = list.map((item, index) => readNumber(item, `${path}[${index}]`));
  if (thresholds[0] !== 0) throw invalid(`${path}: the first level starts at 0 experience`);
  for (let index = 1; index < thresholds.length; index += 1) {
    if ((thresholds[index] ?? 0) <= (thresholds[index - 1] ?? 0)) {
      throw invalid(`${path}: each level must start at more experience than the one before`);
    }
  }
  return thresholds;
}

/** Reads the affinity rules. Without a `relationships` block, the defaults of docs/formats/CHARACTER.md apply. */
function readRelationships(value: unknown): RelationshipRules {
  if (value === undefined) return { min: -100, max: 100, default: 0, history_limit: 50, levels: [] };
  const record = readRecord(value, "relationships");
  const min = record["min"] === undefined ? -100 : readNumber(record["min"], "relationships.min");
  const max = record["max"] === undefined ? 100 : readNumber(record["max"], "relationships.max");
  if (min > max) throw invalid("relationships: min is above max");
  const defaultValue =
    record["default"] === undefined ? 0 : readInRange(record["default"], "relationships.default", min, max);
  const historyLimit =
    record["history_limit"] === undefined
      ? 50
      : readInteger(record["history_limit"], "relationships.history_limit", 1, 1000);
  const levels = readLevels(optionalList(record, "relationships", "levels"), min, max);
  return { min, max, default: defaultValue, history_limit: historyLimit, levels };
}

/**
 * Reads the named levels of affinity. They rise strictly, the first starts at the lowest affinity, and
 * every one lies within the range.
 */
function readLevels(list: readonly unknown[], min: number, max: number): LevelDef[] {
  const seen = new Set<string>();
  const levels = list.map((item, index): LevelDef => {
    const path = `relationships.levels[${index}]`;
    const record = readRecord(item, path);
    const id = readId(record["id"], `${path}.id`);
    checkUnique(seen, id, "relationships.levels");
    const label = readKey(record["label"], `${path}.label`);
    const at = readInRange(record["at"], `${path}.at`, min, max);
    return { id, label, at };
  });
  const [first] = levels;
  if (first !== undefined && first.at !== min) {
    throw invalid("relationships.levels: the first level must start at the lowest affinity (min)");
  }
  for (let index = 1; index < levels.length; index += 1) {
    if ((levels[index]?.at ?? 0) <= (levels[index - 1]?.at ?? 0)) {
      throw invalid("relationships.levels: each level must start higher than the one before");
    }
  }
  return levels;
}

/** Reads the traits: each names the stats and the need rates that it changes. */
function readTraits(
  list: readonly unknown[],
  stats: ReadonlyMap<string, StatDef>,
  needs: ReadonlyMap<string, NeedDef>,
): TraitDef[] {
  const seen = new Set<string>();
  return list.map((item, index): TraitDef => {
    const path = `traits[${index}]`;
    const record = readRecord(item, path);
    const id = readId(record["id"], `${path}.id`);
    checkUnique(seen, id, "traits");
    const label = readKey(record["label"], `${path}.label`);
    const modifiers = optionalList(record, path, "modifiers").map((modifier, position) =>
      readModifier(modifier, `${path}.modifiers[${position}]`, stats, needs),
    );
    return { id, label, modifiers };
  });
}

/**
 * Reads one trait modifier: it names one stat or one need rate, and has one operation (`add` or `mul`).
 * `mul` may not be negative.
 */
function readModifier(
  item: unknown,
  path: string,
  stats: ReadonlyMap<string, StatDef>,
  needs: ReadonlyMap<string, NeedDef>,
): TraitModifier {
  const record = readRecord(item, path);
  const hasStat = record["stat"] !== undefined;
  const hasRate = record["rate"] !== undefined;
  if (hasStat === hasRate) throw invalid(`${path} must name one stat or one rate`);
  const hasAdd = record["add"] !== undefined;
  const hasMul = record["mul"] !== undefined;
  if (hasAdd === hasMul) throw invalid(`${path} must have one of "add" or "mul"`);

  let target: "stat" | "rate";
  let id: string;
  if (hasStat) {
    target = "stat";
    id = readId(record["stat"], `${path}.stat`);
    if (!stats.has(id)) throw new CharacterError("unknown-stat", `${path}.stat names no stat of the schema`);
  } else {
    target = "rate";
    id = readId(record["rate"], `${path}.rate`);
    if (!needs.has(id)) throw new CharacterError("unknown-need", `${path}.rate names no need of the schema`);
  }

  if (hasAdd) return { target, id, op: "add", value: readNumber(record["add"], `${path}.add`) };
  const factor = readNumber(record["mul"], `${path}.mul`);
  if (factor < 0) throw invalid(`${path}.mul must be 0 or more`);
  return { target, id, op: "mul", value: factor };
}

/**
 * Parses a formula and returns its tree. A syntax error keeps its code, and its message names the path.
 * @throws {CharacterError} with the code of the error that parsing raised.
 */
function readFormula(source: string, path: string): Expression {
  try {
    return parseExpression(source);
  } catch (error) {
    if (error instanceof CharacterError) {
      throw new CharacterError(error.code, `${path}: ${error.message}`, { cause: error });
    }
    throw error;
  }
}

/**
 * Checks every formula of the schema: its syntax, its functions and their argument counts, and the
 * stats it reads (which must exist). Then checks that no derived stat depends on itself.
 */
function checkFormulas(stats: readonly StatDef[]): void {
  const known = new Set(stats.map((stat) => stat.id));
  const dependencies = new Map<string, readonly string[]>();
  stats.forEach((stat, index) => {
    if (stat.formula === undefined) return;
    const path = `stats[${index}].formula`;
    const variables = expressionVariables(readFormula(stat.formula, path));
    for (const name of variables) {
      if (!known.has(name)) {
        throw new CharacterError("unknown-stat", `${path} reads "${name}", which is not a stat of the schema`);
      }
    }
    dependencies.set(stat.id, variables);
  });
  checkNoLoops(dependencies);
}

/**
 * Checks that the derived stats form no loop: each one's formula reads only stats that do not lead
 * back to it. Base stats read nothing, so they never close a loop.
 * @throws {CharacterError} with code "cycle", naming the loop.
 */
function checkNoLoops(dependencies: ReadonlyMap<string, readonly string[]>): void {
  const state = new Map<string, "visiting" | "done">();
  const visit = (id: string, trail: readonly string[]): void => {
    const mark = state.get(id);
    if (mark === "done") return;
    if (mark === "visiting") {
      throw new CharacterError("cycle", `derived stats depend on each other in a loop: ${[...trail, id].join(" -> ")}`);
    }
    state.set(id, "visiting");
    for (const dependency of dependencies.get(id) ?? []) visit(dependency, [...trail, id]);
    state.set(id, "done");
  };
  for (const id of dependencies.keys()) visit(id, []);
}

/** The lookups that the runtime uses, built once from a schema. */
export interface SchemaIndex {
  /** The schema itself. */
  readonly schema: StatSchema;
  /** The stats by id, in schema order. */
  readonly stats: ReadonlyMap<string, StatDef>;
  /** The needs by id, in schema order. */
  readonly needs: ReadonlyMap<string, NeedDef>;
  /** The professions by id, in schema order. */
  readonly professions: ReadonlyMap<string, ProfessionDef>;
  /** The traits by id, in schema order. */
  readonly traits: ReadonlyMap<string, TraitDef>;
  /** The parsed formula of each derived stat. */
  readonly formulas: ReadonlyMap<string, Expression>;
}

/** The index of each schema object that has been seen. */
const indexes = new WeakMap<StatSchema, SchemaIndex>();

/**
 * Builds the lookups of a schema, or returns the ones built before for the same object.
 * @throws {CharacterError} when a formula of a schema that was not made by parseStatSchema does not parse.
 */
export function indexSchema(schema: StatSchema): SchemaIndex {
  const known = indexes.get(schema);
  if (known !== undefined) return known;
  const formulas = new Map<string, Expression>();
  for (const stat of schema.stats) {
    if (stat.formula !== undefined) formulas.set(stat.id, parseExpression(stat.formula));
  }
  const index: SchemaIndex = {
    schema,
    stats: indexById(schema.stats),
    needs: indexById(schema.needs),
    professions: indexById(schema.professions),
    traits: indexById(schema.traits),
    formulas,
  };
  indexes.set(schema, index);
  return index;
}

/**
 * The position of the level that a value falls in: the highest level whose `at` is at or below the
 * value. Returns -1 when the value is under the first level, or when there are no levels.
 */
export function levelPosition(levels: readonly LevelDef[], value: number): number {
  let position = -1;
  levels.forEach((level, index) => {
    if (value >= level.at) position = index;
  });
  return position;
}

/** The level that a value falls in, or null when the rules have no levels. */
export function levelOf(rules: RelationshipRules, value: number): LevelDef | null {
  const position = levelPosition(rules.levels, value);
  return position < 0 ? null : (rules.levels[position] ?? null);
}
