// Tests of the runtime character: its values, the events of its changes, and its save state. The
// shared fixtures cover the main rules; these tests cover the refusals, the edges and the defaults.

import { describe, expect, it } from "vitest";
import { Character } from "./character";
import { parseCharacter, parseStatSchema } from "./index";
import { expectCode } from "./test-support";
import type { CharacterDefinition, CharacterState, StatSchema } from "./types";

/**
 * A small schema for these tests. One stat is named `constructor`, which is a valid id that every
 * object inherits; another is derived from a division by wisdom minus 6, which is zero at the default.
 */
const SCHEMA: StatSchema = parseStatSchema(
  JSON.stringify({
    format: "velvet-character",
    version: 1,
    kind: "schema",
    stats: [
      { id: "constructor", label: "stat.constructor", min: 0, max: 10, default: 4 },
      { id: "wisdom", label: "stat.wisdom", min: 0, max: 20, default: 6 },
      { id: "ratio", label: "stat.ratio", min: 0, max: 100, formula: "constructor / (wisdom - 6)" },
    ],
    needs: [
      {
        id: "hunger",
        label: "need.hunger",
        min: 0,
        max: 100,
        default: 50,
        rate: -1,
        thresholds: [{ id: "low", at: 20, when: "below" }],
      },
    ],
    professions: [{ id: "craft", label: "profession.craft", xp_thresholds: [0, 10, 30] }],
    relationships: {
      levels: [
        { id: "wary", label: "relation.wary", at: -100 },
        { id: "warm", label: "relation.warm", at: 10 },
      ],
    },
    traits: [
      {
        id: "keen",
        label: "trait.keen",
        modifiers: [
          { stat: "wisdom", add: 2 },
          { rate: "hunger", mul: 0.5 },
        ],
      },
      { id: "lazy", label: "trait.lazy", modifiers: [{ rate: "hunger", add: -1 }] },
    ],
  }),
);

/** A schema with no levels of affinity. */
const PLAIN_SCHEMA: StatSchema = parseStatSchema(
  JSON.stringify({
    format: "velvet-character",
    version: 1,
    kind: "schema",
    stats: [{ id: "wisdom", label: "stat.wisdom", min: 0, max: 20, default: 6 }],
  }),
);

/** A character sheet for the test schema, with the given fields. */
function sheet(fields: Readonly<Record<string, unknown>> = {}, id = "ada"): CharacterDefinition {
  return parseCharacter(
    JSON.stringify({
      format: "velvet-character",
      version: 1,
      kind: "character",
      id,
      name: `character.${id}.name`,
      ...fields,
    }),
    SCHEMA,
  );
}

/** A character of the test schema, with the given fields in its sheet. */
function make(fields: Readonly<Record<string, unknown>> = {}, id = "ada"): Character {
  return new Character(SCHEMA, sheet(fields, id));
}

describe("starting values", () => {
  it("starts at the defaults of the schema when the sheet gives no values", () => {
    const character = make();
    expect(character.baseStat("constructor")).toBe(4);
    expect(character.baseStat("wisdom")).toBe(6);
    expect(character.need("hunger")).toBe(50);
    expect(character.xp("craft")).toBe(0);
    expect(character.level("craft")).toBe(1);
    expect(character.traits).toEqual([]);
    expect(character.tags).toEqual([]);
    expect(character.art).toEqual({});
    expect(character.title).toBeUndefined();
  });

  it("reads a stat that is named after an inherited member of every object", () => {
    expect(make({ stats: { constructor: 7 } }).baseStat("constructor")).toBe(7);
    expect(make().baseStat("constructor")).toBe(4);
  });

  it("refuses a definition that names entries of another schema", () => {
    const other = parseStatSchema(
      JSON.stringify({
        format: "velvet-character",
        version: 1,
        kind: "schema",
        stats: [{ id: "wisdom", label: "stat.wisdom", min: 0, max: 20, default: 6 }],
      }),
    );
    expectCode(() => new Character(other, sheet()), "unknown-stat");
  });
});

