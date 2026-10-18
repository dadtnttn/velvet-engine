// The runtime character: its stats, needs, professions, traits and affinity toward other characters,
// with the events that its changes raise, and the state that a save keeps. The rules are in
// docs/formats/CHARACTER.md.
//
// Every value is kept in `Values`, which is exactly what a save holds. The definition gives the
// starting values, and it is never changed.

import { CharacterError } from "./error";
import { evaluateExpression } from "./expression";
import { completeCharacter } from "./definition";
import { MAX_KEY_LENGTH, invalid, isId, ownValue, readId, readKey, readList, readNumber, readRecord } from "./read";
import { indexSchema, levelOf, levelPosition, type SchemaIndex } from "./schema";
import { assertFinite, clamp, compareText, requireEntry } from "./util";
import {
  CHARACTER_FORMAT,
  CHARACTER_VERSION,
  type AffinityChange,
  type BaseStatDef,
  type CharacterArt,
  type CharacterDefinition,
  type CharacterEvent,
  type CharacterState,
  type NeedDef,
  type ProfessionDef,
  type Relation,
  type RelationshipRules,
  type StatDef,
  type StatSchema,
  type ThresholdDef,
  type TraitDef,
} from "./types";

/** The runtime values of one character: exactly what a save keeps. */
interface Values {
  /** The base value of every base stat. */
  readonly stats: Map<string, number>;
  /** The current value of every need. */
  readonly needs: Map<string, number>;
  /** The experience of every profession. */
  readonly xp: Map<string, number>;
  /** The traits the character has. */
  readonly traits: Set<string>;
  /** The affinity toward each other character that holds a value. Other pairs are at the default. */
  readonly affinity: Map<string, number>;
  /** The affinity changes, oldest first, at most `history_limit` of them. */
  readonly history: AffinityChange[];
}

/** The state of a character's values at the start, from its definition. */
function startValues(start: CharacterDefinition): Values {
  return {
    stats: new Map(Object.entries(start.stats)),
    needs: new Map(Object.entries(start.needs)),
    xp: new Map(Object.entries(start.professions)),
    traits: new Set(start.traits),
    affinity: new Map(Object.entries(start.relationships)),
    history: [],
  };
}

/** The number under `key` of a record of numbers, or undefined when the record has no such key of its own. */
function ownNumber(record: Readonly<Record<string, number>>, key: string): number | undefined {
  return Object.hasOwn(record, key) ? record[key] : undefined;
}

/** The level of a profession for a total of experience: the number of thresholds that it has reached. */
function levelOfExperience(profession: ProfessionDef, xp: number): number {
  return profession.xp_thresholds.filter((threshold) => xp >= threshold).length;
}

/** True when a move of a need from `before` to `after` crosses the threshold in the direction it names. */
function crosses(threshold: ThresholdDef, before: number, after: number): boolean {
  if (threshold.when === "below") return before >= threshold.at && after < threshold.at;
  return before < threshold.at && after >= threshold.at;
}

/**
 * The migrations of states, keyed by the version they start from. Version 1 is the first version, so
 * there are none yet. A version 2 adds `STATE_MIGRATIONS[1]`, which turns version 1 states into version 2.
 */
const STATE_MIGRATIONS: Readonly<Record<number, (state: Record<string, unknown>) => Record<string, unknown>>> = {};

/** Brings a state up to the current version, one step at a time. */
function migrateState(state: Record<string, unknown>, from: number): Record<string, unknown> {
  let current = state;
  for (let version = from; version < CHARACTER_VERSION; version += 1) {
    const migrate = STATE_MIGRATIONS[version];
    if (migrate === undefined) {
      throw new CharacterError("unsupported-document", `no migration from state version ${version} to ${version + 1}`);
    }
    current = migrate(current);
  }
  return current;
}

/**
 * Reads the envelope of a state. A newer version is refused; an older one is migrated.
 * @throws {CharacterError} with code "unsupported-document" for another format or a newer version, and
 *   "invalid" for a version that is not a whole number from 1.
 */
