// The console's parts, without the DOM: word splitting with quotes, typed argument parsing, completion of
// command names and of choices, the history of lines that were run, and the output buffer.

import type { ArgSpec, CommandSpec, CommandValue, CommandValues } from "./types";

/** Lines of history kept. */
export const HISTORY_LIMIT = 100;

/** Output lines kept. The oldest go first. */
export const OUTPUT_LIMIT = 300;

/** What a console line is, which decides how it looks. */
export type ConsoleKind = "input" | "output" | "error";

/** One line of console output. */
export interface ConsoleLine {
  readonly kind: ConsoleKind;
  readonly text: string;
}

/** The result of Tab completion: the new text of the line, and the candidates when there are several. */
export interface Completion {
  readonly text: string;
  readonly options: readonly string[];
}

/** Words that mean true for a boolean argument. */
const TRUE_WORDS: ReadonlySet<string> = new Set(["true", "on", "yes", "1"]);

/** Words that mean false for a boolean argument. */
const FALSE_WORDS: ReadonlySet<string> = new Set(["false", "off", "no", "0"]);

/** A command name: a letter, then letters, digits, "-" or "_". */
const COMMAND_NAME = /^[A-Za-z][A-Za-z0-9_-]*$/;

/** True when a name can be used for a command. */
export function isValidCommandName(name: string): boolean {
  return COMMAND_NAME.test(name);
}

/**
 * Splits a line into words. Whitespace separates words, and double quotes keep a run of words together
 * as one. A quote that is never closed is an error.
 * @throws Error when a quote is not closed.
 */
export function tokenize(line: string): string[] {
  const words: string[] = [];
  let current = "";
  let inWord = false;
  let quoted = false;
  for (const character of line) {
    if (quoted) {
      if (character === '"') quoted = false;
      else current += character;
    } else if (character === '"') {
      quoted = true;
      inWord = true;
    } else if (/\s/.test(character)) {
      if (inWord) {
        words.push(current);
        current = "";
        inWord = false;
      }
    } else {
      current += character;
      inWord = true;
    }
  }
  if (quoted) throw new Error("missing closing quote");
  if (inWord) words.push(current);
  return words;
}

/**
 * Converts the words after a command name into argument values, one per spec.
 * @throws Error, with a message for the console, when there are too many words, a required argument is missing,
 * or a word has the wrong type.
 */
export function parseArgs(specs: readonly ArgSpec[], words: readonly string[]): CommandValues {
  if (words.length > specs.length) {
    throw new Error(
      specs.length === 0 ? "this command takes no arguments" : `too many arguments: at most ${specs.length}`,
    );
  }
  const values: Record<string, CommandValue> = {};
  for (const [index, spec] of specs.entries()) {
    const word = words[index];
    if (word === undefined) {
      if (spec.optional !== true) throw new Error(`missing <${spec.name}>`);
      continue;
    }
    values[spec.name] = convertWord(spec, word);
  }
  return values;
}

/** The usage line of a command: its name, then its arguments, required ones in angle brackets. */
export function usageOf(name: string, args: readonly ArgSpec[]): string {
  const parts = args.map((arg) => {
    const kind = arg.choices === undefined ? arg.type : arg.choices.join("|");
    return arg.optional === true ? `[${arg.name}:${kind}]` : `<${arg.name}:${kind}>`;
  });
  return [name, ...parts].join(" ");
}

/**
 * Completes the line at its end. A line with no space completes a command name. A line with a space
 * completes the argument that is being typed, when that argument has choices.
 *
 * One match gives the whole word and a trailing space. Several matches give their common start, and
 * all of them are listed in `options`.
 */
export function completeLine(commands: ReadonlyMap<string, CommandSpec>, line: string): Completion {
  const body = line.trimStart();
  if (!/\s/.test(body)) {
    const names = [...commands.keys()].sort();
    return finish("", body, names.filter((name) => name.startsWith(body)));
  }

  // The words that are complete, with the command name first. A trailing space starts a new argument.
  const complete = body.trimEnd();
  const words = complete.split(/\s+/);
  const endsWithSpace = complete.length < body.length;
  const partial = endsWithSpace ? "" : (words[words.length - 1] ?? "");
  const argIndex = endsWithSpace ? words.length - 1 : words.length - 2;
  const spec = commands.get(words[0] ?? "");
  const arg = spec?.args?.[argIndex];
  if (arg?.choices === undefined) return { text: line, options: [] };

  const head = line.slice(0, line.length - partial.length);
  return finish(head, partial, arg.choices.filter((choice) => choice.startsWith(partial)));
}

