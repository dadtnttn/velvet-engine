// Runs the shared character fixtures (docs/formats/fixtures/characters) through this package. The
// Rust reader will read the same files, so each case states its results in the format's own terms:
// the events a step raises, the values after it, the error code a refused step gives.

import { describe, expect, it } from "vitest";
import {
  Character,
  Roster,
  evaluateExpression,
  parseCharacter,
  parseExpression,
  parseStatSchema,
  type CharacterDefinition,
  type CharacterErrorCode,
  type CharacterEvent,
} from "./index";
import { expectCode, fixtureFiles, readFixture } from "./test-support";

/** One step of a case. `do` says what happens; the fields that follow say with what. */
interface Step {
  readonly do:
    | "tick"
    | "change_need"
    | "gain_xp"
    | "change_affinity"
    | "change_base"
    | "add_trait"
    | "remove_trait"
    | "restore"
    | "reload"
    | "check";
  readonly character: string;
  readonly dt?: number;
  readonly need?: string;
  readonly delta?: number;
  readonly profession?: string;
  readonly amount?: number;
  readonly other?: string;
  readonly reason?: string;
  readonly stat?: string;
  readonly trait?: string;
  readonly state?: unknown;
  /** The events the step must raise, in order. A step without this field raises any events. */
  readonly events?: readonly CharacterEvent[];
  /** The code of the error that the step must raise. The step then changes nothing. */
  readonly error?: CharacterErrorCode;
  readonly base?: Readonly<Record<string, number>>;
  readonly stats?: Readonly<Record<string, number>>;
  readonly rates?: Readonly<Record<string, number>>;
  readonly needs?: Readonly<Record<string, number>>;
  readonly xp?: Readonly<Record<string, number>>;
  readonly levels?: Readonly<Record<string, number>>;
  readonly affinity?: Readonly<Record<string, number>>;
  readonly affinity_levels?: Readonly<Record<string, string | null>>;
  readonly traits?: readonly string[];
  readonly history?: readonly unknown[];
}

/** One query of the roster, run after the steps. */
interface Query {
  readonly kind: "by_tag" | "relations" | "opinions";
  readonly tag?: string;
  readonly of?: string;
  readonly min_level?: string;
  readonly expect?: readonly unknown[];
  readonly error?: CharacterErrorCode;
}

/** One case file: the characters it loads, its steps, and the roster queries at the end. */
interface CaseFile {
  readonly name: string;
  readonly characters: readonly string[];
  readonly steps: readonly Step[];
  readonly queries?: readonly Query[];
}

/** One formula of the expression cases. */
interface ExpressionCase {
  readonly formula: string;
  readonly vars?: Readonly<Record<string, number>>;
  readonly value?: number;
  readonly error?: CharacterErrorCode;
}

/** One document that must be refused. */
interface InvalidCase {
  readonly name: string;
  readonly as: "schema" | "character";
  readonly text?: string;
  readonly document?: unknown;
  readonly error: CharacterErrorCode;
}

/** Reads a required field of a case, and fails the test with its name when the fixture leaves it out. */
function required<T>(value: T | undefined, what: string): T {
  if (value === undefined) throw new Error(`the fixture leaves out ${what}`);
  return value;
}

/** Runs a call that a step expects to fail with `error`, or returns what the call returns. */
function act<T>(step: Step, call: () => T): T | undefined {
  if (step.error !== undefined) {
    expectCode(call, step.error);
    return undefined;
  }
  return call();
}

/** Checks the events of a step, when the step lists them. A step that expects an error has none. */
function expectEvents(step: Step, events: readonly CharacterEvent[] | undefined): void {
  if (step.error === undefined && step.events !== undefined) expect(events).toEqual(step.events);
}

const schema = parseStatSchema(await readFixture("schema.json"));

const definitions = new Map<string, CharacterDefinition>();
for (const name of await fixtureFiles("sheets")) {
  const definition = parseCharacter(await readFixture(`sheets/${name}`), schema);
  definitions.set(definition.id, definition);
}

/** The case files, in name order: each one is a test. */
const caseFiles = await fixtureFiles("cases");

/** Runs one step against the case's characters. */
function runStep(step: Step, characters: Map<string, Character>): void {
  const character = required(characters.get(step.character), `a character "${step.character}" in the case`);
  switch (step.do) {
    case "tick":
      expectEvents(step, act(step, () => character.tickNeeds(required(step.dt, "dt"))));
      return;
    case "change_need":
      expectEvents(
        step,
        act(step, () => character.changeNeed(required(step.need, "need"), required(step.delta, "delta"))),
      );
      return;
    case "gain_xp":
      expectEvents(
        step,
        act(step, () => character.gainXp(required(step.profession, "profession"), required(step.amount, "amount"))),
      );
      return;
    case "change_affinity":
      expectEvents(
        step,
        act(step, () =>
          character.changeAffinity(
            required(step.other, "other"),
            required(step.delta, "delta"),
            required(step.reason, "reason"),
          ),
        ),
      );
      return;
    case "change_base":
      act(step, () => character.changeBase(required(step.stat, "stat"), required(step.delta, "delta")));
      return;
    case "add_trait":
      act(step, () => character.addTrait(required(step.trait, "trait")));
      return;
    case "remove_trait":
      act(step, () => character.removeTrait(required(step.trait, "trait")));
      return;
    case "restore":
      act(step, () => character.restore(step.state));
      return;
    case "reload": {
      // A save and a reload: the state read back from JSON must give the same state, and the same character.
      const before = character.snapshot();
      const fresh = new Character(schema, required(definitions.get(step.character), "a sheet"));
      fresh.restore(JSON.parse(JSON.stringify(before)));
      expect(fresh.snapshot()).toEqual(before);
      characters.set(step.character, fresh);
      return;
    }
    case "check":
      checkValues(step, character);
      return;
  }
}