describe("stats", () => {
  it("applies the traits to the base value, and then clamps the result", () => {
    expect(make({ stats: { wisdom: 18 }, traits: ["keen"] }).stat("wisdom")).toBe(20);
    expect(make({ stats: { wisdom: 19 }, traits: ["keen"] }).stat("wisdom")).toBe(20);
    expect(make({ stats: { wisdom: 10 }, traits: ["keen"] }).baseStat("wisdom")).toBe(10);
  });

  it("returns the effective value of every stat in schema order", () => {
    const character = make({ stats: { wisdom: 7 } });
    expect(Object.keys(character.stats())).toEqual(["constructor", "wisdom", "ratio"]);
    expect(character.stats()).toEqual({ constructor: 4, wisdom: 7, ratio: 4 });
  });

  it("fails with division-by-zero when a formula divides by zero at run time (strict)", () => {
    expectCode(() => new Character(SCHEMA, sheet({}, "ada"), { strict: true }).stat("ratio"), "division-by-zero");
  });

  it("by default uses the stat's minimum when a formula divides by zero, and reports it once", () => {
    const told: string[] = [];
    const character = new Character(SCHEMA, sheet({}, "ada"), { onFormulaError: (stat, error) => told.push(`${stat}:${error.code}`) });
    const first = character.stat("ratio");
    character.stat("ratio");
    expect(Number.isFinite(first)).toBe(true);
    expect(told).toEqual(["ratio:division-by-zero"]);
  });

  it("refuses an unknown stat, and a change to a derived stat", () => {
    const character = make();
    expectCode(() => character.stat("speed"), "unknown-stat");
    expectCode(() => character.changeBase("ratio", 1), "derived-stat");
    expectCode(() => character.baseStat("ratio"), "derived-stat");
  });

  it("clamps a change to a base stat, and refuses a value that is not finite", () => {
    const character = make();
    expect(character.changeBase("wisdom", 100)).toBe(20);
    expect(character.baseStat("wisdom")).toBe(20);
    expect(character.changeBase("wisdom", -100)).toBe(0);
    expectCode(() => character.changeBase("wisdom", Number.NaN), "argument");
  });
});

describe("needs", () => {
  it("reports the thresholds that a move crossed, and nothing for a move that stays on one side", () => {
    const character = make();
    expect(character.tickNeeds(25)).toEqual([]);
    expect(character.need("hunger")).toBe(25);
    expect(character.tickNeeds(10)).toEqual([
      { event: "threshold_crossed", character: "ada", need: "hunger", threshold: "low", when: "below", value: 15 },
    ]);
  });

  it("applies the trait modifiers to the rate: keen halves the decay, lazy adds to it", () => {
    expect(make({ traits: ["keen"] }).rate("hunger")).toBe(-0.5);
    expect(make({ traits: ["lazy"] }).rate("hunger")).toBe(-2);
    expect(make({ traits: ["keen", "lazy"] }).rate("hunger")).toBe(-1);
  });

  it("refuses a time step that is negative or not finite", () => {
    const character = make();
    expectCode(() => character.tickNeeds(-1), "argument");
    expectCode(() => character.tickNeeds(Number.NaN), "argument");
    expectCode(() => character.tickNeeds(Number.POSITIVE_INFINITY), "argument");
  });

  it("clamps a change to a need, and refuses an unknown need", () => {
    const character = make();
    expect(character.changeNeed("hunger", 100)).toEqual([]);
    expect(character.need("hunger")).toBe(100);
    expectCode(() => character.changeNeed("thirst", 1), "unknown-need");
    expectCode(() => character.rate("thirst"), "unknown-need");
  });
});

