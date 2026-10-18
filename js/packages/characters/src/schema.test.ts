// Tests of parseStatSchema beyond the shared fixtures: the defaults that a small document gets, the
// rules that only some documents touch, and the lookups of a schema.

import { describe, expect, it } from "vitest";
import { levelOf, levelPosition, parseStatSchema } from "./schema";
import { expectCode } from "./test-support";
import type { StatSchema } from "./types";

/** A schema document with the given sections, under the envelope of version 1. */
function schemaText(sections: Readonly<Record<string, unknown>>): string {
  return JSON.stringify({ format: "velvet-character", version: 1, kind: "schema", ...sections });
}

/** A base stat that most tests can use. */
const STRENGTH = { id: "strength", label: "stat.strength", min: 0, max: 20, default: 8 };

describe("parseStatSchema defaults", () => {
  it("gives an empty schema when a document has no sections", () => {
    const schema = parseStatSchema(schemaText({}));
    expect(schema).toEqual({
      format: "velvet-character",
      version: 1,
      kind: "schema",
      stats: [],
      needs: [],
      professions: [],
      relationships: { min: -100, max: 100, default: 0, history_limit: 50, levels: [] },
      traits: [],
    });
  });

  it("fills in the rate, the thresholds and the modifiers when they are left out", () => {
    const schema = parseStatSchema(
      schemaText({
        stats: [STRENGTH],
        needs: [{ id: "hunger", label: "need.hunger", min: 0, max: 100, default: 80 }],
        traits: [{ id: "calm", label: "trait.calm" }],
      }),
    );
    expect(schema.needs[0]).toEqual({
      id: "hunger",
      label: "need.hunger",
      min: 0,
      max: 100,
      default: 80,
      rate: 0,
      thresholds: [],
    });
    expect(schema.traits[0]).toEqual({ id: "calm", label: "trait.calm", modifiers: [] });
  });

  it("keeps the order in which the document lists the stats, and gives a base stat no formula", () => {
    const schema = parseStatSchema(
      schemaText({
        stats: [
          { id: "double", label: "stat.double", min: 0, max: 40, formula: "strength * 2" },
          STRENGTH,
        ],
      }),
    );
    expect(schema.stats.map((stat) => stat.id)).toEqual(["double", "strength"]);
    expect(schema.stats[1]).toEqual(STRENGTH);
    expect(schema.stats[0]).not.toHaveProperty("default");
  });
});

describe("parseStatSchema rules", () => {
  it("refuses two thresholds of one need that share an id, but allows the same id in two needs", () => {
    const need = (id: string) => ({
      id,
      label: `need.${id}`,
      min: 0,
      max: 100,
      default: 50,
      thresholds: [
        { id: "low", at: 10, when: "below" },
        { id: "low", at: 20, when: "below" },
      ],
    });
    expectCode(() => parseStatSchema(schemaText({ needs: [need("hunger")] })), "duplicate-id");
    const twoNeeds = [
      { ...need("hunger"), thresholds: [{ id: "low", at: 10, when: "below" }] },
      { ...need("energy"), thresholds: [{ id: "low", at: 10, when: "below" }] },
    ];
    expect(parseStatSchema(schemaText({ needs: twoNeeds })).needs).toHaveLength(2);
  });

  it("refuses a relationship level that is listed twice", () => {
    const levels = [
      { id: "wary", label: "relation.wary", at: 0 },
      { id: "wary", label: "relation.wary", at: 30 },
    ];
    expectCode(() => parseStatSchema(schemaText({ relationships: { levels } })), "duplicate-id");
  });

  it("refuses a relationship range whose minimum is above its maximum", () => {
    expectCode(() => parseStatSchema(schemaText({ relationships: { min: 10, max: -10 } })), "invalid");
  });

  it("refuses a history limit outside 1 to 1000", () => {
    expectCode(() => parseStatSchema(schemaText({ relationships: { history_limit: 0 } })), "invalid");
    expectCode(() => parseStatSchema(schemaText({ relationships: { history_limit: 1001 } })), "invalid");
  });

  it("refuses levels that are not in ascending order", () => {
    const levels = [
      { id: "neutral", label: "relation.neutral", at: -100 },
      { id: "wary", label: "relation.wary", at: -30 },
      { id: "hostile", label: "relation.hostile", at: -50 },
    ];
    expectCode(() => parseStatSchema(schemaText({ relationships: { levels } })), "invalid");
  });

  it("refuses an experience curve that is empty", () => {
    const profession = { id: "farming", label: "profession.farming", xp_thresholds: [] };
    expectCode(() => parseStatSchema(schemaText({ professions: [profession] })), "invalid");
  });

  it("refuses a formula that is only white space", () => {
    const stat = { id: "double", label: "stat.double", min: 0, max: 40, formula: "   " };
    expectCode(() => parseStatSchema(schemaText({ stats: [stat] })), "invalid");
  });

  it("accepts derived stats that read other derived stats, in any order", () => {
    const schema = parseStatSchema(
      schemaText({
        stats: [
          { id: "top", label: "stat.top", min: 0, max: 100, formula: "middle + 1" },
          { id: "middle", label: "stat.middle", min: 0, max: 100, formula: "base * 2" },
          { ...STRENGTH, id: "base" },
        ],
      }),
    );
    expect(schema.stats.map((stat) => stat.id)).toEqual(["top", "middle", "base"]);
  });

  it("refuses a formula that reads a need, because formulas read stats only", () => {
    const schema = {
      stats: [{ id: "double", label: "stat.double", min: 0, max: 40, formula: "hunger * 2" }],
      needs: [{ id: "hunger", label: "need.hunger", min: 0, max: 100, default: 80 }],
    };
    expectCode(() => parseStatSchema(schemaText(schema)), "unknown-stat");
  });

  it("refuses a derived stat that reads itself", () => {
    const stat = { id: "loop", label: "stat.loop", min: 0, max: 10, formula: "loop + 1" };
    expectCode(() => parseStatSchema(schemaText({ stats: [stat] })), "cycle");
  });
});

describe("levels of affinity", () => {
  const schema: StatSchema = parseStatSchema(
    schemaText({
      relationships: {
        levels: [
          { id: "hostile", label: "relation.hostile", at: -100 },
          { id: "wary", label: "relation.wary", at: -30 },
          { id: "neutral", label: "relation.neutral", at: 0 },
        ],
      },
    }),
  );

  it("puts a value on the level whose line it reaches, so a line belongs to the higher level", () => {
    expect(levelPosition(schema.relationships.levels, -100)).toBe(0);
    expect(levelPosition(schema.relationships.levels, -31)).toBe(0);
    expect(levelPosition(schema.relationships.levels, -30)).toBe(1);
    expect(levelOf(schema.relationships, 0)?.id).toBe("neutral");
    expect(levelOf(schema.relationships, 100)?.id).toBe("neutral");
  });

  it("has no level for a value under the first level, and none when the schema has no levels", () => {
    expect(levelPosition(schema.relationships.levels, -101)).toBe(-1);
    expect(levelOf(parseStatSchema(schemaText({})).relationships, 5)).toBeNull();
  });
});
