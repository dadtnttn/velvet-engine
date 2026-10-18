// The links between a game's documents: the art ids and variants that the sheets and the shots name must
// be in an art manifest, the characters that a shot shows must have an identity, and the keys that the
// data uses must be in the source language. Each reader checks its own file; these checks look across files.

import type { ArtEntry } from "@velvet/assets";
import { makeIssue, WHOLE_FILE, type Issue } from "../issues";
import type { GameDocument } from "../game/discover";
import { isIdKey, keyFieldsOf } from "../game/keys";
import type { LoadedLocales } from "../game/locales";
import type { ValidatedDocuments } from "./documents";

/**
 * Checks the links across the documents of a game.
 * @param validated - the documents that passed their readers.
 * @param documents - every data document, for the key fields.
 * @param locales - the loaded languages. The source language decides which keys exist.
 * @returns the problems found, as errors, and one warning when the game has no source language.
 */
export function crossCheck(
  validated: ValidatedDocuments,
  documents: readonly GameDocument[],
  locales: LoadedLocales,
): Issue[] {
  const issues: Issue[] = [];

  const entries = new Map<string, ArtEntry>();
  for (const art of validated.art) {
    for (const entry of art.manifest.entries) {
      if (entries.has(entry.id)) {
        issues.push(makeIssue("error", art.file, WHOLE_FILE, "duplicate-art-id", `the art id "${entry.id}" is declared twice`));
      } else {
        entries.set(entry.id, entry);
      }
    }
  }

  const checkArt = (file: string, path: string, id: string, variant: string | null): void => {
    const entry = entries.get(id);
    if (entry === undefined) {
      issues.push(makeIssue("error", file, path, "unknown-art-id", `the art id "${id}" is not declared by any art manifest`));
      return;
    }
    if (variant !== null && !entry.variants.includes(variant)) {
      issues.push(makeIssue("error", file, path, "unknown-variant", `"${id}" has no variant "${variant}" in its art manifest`));
    }
  };

  for (const sheet of validated.sheets) {
    const art = sheet.definition.art;
    if (art.portrait !== undefined) checkArt(sheet.file, "art.portrait", art.portrait, null);
    if (art.fullbody !== undefined) checkArt(sheet.file, "art.fullbody", art.fullbody, null);
  }

  const identities = new Set(validated.identities.map((info) => info.identity.id));
  for (const shots of validated.shots) {
    shots.shots.forEach((shot, index) => {
      const at = `shots[${index}]`;
      checkArt(shots.file, `${at}.art`, shot.art, shot.variant);
      shot.references.forEach((reference, position) => {
        checkArt(shots.file, `${at}.references[${position}]`, reference.id, reference.variant);
      });
      if (shot.edit !== null) {
        checkArt(shots.file, `${at}.edit.base`, shot.edit.base.id, shot.edit.base.variant);
      }
      shot.characters.forEach((character, position) => {
        if (!identities.has(character)) {
          issues.push(
            makeIssue(
              "error",
              shots.file,
              `${at}.characters[${position}]`,
              "unknown-character",
              `"${character}" has no identity document in the game`,
            ),
          );
        }
      });
    });
  }

  const source = locales.source;
  let reportedNoSource = false;
  for (const doc of documents) {
    if (doc.kind !== "stacks-catalog" && doc.kind !== "character-schema" && doc.kind !== "character-sheet") continue;
    for (const field of keyFieldsOf(doc.json)) {
      if (!isIdKey(field.key)) continue;
      if (source === undefined) {
        if (!reportedNoSource) {
          reportedNoSource = true;
          issues.push(
            makeIssue(
              "warning",
              doc.file,
              WHOLE_FILE,
              "no-source-locale",
              "the game has no locale catalogs, so the id keys of its data are not checked",
            ),
          );
        }
        continue;
      }
      if (!source.locale.has(field.key)) {
        issues.push(
          makeIssue(
            "error",
            doc.file,
            field.path,
            "unknown-key",
            `"${field.key}" has no entry in the source language (${source.tag}), so the game would show the key itself`,
          ),
        );
      }
    }
  }
  return issues;
}
