// Pseudo-locales for layout tests. A pseudo-text keeps every hole and brace escape of its source, swaps
// each ASCII letter for an accented look-alike, and grows by about 35% with a run of "~", inside
// brackets. Its text is easy to spot on screen, and a label that is too narrow for 35% more text shows
// at once. It is never a translation.

import { parseTemplate, type EntryValue, type PluralForms } from "@velvet/i18n";

/** The note that every pseudo-locale file carries, so that a later run knows the files are its own. */
export const PSEUDO_NOTE = "velvet-js pseudo-locale: generated for layout tests. Do not translate or edit by hand.";

/** The growth of a pseudo-text, as a fraction of the source length. */
export const PSEUDO_GROWTH = 0.35;

/** The filler that makes a text longer. */
const FILLER = "~";

/** The accented look-alike of each ASCII letter, in alphabetical order. */
const LOWER_ACCENTS = "áƀçđéƒğĥíĵķĺṁñóṕǫŕšţúṽŵẋýž";
const UPPER_ACCENTS = "ÁƁÇĐÉƑĞĤÍĴĶĹṀÑÓṔǪŔŠŢÚṼŴẊÝŽ";
const ACCENTS: ReadonlyMap<string, string> = new Map([
  ...[...LOWER_ACCENTS].map((accent, index): [string, string] => [String.fromCharCode(97 + index), accent]),
  ...[...UPPER_ACCENTS].map((accent, index): [string, string] => [String.fromCharCode(65 + index), accent]),
]);

/** One piece of a template as written: literal text, or a hole. */
interface Piece {
  readonly kind: "text" | "hole";
  /** The piece as it is written in the source, so that the pseudo-text keeps the same syntax. */
  readonly written: string;
}

/**
 * Splits a template into its pieces, keeping the escapes (`{{`, `}}`) and the holes as written.
 * @param source - a template that the i18n package accepts.
 */
function pieces(source: string): Piece[] {
  const result: Piece[] = [];
  let text = "";
  let index = 0;
  const flush = (): void => {
    if (text !== "") result.push({ kind: "text", written: text });
    text = "";
  };
  while (index < source.length) {
    const char = source.charAt(index);
    const next = source.charAt(index + 1);
    if (char === "{" && next === "{") {
      text += "{{";
      index += 2;
    } else if (char === "}" && next === "}") {
      text += "}}";
      index += 2;
    } else if (char === "{") {
      // A valid template closes every hole; the guard only stops a malformed input from looping.
      const close = source.indexOf("}", index);
      if (close < 0) break;
      flush();
      result.push({ kind: "hole", written: source.slice(index, close + 1) });
      index = close + 1;
    } else {
      text += char;
      index += 1;
    }
  }
  flush();
  return result;
}

/**
 * The pseudo-text of one template. A text with no letter of its own (only holes, digits or punctuation)
 * is returned as it is, because it has nothing to translate.
 * @param source - the template, as the catalog writes it.
 * @returns the pseudo-text, or undefined when the source is not a valid template.
 */
export function pseudoText(source: string): string | undefined {
  const parsed = parseTemplate(source);
  if (!parsed.ok) return undefined;
  const parts = pieces(source);
  const letters = parts.some((part) => part.kind === "text" && /\p{L}/u.test(part.written));
  if (!letters) return source;

  const body = parts
    .map((part) => (part.kind === "hole" ? part.written : [...part.written].map((char) => ACCENTS.get(char) ?? char).join("")))
    .join("");
  const filler = FILLER.repeat(Math.max(1, Math.ceil(source.length * PSEUDO_GROWTH)));
  return `[${body}${filler}]`;
}

/**
 * The pseudo-value of one entry: a template, or the forms of a plural entry.
 * @param value - the entry's value as the catalog writes it.
 * @returns the pseudo-value, or undefined when a form is not a valid template.
 */
export function pseudoValue(value: EntryValue): EntryValue | undefined {
  if (typeof value === "string") return pseudoText(value);
  const other = pseudoText(value.other);
  if (other === undefined) return undefined;
  const result: { -readonly [Category in keyof PluralForms]: PluralForms[Category] } = { other };
  for (const category of ["zero", "one", "two", "few", "many"] as const) {
    const form = value[category];
    if (form === undefined) continue;
    const pseudo = pseudoText(form);
    if (pseudo === undefined) return undefined;
    result[category] = pseudo;
  }
  return result;
}
