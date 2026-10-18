// Character documents: the static definition of one character (kind "character" in
// docs/formats/CHARACTER.md). parseCharacter reads one against a stat schema, checks every rule, and
// fills in the starting values that the document leaves out.

import { CharacterError } from "./error";
import {
  checkUnique,
  invalid,
  isId,
  optionalList,
  ownValue,
  parseJson,
  readArtId,
  readEnvelope,
  readId,
  readKey,
  readNumber,
  readRecord,
} from "./read";
import { indexSchema, type SchemaIndex } from "./schema";
import {
  CHARACTER_FORMAT,
  CHARACTER_VERSION,
  type CharacterArt,
  type CharacterDefinition,
  type StatSchema,
} from "./types";

/**
 * Reads a character document against a schema, checks every rule, and returns the definition in its
 * normal form: every starting value written out, the defaults of the schema filled in.
 *
 * @param json - the text of the character file.
 * @param schema - the schema that the character follows (from parseStatSchema).
 * @returns the checked definition.
 * @throws {CharacterError} with the code of the rule that broke: "json", "unsupported-document",
 *   "invalid", "bad-id", "duplicate-id", "unknown-stat", "unknown-need", "unknown-profession",
 *   "unknown-trait", "derived-stat" (a starting value for a derived stat), or "out-of-range".
 *
 * @example
 * const mara = parseCharacter(await readText("mara.json"), schema);
 */
export function parseCharacter(json: string, schema: StatSchema): CharacterDefinition {
  return completeCharacter(indexSchema(schema), readEnvelope(parseJson(json), "character"));
}

/**
 * Checks the fields of a character against a schema and returns the normal form. Used by parseCharacter
 * and by the Character constructor, so that a definition made by hand is checked the same way.
 *
 * @param index - the schema's lookups.
 * @param fields - the character's fields, as a document or a definition object gives them.
 * @throws {CharacterError} with the code of the rule that broke.
 */
export function completeCharacter(index: SchemaIndex, fields: Record<string, unknown>): CharacterDefinition {
  const id = readId(fields["id"], "id");
  const definition: CharacterDefinition = {
    format: CHARACTER_FORMAT,
    version: CHARACTER_VERSION,
    kind: "character",
    id,
    name: readKey(fields["name"], "name"),
    ...(fields["title"] === undefined ? {} : { title: readKey(fields["title"], "title") }),
    art: readArt(fields["art"]),
    tags: readTags(optionalList(fields, "character", "tags")),
    stats: readStartStats(index, fields["stats"]),
    needs: readStartNeeds(index, fields["needs"]),
    professions: readStartExperience(index, fields["professions"]),
    traits: readTraitIds(index, optionalList(fields, "character", "traits")),
    relationships: readStartAffinity(index, id, fields["relationships"]),
  };
  return definition;
}

/** Reads the art ids of a character. Each is optional. */
function readArt(value: unknown): CharacterArt {
  if (value === undefined) return {};
  const record = readRecord(value, "art");
  const portrait = record["portrait"] === undefined ? undefined : readArtId(record["portrait"], "art.portrait");
  const fullbody = record["fullbody"] === undefined ? undefined : readArtId(record["fullbody"], "art.fullbody");
  return {
    ...(portrait === undefined ? {} : { portrait }),
    ...(fullbody === undefined ? {} : { fullbody }),
  };
}

/** Reads the free tags of a character. Each one is an id, and no tag is listed twice. */
function readTags(list: readonly unknown[]): string[] {
  const seen = new Set<string>();
  return list.map((item, index) => {
    const tag = readId(item, `tags[${index}]`);
    checkUnique(seen, tag, "tags");
    return tag;
  });
}

/** Reads the traits that the character starts with. Each must be a trait of the schema. */
function readTraitIds(index: SchemaIndex, list: readonly unknown[]): string[] {
  const seen = new Set<string>();
  return list.map((item, position) => {
    const id = readId(item, `traits[${position}]`);
    if (!index.traits.has(id)) {
      throw new CharacterError("unknown-trait", `traits[${position}] names no trait of the schema`);
    }
    checkUnique(seen, id, "traits");
    return id;
  });
}