describe("professions", () => {
  it("reaches a level exactly when experience reaches its line", () => {
    const character = make();
    expect(character.gainXp("craft", 9)).toEqual([]);
    expect(character.gainXp("craft", 1)).toEqual([
      { event: "level_up", character: "ada", profession: "craft", level: 2 },
    ]);
  });

  it("stops experience at the last line of the curve, and reports every level it passed", () => {
    const character = make();
    expect(character.gainXp("craft", 1000)).toEqual([
      { event: "level_up", character: "ada", profession: "craft", level: 2 },
      { event: "level_up", character: "ada", profession: "craft", level: 3 },
    ]);
    expect(character.xp("craft")).toBe(30);
    expect(character.gainXp("craft", 1)).toEqual([]);
  });

  it("refuses a negative amount, and a profession the schema does not have", () => {
    const character = make();
    expectCode(() => character.gainXp("craft", -1), "argument");
    expectCode(() => character.gainXp("cooking", 1), "unknown-profession");
    expectCode(() => character.level("cooking"), "unknown-profession");
  });
});

describe("affinity", () => {
  it("accepts another character as the other side, and reports the level it moves to", () => {
    const ada = make();
    const bea = make({}, "bea");
    expect(ada.changeAffinity(bea, 40, "gift.bread")).toEqual([
      { event: "affinity_level", character: "ada", other: "bea", from: "wary", to: "warm", direction: "up" },
    ]);
    expect(ada.affinity(bea)).toBe(40);
    expect(ada.affinityLevel("bea")).toBe("warm");
  });

  it("refuses an affinity with itself, and an other side that is not an id", () => {
    const ada = make();
    expectCode(() => ada.affinity("ada"), "argument");
    expectCode(() => ada.changeAffinity("ada", 1, "self"), "argument");
    expectCode(() => ada.affinity("Bad Id"), "bad-id");
  });

  it("refuses a reason that is empty or longer than an i18n key may be", () => {
    const ada = make();
    expectCode(() => ada.changeAffinity("bea", 1, ""), "argument");
    expectCode(() => ada.changeAffinity("bea", 1, "x".repeat(201)), "argument");
  });

  it("keeps only the change that was applied, and records nothing for a change that moved nothing", () => {
    const ada = make();
    ada.changeAffinity("bea", 500, "gift.big");
    expect(ada.affinity("bea")).toBe(100);
    expect(ada.history).toEqual([{ other: "bea", delta: 100, value: 100, reason: "gift.big" }]);
    expect(ada.changeAffinity("bea", 5, "gift.more")).toEqual([]);
    expect(ada.history).toHaveLength(1);
  });

  it("has no level when the schema has no levels", () => {
    const sheetText = JSON.stringify({
      format: "velvet-character",
      version: 1,
      kind: "character",
      id: "ada",
      name: "character.ada.name",
    });
    const plain = new Character(PLAIN_SCHEMA, parseCharacter(sheetText, PLAIN_SCHEMA));
    expect(plain.affinityLevel("bea")).toBeNull();
    expect(plain.changeAffinity("bea", 5, "gift")).toEqual([]);
    expect(plain.affinities()).toEqual([{ other: "bea", value: 5, level: null }]);
  });

  it("lists the pairs that hold a value, sorted by the other character", () => {
    const ada = make({ relationships: { zed: 5, amy: -50 } });
    expect(ada.affinities()).toEqual([
      { other: "amy", value: -50, level: "wary" },
      { other: "zed", value: 5, level: "wary" },
    ]);
  });
});

describe("traits", () => {
  it("lists the traits sorted, and adds and removes them", () => {
    const character = make({ traits: ["lazy", "keen"] });
    expect(character.traits).toEqual(["keen", "lazy"]);
    character.addTrait("keen");
    expect(character.traits).toEqual(["keen", "lazy"]);
    character.removeTrait("lazy");
    expect(character.hasTrait("lazy")).toBe(false);
    character.removeTrait("lazy");
    expect(character.traits).toEqual(["keen"]);
  });

  it("refuses a trait that the schema does not have", () => {
    const character = make();
    expectCode(() => character.addTrait("dragon"), "unknown-trait");
    expectCode(() => character.removeTrait("dragon"), "unknown-trait");
  });
});

