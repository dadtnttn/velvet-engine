// The `i18n` command: reports, for each language of a game, the keys it lacks, the keys nothing uses and the
// texts left as the source wrote them. With --pseudo it writes a pseudo-locale for layout tests.

import { readFile } from "node:fs/promises";
import { basename, join, resolve } from "node:path";
import { isLanguageTag } from "@velvet/i18n";
import { UsageError, type CommandSpec } from "../command";
import { countIssues, formatIssue, makeIssue, WHOLE_FILE, type Issue } from "../issues";
import { discoverGame, type GameDiscovery, type GameDocument } from "../game/discover";
import { keyFieldsOf } from "../game/keys";
import { NO_HINTS } from "../game/manifest";
import { loadLocales, type LoadedLocales } from "../game/locales";
import type { Output } from "../output";
import { coverageOf, textKeysOf, type LanguageCoverage } from "./analyze";
import { scanCode, type CodeScan } from "./scan";
import { PseudoError, writePseudoLocale } from "./write";

/** The extensions of the files that the code scan reads. */
const CODE_EXTENSIONS: ReadonlySet<string> = new Set([".ts", ".tsx", ".js", ".jsx", ".mjs", ".cjs", ".mts", ".cts", ".html"]);

/** The documents whose key fields name texts of the game. */
const DATA_KINDS: ReadonlySet<GameDocument["kind"]> = new Set(["stacks-catalog", "character-schema", "character-sheet"]);

/** The result of reporting on a game's texts. */
interface TextReport {
  readonly game: string;
  readonly discovery: GameDiscovery;
  readonly locales: LoadedLocales;
  readonly scan: CodeScan;
  readonly languages: readonly LanguageCoverage[];
  readonly issues: Issue[];
}

/**
 * True for a file of the code that the scan reads: a source or markup file, but not a test.
 * @param name - the file's name.
 */
function isCodeFile(name: string): boolean {
  const dot = name.lastIndexOf(".");
  if (dot < 0 || !CODE_EXTENSIONS.has(name.slice(dot))) return false;
  return !name.endsWith(".d.ts") && !/\.(?:test|spec)\./u.test(name);
}

/**
 * Reads the code files of a discovery and scans them for keys.
 * @param discovery - the game's discovery, with the files it visited.
 */
async function scanGameCode(discovery: GameDiscovery): Promise<CodeScan> {
  const files: { file: string; text: string }[] = [];
  for (const file of discovery.files) {
    if (!isCodeFile(file.name)) continue;
    files.push({ file: file.file, text: await readFile(file.abs, "utf8") });
  }
  return scanCode(files);
}

/**
 * The keys that the data names in its key fields, from the documents that hold them.
 * @param documents - the game's documents.
 */
function dataKeysOf(documents: readonly GameDocument[]): Set<string> {
  const keys = new Set<string>();
  for (const doc of documents) {
    if (!DATA_KINDS.has(doc.kind)) continue;
    for (const field of keyFieldsOf(doc.json)) keys.add(field.key);
  }
  return keys;
}

/**
 * Reads the game and works out the coverage of its languages.
 * @param folder - the game folder.
 */
async function reportGame(folder: string): Promise<TextReport> {
  const discovery = await discoverGame(folder);
  const hints = discovery.info?.hints ?? NO_HINTS;
  const locales = loadLocales(discovery.documents, hints);
  const scan = await scanGameCode(discovery);
  const coverage = coverageOf(locales, scan, dataKeysOf(discovery.documents));
  const issues: Issue[] = [...discovery.issues, ...coverage.issues];
  if (locales.languages.length === 0) {
    issues.push(
      makeIssue("warning", ".", WHOLE_FILE, "no-locales", "the game has no locale folder, so there are no texts to report on"),
    );
  }
  return {
    game: discovery.info?.id || basename(folder),
    discovery,
    locales,
    scan,
    languages: coverage.languages,
    issues,
  };
}

/** The human summary line of one language. */
function languageLine(language: LanguageCoverage): string {
  const name = language.isSource ? `${language.tag} (source)` : language.tag;
  return `${name}: ${language.missing.length} missing, ${language.unused.length} unused, ${language.untranslated.length} untranslated`;
}

/**
 * Writes the pseudo-locale, after the same checks as the report.
 * @param folder - the game folder.
 * @param tag - the pseudo-locale's tag.
 * @param output - where the result goes.
 * @param json - true for the JSON form.
 */
async function writePseudo(folder: string, tag: string, output: Output, json: boolean): Promise<number> {
  if (!isLanguageTag(tag)) {
    throw new UsageError(`--pseudo needs a language tag in canonical case, such as en-XA: "${tag}" is not one`);
  }
  const report = await reportGame(folder);
  const source = report.locales.source;
  if (source === undefined) {
    throw new PseudoError("the game has no source language, so there is no text to make a pseudo-locale from");
  }
  if (source.tag === tag) {
    throw new PseudoError(`${tag} is the source language: a pseudo-locale needs a tag of its own`);
  }
  const textKeys = textKeysOf(report.scan, source);
  const written = await writePseudoLocale(folder, source, tag, textKeys);
  const destination = join(folder, written.folder).split("\\").join("/");
  if (json) {
    output.out(
      JSON.stringify(
        { ok: true, tag, folder: written.folder, keys: written.keys, skipped: written.skipped },
        null,
        2,
      ),
    );
  } else {
    output.out(`wrote ${written.keys} texts to ${destination}/00_pseudo.json`);
    for (const key of written.skipped) output.out(`skipped "${key}": its text is not a valid template`);
  }
  return 0;
}

/** The `i18n` command. */
export const i18nCommand: CommandSpec = {
  name: "i18n",
  summary: "report each language's missing, unused and untranslated keys; --pseudo writes a test language",
  usage: "<game dir> [--pseudo <tag>]",
  options: [
    {
      name: "pseudo",
      takesValue: true,
      valueName: "<tag>",
      help: "write a pseudo-locale with this tag (for example en-XA) for layout tests",
    },
  ],
  details: [
    "The source language is the one with every id: es when the game has it, else the first language folder.",
    "Missing: a key of the source (or a text the code shows) that a language has no entry for.",
    "Unused: a key that no string, call, prefix of a run-time key or data field of the game uses.",
    "Untranslated: a text that is the same in a language and in the source. Warnings do not fail the command;",
    "an error does (a key the code asks for as an id that the source lacks, or a broken catalog).",
  ],
  async run(args, output) {
    if (args.positional.length !== 1) {
      throw new UsageError("give one game folder, such as my-game");
    }
    const folder = resolve(args.positional[0] ?? "");
    const json = args.flags.has("json");
    const pseudo = args.values.get("pseudo");
    if (pseudo !== undefined) return writePseudo(folder, pseudo, output, json);

    const report = await reportGame(folder);
    const { errors, warnings } = countIssues(report.issues);
    const verdict = errors === 0 ? "ok" : "failed";
    if (json) {
      output.out(
        JSON.stringify(
          {
            ok: errors === 0,
            game: report.game,
            folder: args.positional[0],
            source: report.locales.source?.tag ?? null,
            languages: report.languages,
            issues: report.issues,
            errors,
            warnings,
          },
          null,
          2,
        ),
      );
    } else {
      for (const issue of report.issues) output.out(formatIssue(issue));
      for (const language of report.languages) output.out(languageLine(language));
      output.out(`${report.game}: ${verdict}, ${errors} error${errors === 1 ? "" : "s"}, ${warnings} warning${warnings === 1 ? "" : "s"}`);
    }
    return errors === 0 ? 0 : 1;
  },
};