/** Throws the out-of-range error unless `amount` lies from `min` to `max`. */
function checkRange(amount: number, min: number, max: number, path: string): void {
  if (amount < min || amount > max) {
    throw new CharacterError("out-of-range", `${path} is ${amount}; it must be from ${min} to ${max}`);
  }
}

/**
 * Reads the starting base values of the stats. Every base stat gets a value: the document's, or the
 * default of the schema. A derived stat has no starting value.
 */
function readStartStats(index: SchemaIndex, value: unknown): Record<string, number> {
  const given: Record<string, unknown> = value === undefined ? {} : readRecord(value, "stats");
  for (const key of Object.keys(given)) {
    const def = index.stats.get(key);
    if (def === undefined) throw new CharacterError("unknown-stat", `stats.${key} names no stat of the schema`);
    if (def.formula !== undefined) {
      throw new CharacterError("derived-stat", `stats.${key} is derived from a formula; only base stats have values`);
    }
  }
  const starts: Record<string, number> = {};
  for (const def of index.stats.values()) {
    if (def.formula !== undefined) continue;
    const path = `stats.${def.id}`;
    const raw = ownValue(given, def.id);
    const amount = raw === undefined ? def.default : readNumber(raw, path);
    checkRange(amount, def.min, def.max, path);
    starts[def.id] = amount;
  }
  return starts;
}

/** Reads the starting value of every need: the document's, or the default of the schema. */
function readStartNeeds(index: SchemaIndex, value: unknown): Record<string, number> {
  const given: Record<string, unknown> = value === undefined ? {} : readRecord(value, "needs");
  for (const key of Object.keys(given)) {
    if (!index.needs.has(key)) throw new CharacterError("unknown-need", `needs.${key} names no need of the schema`);
  }
  const starts: Record<string, number> = {};
  for (const def of index.needs.values()) {
    const path = `needs.${def.id}`;
    const raw = ownValue(given, def.id);
    const amount = raw === undefined ? def.default : readNumber(raw, path);
    checkRange(amount, def.min, def.max, path);
    starts[def.id] = amount;
  }
  return starts;
}

/**
 * Reads the starting experience of every profession: the document's, or 0. It may not pass the last
 * threshold of the profession's curve.
 */
function readStartExperience(index: SchemaIndex, value: unknown): Record<string, number> {
  const given: Record<string, unknown> = value === undefined ? {} : readRecord(value, "professions");
  for (const key of Object.keys(given)) {
    if (!index.professions.has(key)) {
      throw new CharacterError("unknown-profession", `professions.${key} names no profession of the schema`);
    }
  }
  const starts: Record<string, number> = {};
  for (const def of index.professions.values()) {
    const path = `professions.${def.id}`;
    const raw = ownValue(given, def.id);
    const amount = raw === undefined ? 0 : readNumber(raw, path);
    checkRange(amount, 0, Math.max(...def.xp_thresholds), path);
    starts[def.id] = amount;
  }
  return starts;
}

/**
 * Reads the starting affinity of the character toward others. A pair that is not listed starts at the
 * schema's default. A character has no affinity with itself.
 */
function readStartAffinity(index: SchemaIndex, id: string, value: unknown): Record<string, number> {
  const given: Record<string, unknown> = value === undefined ? {} : readRecord(value, "relationships");
  const rules = index.schema.relationships;
  const starts: Record<string, number> = {};
  for (const [other, raw] of Object.entries(given)) {
    if (!isId(other)) throw new CharacterError("bad-id", `relationships.${other}: "${other}" is not a character id`);
    if (other === id) throw invalid("relationships: a character has no affinity with itself");
    const path = `relationships.${other}`;
    const amount = readNumber(raw, path);
    checkRange(amount, rules.min, rules.max, path);
    starts[other] = amount;
  }
  return starts;
}