describe("save state", () => {
  it("writes the envelope, and the values in schema order", () => {
    const state = make({ needs: { hunger: 30 }, relationships: { bea: 2 } }).snapshot();
    expect(state).toEqual({
      format: "velvet-character",
      version: 1,
      kind: "state",
      id: "ada",
      stats: { constructor: 4, wisdom: 6 },
      needs: { hunger: 30 },
      xp: { craft: 0 },
      traits: [],
      affinity: { bea: 2 },
      history: [],
    });
    expect(Object.keys(state.stats)).toEqual(["constructor", "wisdom"]);
  });

  it("reads back what it wrote, through JSON", () => {
    const ada = make({ traits: ["keen"] });
    ada.gainXp("craft", 12);
    ada.tickNeeds(7);
    ada.changeAffinity("bea", 30, "gift.bread");
    const before = ada.snapshot();
    const copy = make();
    copy.restore(JSON.parse(JSON.stringify(before)));
    expect(copy.snapshot()).toEqual(before);
    expect(copy.level("craft")).toBe(2);
  });

  it("refuses a state of another format, kind or newer version, or a version that is not whole", () => {
    const character = make();
    const good = character.snapshot();
    expectCode(() => character.restore({ ...good, format: "velvet-stacks" }), "unsupported-document");
    expectCode(() => character.restore({ ...good, kind: "character" }), "unsupported-document");
    expectCode(() => character.restore({ ...good, version: 2 }), "unsupported-document");
    expectCode(() => character.restore({ ...good, version: 0 }), "invalid");
    expectCode(() => character.restore({ ...good, version: 1.5 }), "invalid");
    expectCode(() => character.restore(null), "invalid");
  });

  it("refuses a state of another character, and a state with a section missing", () => {
    const character = make();
    const withoutXp: Record<string, unknown> = { ...character.snapshot() };
    delete withoutXp["xp"];
    expectCode(() => character.restore({ ...character.snapshot(), id: "bea" }), "invalid");
    expectCode(() => character.restore(withoutXp), "invalid");
  });

  it("leaves the character as it was when a state is refused", () => {
    const character = make();
    character.changeBase("wisdom", 3);
    const before = character.snapshot();
    expectCode(() => character.restore({ ...before, affinity: { ada: 1 } }), "invalid");
    expect(character.snapshot()).toEqual(before);
  });

  it("clamps values, drops what the schema no longer has, and takes starting values for what is missing", () => {
    const character = make({ traits: ["keen"] });
    const state: CharacterState = {
      ...character.snapshot(),
      stats: { wisdom: 99, gone: 3 },
      needs: { hunger: -10 },
      xp: { craft: 500 },
      traits: ["lazy", "ghost"],
      affinity: { bea: 500, cy: -500 },
    };
    character.restore(state);
    expect(character.baseStat("wisdom")).toBe(20);
    expect(character.baseStat("constructor")).toBe(4);
    expect(character.need("hunger")).toBe(0);
    expect(character.xp("craft")).toBe(30);
    expect(character.traits).toEqual(["lazy"]);
    expect(character.affinity("bea")).toBe(100);
    expect(character.affinity("cy")).toBe(-100);
  });

  it("keeps the newest entries of a history longer than its limit", () => {
    const history = Array.from({ length: 60 }, (_, index) => ({
      other: "bea",
      delta: 1,
      value: index,
      reason: `step.${index}`,
    }));
    const character = make();
    character.restore({ ...character.snapshot(), history });
    expect(character.history).toHaveLength(50);
    expect(character.history[0]?.reason).toBe("step.10");
    expect(character.history[49]?.reason).toBe("step.59");
  });

  it("refuses an affinity with itself or with a name that is not an id, and a history entry with a bad field", () => {
    const character = make();
    expectCode(() => character.restore({ ...character.snapshot(), affinity: { ada: 1 } }), "invalid");
    expectCode(() => character.restore({ ...character.snapshot(), affinity: { "Bad Id": 1 } }), "bad-id");
    expectCode(
      () =>
        character.restore({
          ...character.snapshot(),
          history: [{ other: "bea", delta: "1", value: 1, reason: "gift" }],
        }),
      "invalid",
    );
  });
});