function readStateEnvelope(value: unknown): Record<string, unknown> {
  const doc = readRecord(value, "the state");
  if (doc["format"] !== CHARACTER_FORMAT || doc["kind"] !== "state") {
    throw new CharacterError("unsupported-document", `expected a ${CHARACTER_FORMAT} state`);
  }
  const version = doc["version"];
  if (typeof version !== "number" || !Number.isInteger(version) || version < 1) {
    throw invalid("the state version must be a whole number, 1 or more");
  }
  if (version > CHARACTER_VERSION) {
    throw new CharacterError(
      "unsupported-document",
      `the state was written by a newer build (version ${version}); this one reads up to version ${CHARACTER_VERSION}`,
    );
  }
  return migrateState(doc, version);
}

/** Reads one entry of an affinity history. */
function readHistoryEntry(item: unknown, position: number, rules: RelationshipRules): AffinityChange {
  const path = `state.history[${position}]`;
  const record = readRecord(item, path);
  return {
    other: readId(record["other"], `${path}.other`),
    delta: readNumber(record["delta"], `${path}.delta`),
    value: clamp(readNumber(record["value"], `${path}.value`), rules.min, rules.max),
    reason: readKey(record["reason"], `${path}.reason`),
  };
}

/**
 * Reads a state into values, for the character `id` of `index`. The rules: the id must match; every
 * number is clamped to its range; a missing stat, need or profession takes its starting value; a trait
 * or stat that the schema does not have is dropped (the schema changed); a history longer than the
 * limit keeps its newest entries.
 * @throws {CharacterError} with code "unsupported-document", "invalid" or "bad-id".
 */
function readState(index: SchemaIndex, id: string, start: CharacterDefinition, value: unknown): Values {
  const doc = readStateEnvelope(value);
  if (doc["id"] !== id) throw invalid(`the state is for "${String(doc["id"])}", not for "${id}"`);
  const rules = index.schema.relationships;

  const statFields = readRecord(doc["stats"], "state.stats");
  const stats = new Map<string, number>();
  for (const def of index.stats.values()) {
    if (def.formula !== undefined) continue;
    const raw = ownValue(statFields, def.id);
    const fallback = ownNumber(start.stats, def.id) ?? def.default;
    const amount = raw === undefined ? fallback : readNumber(raw, `state.stats.${def.id}`);
    stats.set(def.id, clamp(amount, def.min, def.max));
  }

  const needFields = readRecord(doc["needs"], "state.needs");
  const needs = new Map<string, number>();
  for (const def of index.needs.values()) {
    const raw = ownValue(needFields, def.id);
    const fallback = ownNumber(start.needs, def.id) ?? def.default;
    const amount = raw === undefined ? fallback : readNumber(raw, `state.needs.${def.id}`);
    needs.set(def.id, clamp(amount, def.min, def.max));
  }

  const xpFields = readRecord(doc["xp"], "state.xp");
  const xp = new Map<string, number>();
  for (const def of index.professions.values()) {
    const raw = ownValue(xpFields, def.id);
    const fallback = ownNumber(start.professions, def.id) ?? 0;
    const amount = raw === undefined ? fallback : readNumber(raw, `state.xp.${def.id}`);
    xp.set(def.id, clamp(amount, 0, Math.max(...def.xp_thresholds)));
  }

  const traits = new Set<string>();
  readList(doc["traits"], "state.traits").forEach((item, position) => {
    if (typeof item !== "string") throw invalid(`state.traits[${position}] must be text`);
    if (index.traits.has(item)) traits.add(item);
  });

  const affinity = new Map<string, number>();
  for (const [other, raw] of Object.entries(readRecord(doc["affinity"], "state.affinity"))) {
    if (!isId(other)) throw new CharacterError("bad-id", `state.affinity: "${other}" is not a character id`);
    if (other === id) throw invalid("state.affinity: a character has no affinity with itself");
    affinity.set(other, clamp(readNumber(raw, `state.affinity.${other}`), rules.min, rules.max));
  }

  const entries = readList(doc["history"], "state.history").map((item, position) =>
    readHistoryEntry(item, position, rules),
  );
  return { stats, needs, xp, traits, affinity, history: entries.slice(-rules.history_limit) };
}