/** Checks the values that a `check` step names. Numbers are compared to 9 decimal places. */
function checkValues(step: Step, character: Character): void {
  for (const [stat, value] of Object.entries(step.base ?? {})) {
    expect(character.baseStat(stat), `base ${stat}`).toBeCloseTo(value, 9);
  }
  for (const [stat, value] of Object.entries(step.stats ?? {})) {
    expect(character.stat(stat), `stat ${stat}`).toBeCloseTo(value, 9);
  }
  for (const [need, value] of Object.entries(step.rates ?? {})) {
    expect(character.rate(need), `rate of ${need}`).toBeCloseTo(value, 9);
  }
  for (const [need, value] of Object.entries(step.needs ?? {})) {
    expect(character.need(need), `need ${need}`).toBeCloseTo(value, 9);
  }
  for (const [profession, value] of Object.entries(step.xp ?? {})) {
    expect(character.xp(profession), `xp of ${profession}`).toBeCloseTo(value, 9);
  }
  for (const [profession, value] of Object.entries(step.levels ?? {})) {
    expect(character.level(profession), `level of ${profession}`).toBe(value);
  }
  for (const [other, value] of Object.entries(step.affinity ?? {})) {
    expect(character.affinity(other), `affinity toward ${other}`).toBeCloseTo(value, 9);
  }
  for (const [other, level] of Object.entries(step.affinity_levels ?? {})) {
    expect(character.affinityLevel(other), `level toward ${other}`).toBe(level);
  }
  if (step.traits !== undefined) expect(character.traits).toEqual(step.traits);
  if (step.history !== undefined) expect(character.history).toEqual(step.history);
}

/** Runs one roster query and compares its answer with the expected one. */
function runQuery(query: Query, roster: Roster): void {
  const answer = (): unknown => {
    const minLevel = query.min_level === undefined ? {} : { minLevel: query.min_level };
    switch (query.kind) {
      case "by_tag":
        return roster.byTag(required(query.tag, "tag")).map((character) => character.id);
      case "relations":
        return roster
          .relationsOf(required(query.of, "of"), minLevel)
          .map((relation) => ({ other: relation.other, value: relation.value, level: relation.level }));
      case "opinions":
        return roster
          .opinionsOf(required(query.of, "of"), minLevel)
          .map((opinion) => ({ from: opinion.from, value: opinion.value, level: opinion.level }));
    }
  };
  if (query.error !== undefined) {
    expectCode(answer, query.error);
    return;
  }
  expect(answer()).toEqual(required(query.expect, "expect"));
}

describe("the shared character fixtures", () => {
  it("reads the schema and the three sheets", () => {
    expect(schema.stats.map((stat) => stat.id)).toEqual(["strength", "wisdom", "charm", "work_power", "morale"]);
    expect([...definitions.keys()].sort()).toEqual(["bram", "ines", "mara"]);
  });

  for (const name of caseFiles) {
    it(`case ${name}`, async () => {
      const file = JSON.parse(await readFixture(`cases/${name}`)) as CaseFile;
      const characters = new Map<string, Character>();
      for (const id of file.characters) {
        characters.set(id, new Character(schema, required(definitions.get(id), `a sheet for "${id}"`)));
      }
      for (const step of file.steps) runStep(step, characters);
      const roster = new Roster(characters.values());
      for (const query of file.queries ?? []) runQuery(query, roster);
    });
  }

  it("the formula language gives the values and errors of expressions.json", async () => {
    const file = JSON.parse(await readFixture("expressions.json")) as { cases: readonly ExpressionCase[] };
    for (const item of file.cases) {
      const vars = item.vars ?? {};
      const compute = (): number =>
        evaluateExpression(parseExpression(item.formula), (name) =>
          Object.hasOwn(vars, name) ? vars[name] : undefined,
        );
      if (item.error !== undefined) {
        expectCode(compute, item.error);
      } else {
        expect(compute(), item.formula).toBeCloseTo(required(item.value, `the value of ${item.formula}`), 9);
      }
    }
  });

  it("refuses every document of invalid.json with the code that names its rule", async () => {
    const file = JSON.parse(await readFixture("invalid.json")) as { cases: readonly InvalidCase[] };
    for (const item of file.cases) {
      const text = item.text ?? JSON.stringify(item.document);
      if (item.as === "schema") {
        expectCode(() => parseStatSchema(text), item.error);
      } else {
        expectCode(() => parseCharacter(text, schema), item.error);
      }
    }
  });
});
