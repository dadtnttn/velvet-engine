// A roster: the characters of a game, found by id and by tag, with the affinity queries between them.
// Every list that a roster returns is sorted by id, so the same roster always answers the same way.

import { Character } from "./character";
import { CharacterError } from "./error";
import { levelPosition } from "./schema";
import type { CharacterState, Opinion, Relation, StatSchema } from "./types";
import { compareText, isRecord } from "./util";

/** Options of the affinity queries. */
export interface RelationQuery {
  /**
   * Keeps only the affinities at this level or higher. The name must be a level of the schema.
   */
  readonly minLevel?: string;
}

/** A character's state, read for a restore, with the character it belongs to and the state it had before. */
interface PendingRestore {
  readonly character: Character;
  readonly state: unknown;
  readonly before: CharacterState;
}

/** The characters of a game, by id. */
export class Roster {
  private readonly characters = new Map<string, Character>();
  private readonly schema: StatSchema | undefined;

  /**
   * @param characters - the characters. Their ids must differ, and they must share one schema object.
   * @throws {CharacterError} with code "duplicate-id", or "argument" when the schemas differ.
   */
  constructor(characters: Iterable<Character>) {
    let schema: StatSchema | undefined;
    for (const character of characters) {
      if (this.characters.has(character.id)) {
        throw new CharacterError("duplicate-id", `the roster lists "${character.id}" twice`);
      }
      if (schema === undefined) {
        schema = character.schema;
      } else if (character.schema !== schema) {
        throw new CharacterError("argument", `"${character.id}" follows another schema; a roster needs one schema`);
      }
      this.characters.set(character.id, character);
    }
    this.schema = schema;
  }

  /** The character with this id, or undefined when the roster has none. */
  get(id: string): Character | undefined {
    return this.characters.get(id);
  }

  /** Every character, sorted by id. */
  all(): Character[] {
    return [...this.characters.values()].sort((a, b) => compareText(a.id, b.id));
  }

  /** The characters that carry the tag, sorted by id. */
  byTag(tag: string): Character[] {
    return this.all().filter((character) => character.tags.includes(tag));
  }

  /**
   * The affinities that a character holds toward others, sorted by the other's id. Only the pairs that
   * hold a value are listed; the others are at the default.
   *
   * @throws {CharacterError} with code "unknown-character", or "unknown-level" for a `minLevel` the schema lacks.
   */
  relationsOf(id: string, query: RelationQuery = {}): Relation[] {
    const character = this.require(id);
    const keep = this.levelFilter(query);
    return character.affinities().filter((relation) => keep(relation.value));
  }

  /**
   * The affinities that other roster members hold toward a character, sorted by their ids.
   *
   * @throws {CharacterError} with code "unknown-character", or "unknown-level" for a `minLevel` the schema lacks.
   */
  opinionsOf(id: string, query: RelationQuery = {}): Opinion[] {
    this.require(id);
    const keep = this.levelFilter(query);
    const opinions: Opinion[] = [];
    for (const character of this.all()) {
      if (character.id === id) continue;
      const relation = character.affinities().find((entry) => entry.other === id);
      if (relation !== undefined && keep(relation.value)) {
        opinions.push({ from: character.id, value: relation.value, level: relation.level });
      }
    }
    return opinions;
  }

  /** The states of every character, sorted by id. Each one is what Character.snapshot writes. */
  snapshot(): CharacterState[] {
    return this.all().map((character) => character.snapshot());
  }

  /**
   * Restores the states of some characters. A character that has no state in the list keeps its values.
   * Either every state is applied, or none is: when one is refused, the characters keep their values.
   *
   * @param states - a list of states, as `snapshot` writes them.
   * @throws {CharacterError} with code "invalid" (not a list, or a state without an id), "duplicate-id",
   *   "unknown-character", or the code of the state that was refused.
   */
  restore(states: unknown): void {
    if (!Array.isArray(states)) {
      throw new CharacterError("invalid", "the roster state must be a list of character states");
    }
    const list: readonly unknown[] = states;
    const seen = new Set<string>();
    const pending: PendingRestore[] = list.map((state): PendingRestore => {
      const id = isRecord(state) ? state["id"] : undefined;
      if (typeof id !== "string") throw new CharacterError("invalid", "every character state must have an id");
      if (seen.has(id)) throw new CharacterError("duplicate-id", `the roster state lists "${id}" twice`);
      seen.add(id);
      const character = this.require(id);
      return { character, state, before: character.snapshot() };
    });
    try {
      for (const { character, state } of pending) character.restore(state);
    } catch (error) {
      for (const { character, before } of pending) character.restore(before);
      throw error;
    }
  }

  /** The character with this id. @throws {CharacterError} with code "unknown-character". */
  private require(id: string): Character {
    const character = this.characters.get(id);
    if (character === undefined) throw new CharacterError("unknown-character", `the roster has no character "${id}"`);
    return character;
  }

  /** A test for affinity values: every value passes when the query names no level. */
  private levelFilter(query: RelationQuery): (value: number) => boolean {
    if (query.minLevel === undefined) return () => true;
    const rules = this.schema?.relationships;
    const position = rules?.levels.findIndex((level) => level.id === query.minLevel) ?? -1;
    if (rules === undefined || position < 0) {
      throw new CharacterError("unknown-level", `the schema has no level "${query.minLevel}"`);
    }
    return (value) => levelPosition(rules.levels, value) >= position;
  }
}
