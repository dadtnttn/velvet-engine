// Language tags and text direction. A velvet-locale tag has the form language, language-Script,
// language-REGION or language-Script-REGION, in canonical case. docs/formats/LOCALE.md has the rules.

import type { Direction } from "./format";

const TAG = /^[a-z]{2,3}(?:-[A-Z][a-z]{3})?(?:-(?:[A-Z]{2}|[0-9]{3}))?$/u;

/** The primary languages that are written right to left unless a catalog says otherwise. */
const RIGHT_TO_LEFT = new Set(["he", "ar", "fa", "ur"]);

/**
 * True when `tag` is a velvet-locale language tag in canonical case: `pt-BR`, `zh-Hans`, `es-419`.
 * Variants and extensions are not supported, so `de-AT-1996` is not a tag here.
 */
export function isLanguageTag(tag: string): boolean {
  return TAG.test(tag);
}

/**
 * The base of a tag, the tag without its last subtag: `pt-BR` gives `pt`, `zh-Hant-TW` gives
 * `zh-Hant`. A tag with one subtag has no base.
 */
export function baseLanguageTag(tag: string): string | undefined {
  const cut = tag.lastIndexOf("-");
  return cut > 0 ? tag.slice(0, cut) : undefined;
}

/**
 * The direction a language is written in when its catalogs do not say: right to left for he, ar,
 * fa and ur, left to right for the rest. It reads the primary subtag, so `he-IL` is right to left.
 */
export function textDirection(tag: string): Direction {
  const primary = tag.split("-")[0] ?? tag;
  return RIGHT_TO_LEFT.has(primary) ? "rtl" : "ltr";
}
