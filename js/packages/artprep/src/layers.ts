// Reads the two layers that describe what a picture is about before the shot is known: the style layer
// of a project (a style document) and the identity layer of one character (an identity document).
// The rules are in docs/formats/ART_PREP.md, "Layers".

import {
  checkKeys,
  CHARACTER_PATTERN,
  ENVELOPE_KEYS,
  PHRASE_LIMIT,
  problemsError,
  readDocument,
  readIdentifier,
  readPhraseList,
  readText,
  readTextList,
  type Issues,
} from "./read";
import type { CharacterIdentity, StyleLayer } from "./types";

/**
 * Parses the text of a style document: the phrases that every picture of the project carries, and the
 * negative phrases of the profiles that take one. Throws ArtPrepError, listing every problem found.
 * @param text - the JSON text of the document.
 */
export function parseStyle(text: string): StyleLayer {
  const document = readDocument(text, "style");
  const issues: Issues = [];
  checkKeys(document, [...ENVELOPE_KEYS, "phrases", "negative"], "", issues);
  const phrases = document.phrases === undefined ? [] : readPhraseList(document.phrases, "phrases", issues, 60);
  const negative = document.negative === undefined ? [] : readTextList(document.negative, "negative", issues, 60, 64);
  if (issues.length > 0) throw problemsError(issues, "style document");
  return { phrases, negative };
}

/**
 * Parses the text of an identity document: the permanent traits of one character. Its `id` is the id of
 * the character in its character sheet. Throws ArtPrepError, listing every problem found.
 * @param text - the JSON text of the document.
 */
export function parseIdentity(text: string): CharacterIdentity {
  const document = readDocument(text, "identity");
  const issues: Issues = [];
  checkKeys(
    document,
    [...ENVELOPE_KEYS, "id", "name", "species", "body", "skin", "hair", "eyes", "outfit", "has", "lacks", "fixed"],
    "",
    issues,
  );
  const id = readIdentifier(document.id, "id", issues, CHARACTER_PATTERN, 64, "character id");
  const name = readText(document.name, "name", issues, 80);
  const species = readOptionalText(document.species, "species", issues);
  const body = readOptionalText(document.body, "body", issues);
  const skin = readOptionalText(document.skin, "skin", issues);
  const hair = readOptionalText(document.hair, "hair", issues);
  const eyes = readOptionalText(document.eyes, "eyes", issues);
  const outfit = readOptionalText(document.outfit, "outfit", issues);
  const has = document.has === undefined ? [] : readPhraseList(document.has, "has", issues, 30);
  const lacks = document.lacks === undefined ? [] : readTextList(document.lacks, "lacks", issues, 30, 64);
  const fixed = document.fixed === undefined ? [] : readPhraseList(document.fixed, "fixed", issues, 30);
  if (issues.length > 0 || id === undefined || name === undefined) {
    throw problemsError(issues, "identity document");
  }
  return { id, name, species, body, skin, hair, eyes, outfit, has, lacks, fixed };
}

/** Reads an optional line of text. Absent gives null; a wrong value is recorded and also gives null. */
function readOptionalText(value: unknown, path: string, issues: Issues): string | null {
  if (value === undefined) return null;
  return readText(value, path, issues, PHRASE_LIMIT) ?? null;
}
