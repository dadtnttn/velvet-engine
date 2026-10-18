// Reads a shots document: the pictures to make. Each shot names the art id (and variant) it fills, the
// characters that it shows, its references, its variation and, for an edit, its base and its one change.
// The rules that depend on a profile are checked when the jobs are built (docs/formats/ART_PREP.md,
// "Shots").

import {
  addIssue,
  ART_ID_LIMIT,
  ART_ID_PATTERN,
  CHARACTER_PATTERN,
  checkKeys,
  ENVELOPE_KEYS,
  isMissing,
  joinPath,
  problemsError,
  readArray,
  readChoice,
  readDocument,
  readIdentifier,
  readIdentifierList,
  readObject,
  readOptionalVariant,
  readPhrase,
  SLUG_PATTERN,
  type Issues,
} from "./read";
import {
  VARIATION_FIELDS,
  type ArtRef,
  type Phrase,
  type Shot,
  type ShotEdit,
  type ShotVariation,
  type VariationField,
} from "./types";

/** The most shots one document may hold. */
const MAX_SHOTS = 500;

/** The most characters one shot may name. A profile allows fewer; the build checks that. */
const MAX_CHARACTERS = 8;

/** The most references one shot may name. A profile allows fewer; the build checks that. */
const MAX_REFERENCES = 8;

/**
 * Parses the text of a shots document. The shots come in the order of the file. Throws ArtPrepError,
 * listing every problem found.
 * @param text - the JSON text of the document.
 */
export function parseShots(text: string): readonly Shot[] {
  const document = readDocument(text, "shots");
  const issues: Issues = [];
  checkKeys(document, [...ENVELOPE_KEYS, "shots"], "", issues);
  const list = readArray(document.shots, "shots", issues, MAX_SHOTS) ?? [];
  const shots: Shot[] = [];
  const ids = new Set<string>();
  list.forEach((item, index) => {
    const path = `shots[${index}]`;
    const shot = readShot(item, path, issues);
    if (shot === undefined) return;
    if (ids.has(shot.id)) addIssue(issues, "duplicate", `shot "${shot.id}" is listed twice`, `${path}.id`);
    ids.add(shot.id);
    shots.push(shot);
  });
  if (issues.length > 0) throw problemsError(issues, "shots document");
  return shots;
}

/** Reads one shot. Returns undefined when its id or its art id is wrong; the problem is recorded. */
function readShot(value: unknown, path: string, issues: Issues): Shot | undefined {
  const object = readObject(value, path, issues);
  if (object === undefined) return undefined;
  checkKeys(object, ["id", "art", "variant", "characters", "references", "variation", "edit"], path, issues);
  const id = readIdentifier(object.id, joinPath(path, "id"), issues, SLUG_PATTERN, 64, "shot id");
  const art = readIdentifier(object.art, joinPath(path, "art"), issues, ART_ID_PATTERN, ART_ID_LIMIT, "art id");
  const variant = readOptionalVariant(object.variant, joinPath(path, "variant"), issues);
  const characters =
    object.characters === undefined
      ? []
      : readIdentifierList(
          object.characters,
          joinPath(path, "characters"),
          issues,
          MAX_CHARACTERS,
          CHARACTER_PATTERN,
          64,
          "character id",
        );
  const references =
    object.references === undefined ? [] : readRefList(object.references, joinPath(path, "references"), issues);
  const variation =
    object.variation === undefined ? {} : readVariation(object.variation, joinPath(path, "variation"), issues);
  const edit =
    object.edit === undefined || object.edit === null
      ? null
      : (readEdit(object.edit, joinPath(path, "edit"), issues) ?? null);
  if (id === undefined || art === undefined) return undefined;
  return { id, art, variant, characters, references, variation, edit };
}

/** Reads a list of references. */
function readRefList(value: unknown, path: string, issues: Issues): ArtRef[] {
  const list = readArray(value, path, issues, MAX_REFERENCES);
  const refs: ArtRef[] = [];
  list?.forEach((item, index) => {
    const ref = readRef(item, `${path}[${index}]`, issues);
    if (ref !== undefined) refs.push(ref);
  });
  return refs;
}

/** Reads one reference: an art id as a string, or an object with an `id` and an optional `variant`. */
function readRef(value: unknown, path: string, issues: Issues): ArtRef | undefined {
  if (typeof value === "string") {
    const id = readIdentifier(value, path, issues, ART_ID_PATTERN, ART_ID_LIMIT, "art id");
    return id === undefined ? undefined : { id, variant: null };
  }
  const object = readObject(value, path, issues);
  if (object === undefined) return undefined;
  checkKeys(object, ["id", "variant"], path, issues);
  const id = readIdentifier(object.id, joinPath(path, "id"), issues, ART_ID_PATTERN, ART_ID_LIMIT, "art id");
  const variant = readOptionalVariant(object.variant, joinPath(path, "variant"), issues);
  return id === undefined ? undefined : { id, variant };
}

/** Reads the variation of a shot. Only the names of the variation fields are allowed. */
function readVariation(value: unknown, path: string, issues: Issues): ShotVariation {
  const object = readObject(value, path, issues);
  if (object === undefined) return {};
  checkKeys(object, [...VARIATION_FIELDS], path, issues);
  const variation: Partial<Record<VariationField, Phrase>> = {};
  for (const field of VARIATION_FIELDS) {
    if (object[field] === undefined) continue;
    const phrase = readPhrase(object[field], joinPath(path, field), issues);
    if (phrase !== undefined) variation[field] = phrase;
  }
  return variation;
}

/** Reads the edit of an edit shot: its base picture, and the one variation field that it changes. */
function readEdit(value: unknown, path: string, issues: Issues): ShotEdit | undefined {
  const object = readObject(value, path, issues);
  if (object === undefined) return undefined;
  checkKeys(object, ["base", "change"], path, issues);
  const base = isMissing(object.base, joinPath(path, "base"), issues)
    ? undefined
    : readRef(object.base, joinPath(path, "base"), issues);
  const change = readChoice(object.change, joinPath(path, "change"), issues, VARIATION_FIELDS);
  if (base === undefined || change === undefined) return undefined;
  return { base, change };
}
