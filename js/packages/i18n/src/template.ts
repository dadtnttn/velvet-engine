// Templates: an entry's text split at its holes, and the filling of those holes.
//
// `{0}` and `{gold}` are holes. `{{` and `}}` write a literal brace, and any other brace is a
// syntax error. docs/formats/LOCALE.md has the rules, under "Template syntax".

import type { LocaleArgs, LocaleValue } from "./format";

/** A piece of a template: literal text, or a hole with the name of its argument. */
export type TemplatePart =
  | { readonly kind: "text"; readonly text: string }
  | { readonly kind: "hole"; readonly name: string };

/** A parsed template. `holes` holds each hole name once. */
export interface Template {
  readonly parts: readonly TemplatePart[];
  readonly holes: ReadonlySet<string>;
}

/** The outcome of parsing: the template, or the reason the text is not a valid template. */
export type TemplateParse =
  | { readonly ok: true; readonly template: Template }
  | { readonly ok: false; readonly message: string };

/** The outcome of filling a template: the text, and the holes that had no argument. */
export interface RenderedTemplate {
  readonly text: string;
  readonly missing: readonly string[];
}

const POSITIONAL = /^(?:0|[1-9][0-9]*)$/u;
const NAMED = /^[A-Za-z_][A-Za-z0-9_]*$/u;

/** True for a valid hole name: a canonical position (`0`, `12`) or a name (`gold`). */
export function isHoleName(name: string): boolean {
  return POSITIONAL.test(name) || NAMED.test(name);
}

/**
 * Parses a template. It never throws: a syntax error comes back as `ok: false`.
 * @param source - the text of an entry, or a text key.
 */
export function parseTemplate(source: string): TemplateParse {
  const parts: TemplatePart[] = [];
  const holes = new Set<string>();
  let text = "";
  let index = 0;
  while (index < source.length) {
    const char = source.charAt(index);
    const next = source.charAt(index + 1);
    if (char === "{") {
      if (next === "{") {
        text += "{";
        index += 2;
        continue;
      }
      const close = source.indexOf("}", index + 1);
      if (close < 0) {
        return failure(`the { at position ${index} is never closed`);
      }
      const name = source.slice(index + 1, close);
      if (!isHoleName(name)) {
        return failure(`{${name}} is not a hole: write a position such as {0} or a name such as {gold}`);
      }
      if (text !== "") {
        parts.push({ kind: "text", text });
        text = "";
      }
      parts.push({ kind: "hole", name });
      holes.add(name);
      index = close + 1;
      continue;
    }
    if (char === "}") {
      if (next === "}") {
        text += "}";
        index += 2;
        continue;
      }
      return failure(`the } at position ${index} has no { before it`);
    }
    text += char;
    index += 1;
  }
  if (text !== "") {
    parts.push({ kind: "text", text });
  }
  return { ok: true, template: { parts, holes } };
}

/**
 * Fills a template's holes.
 * @param template - a parsed template.
 * @param lookup - the value of a hole by its name, or undefined when there is none. A hole with
 * no value is written as `{name}` and listed in `missing`.
 */
export function renderTemplate(
  template: Template,
  lookup: (name: string) => LocaleValue | undefined,
): RenderedTemplate {
  let text = "";
  const missing: string[] = [];
  for (const part of template.parts) {
    if (part.kind === "text") {
      text += part.text;
      continue;
    }
    const value = lookup(part.name);
    if (value === undefined) {
      text += `{${part.name}}`;
      if (!missing.includes(part.name)) {
        missing.push(part.name);
      }
    } else {
      text += String(value);
    }
  }
  return { text, missing };
}

/**
 * The value that `args` gives a hole. A list gives its items to the positional holes; an object
 * gives its keys to the holes of the same names. Returns undefined when there is no value.
 */
export function argumentValue(args: LocaleArgs | undefined, name: string): LocaleValue | undefined {
  if (args === undefined) {
    return undefined;
  }
  if (isPositionalArgs(args)) {
    return POSITIONAL.test(name) ? args[Number(name)] : undefined;
  }
  return Object.hasOwn(args, name) ? args[name] : undefined;
}

/** True when two sets of hole names hold the same names. */
export function sameHoles(a: ReadonlySet<string>, b: ReadonlySet<string>): boolean {
  if (a.size !== b.size) {
    return false;
  }
  for (const name of a) {
    if (!b.has(name)) {
      return false;
    }
  }
  return true;
}

/** Hole names written as a sorted list, for messages: `{0, gold}`. */
export function describeHoles(names: ReadonlySet<string>): string {
  return `{${[...names].sort().join(", ")}}`;
}

function failure(message: string): TemplateParse {
  return { ok: false, message };
}

function isPositionalArgs(args: LocaleArgs): args is readonly LocaleValue[] {
  return Array.isArray(args);
}
