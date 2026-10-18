// Finds the text keys that a game's code uses. It is a scanner, not a parser: it reads quoted strings
// and the first argument of t(), plural() and lookup() calls. A key that the code builds at run time
// (`t(\`recipe.${name}\`)`) is recorded as a prefix, so every key that starts with it counts as used.

/** One call with a literal key: `t("menu.start")`. */
export interface KeyCall {
  /** The key, as the call names it. */
  readonly key: string;
  /** The file that makes the call, relative to the game folder. */
  readonly file: string;
}

/** What a scan of the code found. */
export interface CodeScan {
  /** Every quoted string in the code, as written. A key that appears in any string counts as used. */
  readonly strings: ReadonlySet<string>;
  /** Every call with a literal key. */
  readonly calls: readonly KeyCall[];
  /** The start of each key that is built at run time, such as `recipe.` for `t(\`recipe.${name}\`)`. */
  readonly prefixes: readonly string[];
}

/** The start of a text-translation call: `t(`, `plural(` or `lookup(`, as a word of its own. */
const CALL = /\b(?:t|plural|lookup)\(\s*/gu;

/**
 * The start of a key that is built at run time: lower-case segments ending in a dot, such as `done.` or
 * `pause.fullscreen.`. A text that merely begins with a word is not a prefix.
 */
const PREFIX = /^[a-z][a-z0-9_.-]*\.$/u;

/**
 * Reads a quoted literal that starts at `start` (the quote character is at `start`).
 * @param source - the file's text.
 * @param start - the index of the opening quote.
 * @returns the text between the quotes, as written, and the index after the closing quote, or undefined
 *   when the literal is not closed on its line.
 */
function readQuoted(source: string, start: number): { text: string; end: number } | undefined {
  const quote = source.charAt(start);
  let index = start + 1;
  while (index < source.length) {
    const char = source.charAt(index);
    if (char === "\\") {
      index += 2;
      continue;
    }
    if (char === quote) return { text: source.slice(start + 1, index), end: index + 1 };
    if (char === "\n" && quote !== "`") return undefined;
    index += 1;
  }
  return undefined;
}

/**
 * Scans the text of code files.
 * @param files - the files to scan: their path (for the calls) and their text.
 * @returns the strings, the literal calls and the run-time prefixes.
 */
export function scanCode(files: readonly { readonly file: string; readonly text: string }[]): CodeScan {
  const strings = new Set<string>();
  const calls: KeyCall[] = [];
  const prefixes = new Set<string>();

  for (const { file, text } of files) {
    // Every quoted string, so that a key held in a table or a property counts as used. A template
    // with a hole gives its start as a prefix (`done.${recipe}` gives `done.`), wherever it is written.
    for (let index = 0; index < text.length; index += 1) {
      const char = text.charAt(index);
      if (char !== '"' && char !== "'" && char !== "`") continue;
      const literal = readQuoted(text, index);
      if (literal === undefined) continue;
      if (literal.text.includes("${")) {
        const head = literal.text.slice(0, literal.text.indexOf("${"));
        if (PREFIX.test(head)) prefixes.add(head);
      } else {
        strings.add(literal.text);
        if (PREFIX.test(literal.text)) prefixes.add(literal.text);
      }
      index = literal.end - 1;
    }

    // The first argument of each translation call: a literal key, or the start of a dynamic one.
    for (const match of text.matchAll(CALL)) {
      const at = (match.index ?? 0) + match[0].length;
      const quote = text.charAt(at);
      if (quote !== '"' && quote !== "'" && quote !== "`") continue;
      const literal = readQuoted(text, at);
      if (literal === undefined || literal.text === "") continue;
      if (literal.text.includes("${")) continue;
      const after = text.slice(literal.end).match(/^\s*(.)/u)?.[1];
      if (after === "," || after === ")") {
        calls.push({ key: literal.text, file });
      } else if (after === "+" && PREFIX.test(literal.text)) {
        prefixes.add(literal.text);
      }
    }
  }
  return { strings, calls, prefixes: [...prefixes].sort() };
}