/** Writes values as a state. Lists are in schema order, and sorted where the schema has no order. */
function toState(index: SchemaIndex, id: string, values: Values): CharacterState {
  const stats: Record<string, number> = {};
  for (const def of index.stats.values()) {
    if (def.formula === undefined) stats[def.id] = requireEntry(values.stats, def.id);
  }
  const needs: Record<string, number> = {};
  for (const def of index.needs.values()) needs[def.id] = requireEntry(values.needs, def.id);
  const xp: Record<string, number> = {};
  for (const def of index.professions.values()) xp[def.id] = requireEntry(values.xp, def.id);
  const affinity: Record<string, number> = {};
  for (const [other, value] of [...values.affinity.entries()].sort(([a], [b]) => compareText(a, b))) {
    affinity[other] = value;
  }
  return {
    format: CHARACTER_FORMAT,
    version: CHARACTER_VERSION,
    kind: "state",
    id,
    stats,
    needs,
    xp,
    traits: [...values.traits].sort(compareText),
    affinity,
    history: values.history.map((entry) => ({ ...entry })),
  };
}

/**
 * A character at runtime. It holds the values of its definition, changes them through its methods, and
 * reports what the changes did as events. Its state goes in and out with `snapshot` and `restore`.
 *
 * @example
 * const character = new Character(schema, parseCharacter(json, schema));
 * const events = character.tickNeeds(160); // game seconds
 */
/** Options of a Character. */
export interface CharacterOptions {
  /**
   * True: a formula that divides by zero while it is evaluated throws (for tools and tests). False,
   * the default: that stat takes its minimum and onFormulaError is told once per stat, so one bad
   * divisor cannot break the frame that reads the stats.
   */
  readonly strict?: boolean;
  /** Told once per stat when a formula fails and the character is not strict. Defaults to console.warn. */
  readonly onFormulaError?: (statId: string, error: CharacterError) => void;
}

export class Character {
  /** The id: unique in a roster. */
  readonly id: string;
  /** An i18n key for the name. */
  readonly name: string;
  /** An i18n key for the title, when the character has one. */
  readonly title: string | undefined;
  /** The art ids. */
  readonly art: CharacterArt;
  /** The tags. */
  readonly tags: readonly string[];
  /** The schema that the character follows. */
  readonly schema: StatSchema;

  private readonly index: SchemaIndex;
  private readonly start: CharacterDefinition;
  private values: Values;
  private readonly strict: boolean;
  private readonly onFormulaError: (statId: string, error: CharacterError) => void;
  private readonly reported = new Set<string>();

  /**
   * Makes a character at its starting values.
   *
   * @param schema - the schema of the game (from parseStatSchema).
   * @param definition - the character's definition (from parseCharacter). It is checked again here.
   * @param options - strict formula errors, and where a lenient character reports them.
   * @throws {CharacterError} when the definition names an entry that the schema does not have, or a
   *   value outside its range.
   */
  constructor(schema: StatSchema, definition: CharacterDefinition, options: CharacterOptions = {}) {
    this.strict = options.strict ?? false;
    this.onFormulaError =
      options.onFormulaError ?? ((statId, error) => console.warn(`@velvet/characters: stat "${statId}": ${error.message}`));
    this.index = indexSchema(schema);
    this.start = completeCharacter(this.index, { ...definition });
    this.schema = schema;
    this.id = this.start.id;
    this.name = this.start.name;
    this.title = this.start.title;
    this.art = this.start.art;
    this.tags = this.start.tags;
    this.values = startValues(this.start);
  }

  /**
   * The effective value of a stat: a base stat's base value, or a derived stat's formula, then the
   * modifiers of the traits, then the clamp to the stat's range.
   * @throws {CharacterError} with code "unknown-stat"; also "division-by-zero" when a formula divides by
   *   zero and the character is strict (a lenient one uses the stat's minimum and reports it once).
   */
  stat(id: string): number {
    const def = this.statDef(id);
    const formula = this.index.formulas.get(id);
    const raw = formula === undefined ? requireEntry(this.values.stats, id) : this.evaluate(id, formula, def.min);
    const { add, mul } = this.modifiers("stat", id);
    return clamp((raw + add) * mul, def.min, def.max);
  }

