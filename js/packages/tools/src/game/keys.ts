// i18n keys in a game's data. A key that looks like an id (dotted, lower case: `menu.start`) must have an
// entry in the source language; a text key (the source text itself, such as `Hola`) needs none.

import type { DocumentKind } from "./discover";

/** The fields that hold an i18n key in the formats: a card's name, a character's name and labels. */
export const KEY_FIELDS: ReadonlySet<string> = new Set(["name", "label", "title", "reason"]);

/** The fields that hold an i18n key in an achievements definitions file: a name, a description and a hint. */
export const ACHIEVEMENT_KEY_FIELDS: ReadonlySet<string> = new Set(["name", "description", "hint"]);

/**
 * The fields that hold i18n keys in a document of a kind. A kind that holds no keys has none.
 * @param kind - the kind of the document.
 * @returns the field names, or undefined when documents of the kind hold no keys.
 */
export function keyFieldsOfKind(kind: DocumentKind): ReadonlySet<string> | undefined {
  switch (kind) {
    case "stacks-catalog":
    case "character-schema":
    case "character-sheet":
      return KEY_FIELDS;
    case "achievement-definitions":
      return ACHIEVEMENT_KEY_FIELDS;
    default:
      return undefined;
  }
}

/** An id: lower-case segments joined by dots, such as `menu.start` or `card.berry_bush`. */
const ID_KEY = /^[a-z][a-z0-9_-]*(?:\.[a-z0-9_-]+)+$/u;

/**
 * True when a key is an id: lower-case, dotted, with no space or punctuation. Ids need an entry in the
 * source catalogs; other keys are texts that the source does not need to repeat.
 * @param key - the key.
 */
export function isIdKey(key: string): boolean {
  return ID_KEY.test(key);
}

/** One key that a data document uses in a key field. */
export interface KeyField {
  /** Where the field is in the document, such as `cards[2].name` or `stats[0].label`. */
  readonly path: string;
  /** The key as written. */
  readonly key: string;
}

/**
 * Finds the key fields of a JSON document: every string under a field named in `fields`, at any depth.
 * @param json - the parsed document.
 * @param fields - the field names that hold keys. Defaults to KEY_FIELDS.
 */
export function keyFieldsOf(json: unknown, fields: ReadonlySet<string> = KEY_FIELDS): KeyField[] {
  const found: KeyField[] = [];
  const walk = (value: unknown, path: string): void => {
    if (Array.isArray(value)) {
      value.forEach((item: unknown, index: number) => {
        walk(item, `${path}[${index}]`);
      });
      return;
    }
    if (typeof value !== "object" || value === null) return;
    for (const [field, child] of Object.entries(value)) {
      const childPath = path === "" ? field : `${path}.${field}`;
      if (fields.has(field) && typeof child === "string") {
        found.push({ path: childPath, key: child });
      }
      walk(child, childPath);
    }
  };
  walk(json, "");
  return found;
}
