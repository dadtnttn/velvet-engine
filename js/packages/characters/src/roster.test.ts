// Tests of Roster: the lookups by id and tag, the affinity queries in both directions, and the
// restore of several characters at once.

import { describe, expect, it } from "vitest";
import { Character } from "./character";
import { parseCharacter, parseStatSchema } from "./index";
import { Roster } from "./roster";
import { expectCode } from "./test-support";
import type { StatSchema } from "./types";

/** A schema with two levels of affinity, and one stat. */
function makeSchema(): StatSchema {
  return parseStatSchema(
    JSON.stringify({
      format: "velvet-character",
      version: 1,
      kind: "schema",
      stats: [{ id: "wisdom", label: "stat.wisdom", min: 0, max: 20, default: 6 }],
      relationships: {
        levels: [
          { id: "wary", label: "relation.wary", at: -100 },
          { id: "warm", label: "relation.warm", at: 10 },
        ],
      },
    }),
  );
}

/** A character of the schema, from a sheet with the given fields. */
function character(schema: StatSchema, id: string, fields: Readonly<Record<string, unknown>> = {}): Character {
  const sheet = JSON.stringify({
    format: "velvet-character",
    version: 1,
    kind: "character",
    id,
    name: `character.${id}.name`,
    ...fields,
  });
  return new Character(schema, parseCharacter(sheet, schema));
}