  /** A derived stat's formula, or its minimum when the formula fails and the character is lenient. */
  private evaluate(id: string, formula: Parameters<typeof evaluateExpression>[0], fallback: number): number {
    try {
      return evaluateExpression(formula, (name) => this.stat(name));
    } catch (error) {
      if (this.strict || !(error instanceof CharacterError) || error.code !== "division-by-zero") throw error;
      if (!this.reported.has(id)) {
        this.reported.add(id);
        this.onFormulaError(id, error);
      }
      return fallback;
    }
  }

  /** The effective value of every stat, in schema order. */
  stats(): Record<string, number> {
    const result: Record<string, number> = {};
    for (const def of this.index.stats.values()) result[def.id] = this.stat(def.id);
    return result;
  }

  /**
   * The base value of a base stat, without the traits and the clamp.
   * @throws {CharacterError} with code "unknown-stat", or "derived-stat" for a stat with a formula.
   */
  baseStat(id: string): number {
    this.baseDef(id);
    return requireEntry(this.values.stats, id);
  }

  /**
   * Adds `delta` to the base value of a base stat, clamped to the stat's range.
   * @returns the new base value.
   * @throws {CharacterError} with code "unknown-stat", "derived-stat", or "argument" when `delta` is not finite.
   */
  changeBase(id: string, delta: number): number {
    const def = this.baseDef(id);
    assertFinite(delta, "delta");
    const after = clamp(requireEntry(this.values.stats, id) + delta, def.min, def.max);
    this.values.stats.set(id, after);
    return after;
  }

  /** The traits the character has, sorted by id. */
  get traits(): readonly string[] {
    return [...this.values.traits].sort(compareText);
  }

  /** True when the character has the trait. */
  hasTrait(id: string): boolean {
    return this.values.traits.has(id);
  }

  /**
   * Gives the character a trait. Having it already changes nothing.
   * @throws {CharacterError} with code "unknown-trait" when the schema has no such trait.
   */
  addTrait(id: string): void {
    this.traitDef(id);
    this.values.traits.add(id);
  }

  /**
   * Takes a trait away. Not having it changes nothing.
   * @throws {CharacterError} with code "unknown-trait" when the schema has no such trait.
   */
  removeTrait(id: string): void {
    this.traitDef(id);
    this.values.traits.delete(id);
  }

  /** The current value of a need. @throws {CharacterError} with code "unknown-need". */
  need(id: string): number {
    this.needDef(id);
    return requireEntry(this.values.needs, id);
  }

  /**
   * The change per game second of a need: its rate, with the modifiers of the traits.
   * @throws {CharacterError} with code "unknown-need".
   */
  rate(id: string): number {
    const def = this.needDef(id);
    const { add, mul } = this.modifiers("rate", id);
    return (def.rate + add) * mul;
  }

  /**
   * Moves every need by its rate times the time that passed. A need whose rate is zero does not move.
   * Each need is clamped to its range.
   *
   * @param dtGameSeconds - the game time that passed, in game seconds: a finite number, 0 or more.
   * @returns the thresholds that the move crossed, in schema order (one per threshold at most).
   * @throws {CharacterError} with code "argument" when `dtGameSeconds` is negative or not finite.
   */
  tickNeeds(dtGameSeconds: number): CharacterEvent[] {
    if (!Number.isFinite(dtGameSeconds) || dtGameSeconds < 0) {
      throw new CharacterError("argument", "the time step must be a finite number of game seconds, 0 or more");
    }
    const events: CharacterEvent[] = [];
    for (const def of this.index.needs.values()) {
      const rate = this.rate(def.id);
      if (rate === 0) continue;
      const before = requireEntry(this.values.needs, def.id);
      const after = clamp(before + rate * dtGameSeconds, def.min, def.max);
      this.values.needs.set(def.id, after);
      events.push(...this.crossings(def, before, after));
    }
    return events;
  }