/**
 * The history of lines that were run, with Up and Down to browse them. The line being typed (the draft)
 * is kept while browsing, and comes back after the newest line.
 */
export class ConsoleHistory {
  private readonly capacity: number;
  private readonly lines: string[] = [];
  private cursor: number | null = null;
  private draft = "";

  /** @param capacity - how many lines are kept. The oldest go first. */
  constructor(capacity: number) {
    this.capacity = capacity;
  }

  /** The number of lines kept. */
  get size(): number {
    return this.lines.length;
  }

  /** Remembers a line that was run. Blank lines, and a line that repeats the newest one, are not kept. */
  add(line: string): void {
    this.cursor = null;
    const text = line.trim();
    if (text === "" || this.lines[this.lines.length - 1] === text) return;
    this.lines.push(text);
    const excess = this.lines.length - this.capacity;
    if (excess > 0) this.lines.splice(0, excess);
  }

  /** Stops browsing, so that the next Up starts again from the newest line. */
  reset(): void {
    this.cursor = null;
  }

  /**
   * The line before the current one. The first Up keeps the draft. Returns null when there is no history.
   * At the oldest line it stays there.
   */
  previous(draft: string): string | null {
    if (this.lines.length === 0) return null;
    if (this.cursor === null) {
      this.draft = draft;
      this.cursor = this.lines.length - 1;
    } else if (this.cursor > 0) {
      this.cursor -= 1;
    }
    return this.lines[this.cursor] ?? null;
  }

  /**
   * The line after the current one. Past the newest line, the draft comes back. Returns null when the
   * history is not being browsed.
   */
  next(): string | null {
    if (this.cursor === null) return null;
    if (this.cursor < this.lines.length - 1) {
      this.cursor += 1;
      return this.lines[this.cursor] ?? null;
    }
    this.cursor = null;
    return this.draft;
  }
}

/** The console's output lines, capped, with a revision that changes on every change. */
export class ConsoleOutput {
  private readonly capacity: number;
  private items: ConsoleLine[] = [];
  private changes = 0;

  /** @param capacity - how many lines are kept. The oldest go first. */
  constructor(capacity: number) {
    this.capacity = capacity;
  }

  /** The lines, oldest first. */
  get lines(): readonly ConsoleLine[] {
    return this.items;
  }

  /** Goes up on every change, so that a view can tell when it must redraw. */
  get revision(): number {
    return this.changes;
  }

  /** Adds a line. */
  add(kind: ConsoleKind, text: string): void {
    this.items.push({ kind, text });
    const excess = this.items.length - this.capacity;
    if (excess > 0) this.items = this.items.slice(excess);
    this.changes += 1;
  }

  /** Removes every line. */
  clear(): void {
    this.items = [];
    this.changes += 1;
  }
}

/** Converts one word to the type of its argument. */
function convertWord(spec: ArgSpec, word: string): CommandValue {
  if (spec.type === "number") {
    const value = Number(word);
    if (word.trim() === "" || !Number.isFinite(value)) {
      throw new Error(`${spec.name} must be a number, got "${word}"`);
    }
    return value;
  }
  if (spec.type === "boolean") {
    const lower = word.toLowerCase();
    if (TRUE_WORDS.has(lower)) return true;
    if (FALSE_WORDS.has(lower)) return false;
    throw new Error(`${spec.name} must be on or off, got "${word}"`);
  }
  if (spec.choices !== undefined && !spec.choices.includes(word)) {
    throw new Error(`${spec.name} must be one of ${spec.choices.join(", ")}, got "${word}"`);
  }
  return word;
}

/**
 * Builds the completion from the candidates. `head` is the text before the word being completed, and
 * `partial` is that word. One candidate completes the word and adds a space; several complete to their
 * common start, when that is longer than the word.
 */
function finish(head: string, partial: string, matches: readonly string[]): Completion {
  if (matches.length === 0) return { text: `${head}${partial}`, options: [] };
  if (matches.length === 1) {
    return { text: `${head}${matches[0] ?? partial} `, options: matches };
  }
  const common = commonStart(matches);
  const text = common.length > partial.length ? `${head}${common}` : `${head}${partial}`;
  return { text, options: matches };
}

/** The longest start that all the words share. */
function commonStart(words: readonly string[]): string {
  let start = words[0] ?? "";
  for (const word of words.slice(1)) {
    while (!word.startsWith(start)) start = start.slice(0, -1);
  }
  return start;
}