describe("Roster", () => {
  it("refuses two characters with one id, and characters that follow different schemas", () => {
    const schema = makeSchema();
    expectCode(() => new Roster([character(schema, "ada"), character(schema, "ada")]), "duplicate-id");
    expectCode(() => new Roster([character(schema, "ada"), character(makeSchema(), "bea")]), "argument");
  });

  it("finds characters by id, and by tag, sorted by id", () => {
    const schema = makeSchema();
    const roster = new Roster([
      character(schema, "zoe", { tags: ["farm"] }),
      character(schema, "ada", { tags: ["farm", "guard"] }),
      character(schema, "bea", { tags: ["guard"] }),
    ]);
    expect(roster.all().map((entry) => entry.id)).toEqual(["ada", "bea", "zoe"]);
    expect(roster.byTag("farm").map((entry) => entry.id)).toEqual(["ada", "zoe"]);
    expect(roster.byTag("cook")).toEqual([]);
    expect(roster.get("bea")?.id).toBe("bea");
    expect(roster.get("nobody")).toBeUndefined();
  });

  it("lists the affinities a character holds, and the opinions that others hold about it", () => {
    const schema = makeSchema();
    const ada = character(schema, "ada", { relationships: { bea: 40, zoe: -20 } });
    const bea = character(schema, "bea", { relationships: { ada: 5 } });
    const zoe = character(schema, "zoe", { relationships: { ada: -20 } });
    const roster = new Roster([ada, bea, zoe]);

    expect(roster.relationsOf("ada")).toEqual([
      { other: "bea", value: 40, level: "warm" },
      { other: "zoe", value: -20, level: "wary" },
    ]);
    expect(roster.opinionsOf("ada")).toEqual([
      { from: "bea", value: 5, level: "wary" },
      { from: "zoe", value: -20, level: "wary" },
    ]);
    expect(roster.opinionsOf("zoe")).toEqual([{ from: "ada", value: -20, level: "wary" }]);
  });

  it("keeps only the affinities at a level or higher when a minimum level is given", () => {
    const schema = makeSchema();
    const ada = character(schema, "ada", { relationships: { bea: 40, zoe: -20 } });
    const bea = character(schema, "bea", { relationships: { ada: 5 } });
    const roster = new Roster([ada, bea]);
    expect(roster.relationsOf("ada", { minLevel: "warm" })).toEqual([{ other: "bea", value: 40, level: "warm" }]);
    expect(roster.opinionsOf("ada", { minLevel: "warm" })).toEqual([]);
  });

  it("refuses a character or a level that the roster or the schema does not have", () => {
    const schema = makeSchema();
    const roster = new Roster([character(schema, "ada")]);
    expectCode(() => roster.relationsOf("nobody"), "unknown-character");
    expectCode(() => roster.opinionsOf("nobody"), "unknown-character");
    expectCode(() => roster.relationsOf("ada", { minLevel: "legend" }), "unknown-level");
  });

  it("writes one state per character, sorted by id, and restores them", () => {
    const schema = makeSchema();
    const roster = new Roster([character(schema, "bea"), character(schema, "ada")]);
    const saved = roster.snapshot();
    expect(saved.map((state) => state.id)).toEqual(["ada", "bea"]);

    roster.get("ada")?.changeBase("wisdom", 5);
    roster.restore(JSON.parse(JSON.stringify(saved)));
    expect(roster.snapshot()).toEqual(saved);
  });

  it("applies every state or none: a refused state leaves every character as it was", () => {
    const schema = makeSchema();
    const roster = new Roster([character(schema, "ada"), character(schema, "bea")]);
    const before = roster.snapshot();
    const ada = { ...before[0], stats: { wisdom: 10 } };
    const bea = { ...before[1], affinity: { bea: 1 } };
    expectCode(() => roster.restore([ada, bea]), "invalid");
    expect(roster.snapshot()).toEqual(before);
  });

  it("leaves a character without a state as it is", () => {
    const schema = makeSchema();
    const roster = new Roster([character(schema, "ada"), character(schema, "bea")]);
    roster.get("ada")?.changeBase("wisdom", 5);
    const adaBefore = roster.get("ada")?.snapshot();
    roster.restore([roster.get("bea")?.snapshot()]);
    expect(roster.get("ada")?.snapshot()).toEqual(adaBefore);
  });

  it("refuses a restore that is not a list, a state without an id, a repeated id, or an unknown character", () => {
    const schema = makeSchema();
    const roster = new Roster([character(schema, "ada")]);
    const ada = roster.get("ada")?.snapshot();
    expectCode(() => roster.restore({}), "invalid");
    expectCode(() => roster.restore([{ stats: {} }]), "invalid");
    expectCode(() => roster.restore([ada, ada]), "duplicate-id");
    expectCode(() => roster.restore([{ ...ada, id: "nobody" }]), "unknown-character");
  });

  it("adds a character with join, and refuses an id that the roster lists or a character of another schema", () => {
    const schema = makeSchema();
    const roster = new Roster([character(schema, "ada")]);
    roster.join(character(schema, "bea"));
    expect(roster.all().map((entry) => entry.id)).toEqual(["ada", "bea"]);
    expect(roster.snapshot().map((state) => state.id)).toEqual(["ada", "bea"]);
    expectCode(() => roster.join(character(schema, "ada")), "duplicate-id");
    expectCode(() => roster.join(character(makeSchema(), "cora")), "argument");
    expect(roster.get("cora")).toBeUndefined();
  });

  it("takes the schema from the first character that joins an empty roster", () => {
    const schema = makeSchema();
    const roster = new Roster([]);
    roster.join(character(schema, "ada"));
    expectCode(() => roster.join(character(makeSchema(), "bea")), "argument");
  });

  it("removes a character with leave, returns it, and leaves the queries and the snapshot without it", () => {
    const schema = makeSchema();
    const bea = character(schema, "bea", { relationships: { ada: 5 } });
    const roster = new Roster([character(schema, "ada", { relationships: { bea: 40 } }), bea]);
    expect(roster.leave("bea")).toBe(bea);
    expect(roster.get("bea")).toBeUndefined();
    expect(roster.opinionsOf("ada")).toEqual([]);
    expect(roster.snapshot().map((state) => state.id)).toEqual(["ada"]);
    expectCode(() => roster.leave("bea"), "unknown-character");
    expectCode(() => roster.relationsOf("bea"), "unknown-character");
  });

  it("brings a character that left back when a restore names a state written while it was a member", () => {
    const schema = makeSchema();
    const roster = new Roster([character(schema, "ada"), character(schema, "bea")]);
    roster.get("bea")?.changeBase("wisdom", 4);
    const saved = roster.snapshot();
    roster.leave("bea");
    roster.get("ada")?.changeBase("wisdom", 9);
    roster.restore(JSON.parse(JSON.stringify(saved)));
    expect(roster.snapshot()).toEqual(saved);
    expect(roster.get("bea")?.baseStat("wisdom")).toBe(10);
  });

  it("brings nobody back when another state of the restore is refused", () => {
    const schema = makeSchema();
    const roster = new Roster([character(schema, "ada"), character(schema, "bea")]);
    const saved = roster.snapshot();
    roster.leave("bea");
    const refusedAda = { ...saved[0], affinity: { ada: 1 } };
    expectCode(() => roster.restore([refusedAda, saved[1]]), "invalid");
    expect(roster.get("bea")).toBeUndefined();
    expect(roster.snapshot().map((state) => state.id)).toEqual(["ada"]);
  });

  it("gives the id of a character that left to a new character, which a restore then updates", () => {
    const schema = makeSchema();
    const roster = new Roster([character(schema, "ada"), character(schema, "bea")]);
    const saved = roster.snapshot();
    roster.leave("bea");
    const newBea = character(schema, "bea");
    roster.join(newBea);
    roster.restore(saved);
    expect(roster.get("bea")).toBe(newBea);
    expect(roster.snapshot()).toEqual(saved);
  });

  it("keeps a character that joined after the save, because a restore never removes a member", () => {
    const schema = makeSchema();
    const roster = new Roster([character(schema, "ada")]);
    const saved = roster.snapshot();
    roster.join(character(schema, "bea"));
    roster.restore(saved);
    expect(roster.all().map((entry) => entry.id)).toEqual(["ada", "bea"]);
  });
});