  /**
   * Adds `delta` to a need (for example, a meal adds to hunger), clamped to its range. The thresholds
   * that the change crosses raise their events, as in tickNeeds.
   *
   * @returns the thresholds that the change crossed.
   * @throws {CharacterError} with code "unknown-need" or "argument" when `delta` is not finite.
   */
  changeNeed(id: string, delta: number): CharacterEvent[] {
    const def = this.needDef(id);
    assertFinite(delta, "delta");
    const before = requireEntry(this.values.needs, id);
    const after = clamp(before + delta, def.min, def.max);
    this.values.needs.set(id, after);
    return this.crossings(def, before, after);
  }

  /** The experience of a profession. @throws {CharacterError} with code "unknown-profession". */
  xp(profession: string): number {
    this.professionDef(profession);
    return requireEntry(this.values.xp, profession);
  }

  /** The level of a profession: 1 at the start, one more for each threshold of its curve reached. */
  level(profession: string): number {
    return levelOfExperience(this.professionDef(profession), this.xp(profession));
  }

  /**
   * Adds experience to a profession. Experience stops at the last threshold of the curve, so an excess
   * is lost.
   *
   * @param profession - the id of a profession of the schema.
   * @param amount - the experience gained: a finite number, 0 or more.
   * @returns one `level_up` event for each level reached, in order.
   * @throws {CharacterError} with code "unknown-profession" or "argument".
   */
  gainXp(profession: string, amount: number): CharacterEvent[] {
    const def = this.professionDef(profession);
    if (!Number.isFinite(amount) || amount < 0) {
      throw new CharacterError("argument", "the experience gained must be a finite number, 0 or more");
    }
    const before = requireEntry(this.values.xp, profession);
    const after = Math.min(before + amount, Math.max(...def.xp_thresholds));
    this.values.xp.set(profession, after);
    const events: CharacterEvent[] = [];
    for (let level = levelOfExperience(def, before) + 1; level <= levelOfExperience(def, after); level += 1) {
      events.push({ event: "level_up", character: this.id, profession, level });
    }
    return events;
  }

  /**
   * The affinity of this character toward another. A pair that has no value yet is at the default.
   * @throws {CharacterError} with code "bad-id", or "argument" for the character itself.
   */
  affinity(other: string | Character): number {
    return this.values.affinity.get(this.otherId(other)) ?? this.schema.relationships.default;
  }

  /** The name of the level that the affinity toward `other` is on, or null when the schema has no levels. */
  affinityLevel(other: string | Character): string | null {
    return levelOf(this.schema.relationships, this.affinity(other))?.id ?? null;
  }

  /** The pairs that hold an affinity value, sorted by the other character's id. */
  affinities(): Relation[] {
    const rules = this.schema.relationships;
    return [...this.values.affinity.entries()]
      .sort(([a], [b]) => compareText(a, b))
      .map(([other, value]) => ({ other, value, level: levelOf(rules, value)?.id ?? null }));
  }

  /** The affinity changes, oldest first, at most `history_limit` of them. */
  get history(): readonly AffinityChange[] {
    return this.values.history.map((entry) => ({ ...entry }));
  }

  /**
   * Moves the affinity toward `other` by `delta`, clamped to the range. A change that moves nothing
   * (because the value is at the limit already) changes nothing, and is not recorded in the history.
   *
   * @param other - the other character, or its id.
   * @param delta - the change: a finite number.
   * @param reason - an i18n key or an id that says why the change happened, kept in the history.
   * @returns an `affinity_level` event when the value moved to another level.
   * @throws {CharacterError} with code "bad-id", or "argument" for a character's own id, a change that is
   *   not finite, or a reason that is empty or too long.
   */
  changeAffinity(other: string | Character, delta: number, reason: string): CharacterEvent[] {
    const target = this.otherId(other);
    assertFinite(delta, "delta");
    if (typeof reason !== "string" || reason.length === 0 || reason.length > MAX_KEY_LENGTH) {
      throw new CharacterError(
        "argument",
        `the reason must be an i18n key or an id of 1 to ${MAX_KEY_LENGTH} characters`,
      );
    }
    const rules = this.schema.relationships;
    const before = this.affinity(target);
    const after = clamp(before + delta, rules.min, rules.max);
    const applied = after - before;
    if (applied === 0) return [];
    this.values.affinity.set(target, after);
    this.values.history.push({ other: target, delta: applied, value: after, reason });
    const excess = this.values.history.length - rules.history_limit;
    if (excess > 0) this.values.history.splice(0, excess);

    const fromPosition = levelPosition(rules.levels, before);
    const toPosition = levelPosition(rules.levels, after);
    const from = rules.levels[fromPosition];
    const to = rules.levels[toPosition];
    if (from === undefined || to === undefined || from.id === to.id) return [];
    return [
      {
        event: "affinity_level",
        character: this.id,
        other: target,
        from: from.id,
        to: to.id,
        direction: toPosition > fromPosition ? "up" : "down",
      },
    ];
  }

