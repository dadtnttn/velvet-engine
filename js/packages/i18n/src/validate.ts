// Validation for tooling: compares a translation with its source, key by key.

import { makeIssue, type LocaleIssue } from "./format";
import type { Locale } from "./locale";
import { describeHoles, sameHoles } from "./template";

/**
 * Checks a translation against its source. It reports the translation's own load problems (its
 * format, version, language, bad entries), the keys the translation lacks, the keys it has that the
 * source does not, and the entries whose holes differ from the source's.
 *
 * The translation should be built without a reference, so that its holes are checked here, once.
 *
 * @param source - the source language's locale.
 * @param translation - the translated locale.
 * @returns the issues found. An empty list means the translation is complete and sound.
 */
export function validateCatalog(source: Locale, translation: Locale): LocaleIssue[] {
  const language = translation.language;
  const issues: LocaleIssue[] = [...translation.issues];
  for (const key of source.keys()) {
    if (!translation.has(key)) {
      issues.push(
        makeIssue("warning", "missing-key", `${language} has no entry for "${key}", so lookups fall back`, { language, key }),
      );
    }
  }
  for (const key of translation.keys()) {
    const wanted = source.holesOf(key);
    if (wanted === undefined) {
      issues.push(
        makeIssue("warning", "unknown-key", `${language} has "${key}", which the source does not have`, { language, key }),
      );
      continue;
    }
    const got = translation.holesOf(key);
    if (got !== undefined && !sameHoles(wanted, got)) {
      const uses = describeHoles(got);
      const sourceUses = describeHoles(wanted);
      issues.push(
        makeIssue("error", "holes", `${language} "${key}" uses ${uses} where its source uses ${sourceUses}`, { language, key }),
      );
    }
  }
  return issues;
}