  /** The state that a save keeps: the runtime values, and the format and version tags. */
  snapshot(): CharacterState {
    return toState(this.index, this.id, this.values);
  }

  /**
   * Replaces the runtime values with the ones of a state. The state is checked and migrated first; when
   * it is refused, the character keeps its values.
   *
   * @param state - a state, as `snapshot` writes it (for example, parsed from a save).
   * @throws {CharacterError} with code "unsupported-document" (another format, kind or a newer version),
   *   "invalid" (a field of the wrong type, or another character's state), or "bad-id".
   */
  restore(state: unknown): void {
    this.values = readState(this.index, this.id, this.start, state);
  }

  /** The thresholds that a move of a need from `before` to `after` crosses, in declaration order. */
  private crossings(def: NeedDef, before: number, after: number): CharacterEvent[] {
    return def.thresholds
      .filter((threshold) => crosses(threshold, before, after))
      .map((threshold): CharacterEvent => ({
        event: "threshold_crossed",
        character: this.id,
        need: def.id,
        threshold: threshold.id,
        when: threshold.when,
        value: after,
      }));
  }

  /**
   * The sum of the `add` values and the product of the `mul` values of the traits that this character has,
   * for one target. The traits are read in schema order, so the sums come out the same every time.
   */
  private modifiers(target: "stat" | "rate", id: string): { add: number; mul: number } {
    let add = 0;
    let mul = 1;
    for (const trait of this.index.traits.values()) {
      if (!this.values.traits.has(trait.id)) continue;
      for (const modifier of trait.modifiers) {
        if (modifier.target !== target || modifier.id !== id) continue;
        if (modifier.op === "add") add += modifier.value;
        else mul *= modifier.value;
      }
    }
    return { add, mul };
  }

  /** The other character's id, checked. */
  private otherId(other: string | Character): string {
    const id = typeof other === "string" ? other : other.id;
    if (!isId(id)) throw new CharacterError("bad-id", `"${id}" is not a character id`);
    if (id === this.id) throw new CharacterError("argument", "a character has no affinity with itself");
    return id;
  }

  /** The stat of the schema with this id. @throws {CharacterError} with code "unknown-stat". */
  private statDef(id: string): StatDef {
    const def = this.index.stats.get(id);
    if (def === undefined) throw new CharacterError("unknown-stat", `the schema has no stat "${id}"`);
    return def;
  }

  /** The base stat with this id. @throws {CharacterError} with code "unknown-stat" or "derived-stat". */
  private baseDef(id: string): BaseStatDef {
    const def = this.statDef(id);
    if (def.formula !== undefined) {
      throw new CharacterError("derived-stat", `"${id}" is derived from a formula and cannot be set`);
    }
    return def;
  }

  /** The need of the schema with this id. @throws {CharacterError} with code "unknown-need". */
  private needDef(id: string): NeedDef {
    const def = this.index.needs.get(id);
    if (def === undefined) throw new CharacterError("unknown-need", `the schema has no need "${id}"`);
    return def;
  }

  /** The profession of the schema with this id. @throws {CharacterError} with code "unknown-profession". */
  private professionDef(id: string): ProfessionDef {
    const def = this.index.professions.get(id);
    if (def === undefined) {
      throw new CharacterError("unknown-profession", `the schema has no profession "${id}"`);
    }
    return def;
  }

  /** The trait of the schema with this id. @throws {CharacterError} with code "unknown-trait". */
  private traitDef(id: string): TraitDef {
    const def = this.index.traits.get(id);
    if (def === undefined) throw new CharacterError("unknown-trait", `the schema has no trait "${id}"`);
    return def;
  }
}
