// `check`: reads a game folder, validates every data document with the package that owns its format,
// runs the stacks cases, checks the links between the files, and returns what it found.

import { basename } from "node:path";
import { makeIssue, countIssues, WHOLE_FILE, type Issue } from "../issues";
import { discoverGame, type DocumentKind, type GameDocument } from "../game/discover";
import { NO_HINTS } from "../game/manifest";
import { loadLocales } from "../game/locales";
import { runStacksCase } from "./cases";
import { crossCheck } from "./cross";
import { validateDocuments, type CatalogInfo, type ValidatedDocuments } from "./documents";

/** How many files of each kind the game holds. A file counts even when it has problems. */
export interface CheckCounts {
  readonly catalogs: number;
  readonly cases: number;
  readonly stat_schemas: number;
  readonly characters: number;
  readonly art_manifests: number;
  readonly artprep: number;
  readonly guiones: number;
  readonly locale_files: number;
  readonly languages: number;
}

/** The result of checking a game. */
export interface CheckResult {
  /** The game's id, or the folder's name when the manifest names none. */
  readonly game: string;
  /** The folder that was checked, as given. */
  readonly folder: string;
  /** Every problem found, in the order the checks ran. */
  readonly issues: readonly Issue[];
  readonly counts: CheckCounts;
  readonly errors: number;
  readonly warnings: number;
}

/** The kinds of document that the artprep reader covers. */
const ARTPREP_KINDS: readonly DocumentKind[] = [
  "artprep-profiles",
  "artprep-style",
  "artprep-identity",
  "artprep-shots",
  "artprep-results",
];

/**
 * Counts the documents of each kind.
 * @param documents - the game's documents.
 * @param languages - how many language folders the game has.
 */
function countDocuments(documents: readonly GameDocument[], languages: number): CheckCounts {
  const of = (...kinds: readonly DocumentKind[]): number => documents.filter((doc) => kinds.includes(doc.kind)).length;
  return {
    catalogs: of("stacks-catalog"),
    cases: of("stacks-case"),
    stat_schemas: of("character-schema"),
    characters: of("character-sheet"),
    art_manifests: of("art-manifest"),
    artprep: documents.filter((doc) => ARTPREP_KINDS.includes(doc.kind)).length,
    guiones: of("guion"),
    locale_files: of("locale"),
    languages,
  };
}

/**
 * Runs every stacks case against a catalog. A case uses the catalog in its own folder, or the only
 * catalog of the game. When neither holds, the case is not run and a warning says so.
 * @param validated - the documents that passed.
 * @param documents - the game's documents, for the case documents' JSON.
 */
async function runCases(validated: ValidatedDocuments, documents: readonly GameDocument[]): Promise<Issue[]> {
  const issues: Issue[] = [];
  const cases = documents.filter((doc) => doc.kind === "stacks-case");
  for (const doc of cases) {
    if (validated.catalogs.length === 0) {
      issues.push(makeIssue("error", doc.file, WHOLE_FILE, "no-catalog", "this case needs a stacks catalog in the game"));
      continue;
    }
    const beside = validated.catalogs.filter((info: CatalogInfo) => info.folder === doc.folder);
    const chosen = beside.length === 1 ? beside[0] : validated.catalogs.length === 1 ? validated.catalogs[0] : undefined;
    if (chosen === undefined) {
      issues.push(
        makeIssue(
          "warning",
          doc.file,
          WHOLE_FILE,
          "not-run",
          `the case was not run: the game has ${validated.catalogs.length} catalogs and none is in its folder`,
        ),
      );
      continue;
    }
    issues.push(...(await runStacksCase(doc.file, doc.json, chosen.catalog)));
  }
  return issues;
}

/**
 * Checks a game folder.
 * @param folder - the game folder (the one with velvet-game.json).
 * @returns the problems found and the counts of each kind of file.
 */
export async function checkGame(folder: string): Promise<CheckResult> {
  const discovery = await discoverGame(folder);
  const issues: Issue[] = [...discovery.issues];
  const hints = discovery.info?.hints ?? NO_HINTS;

  const validated = await validateDocuments(discovery.documents);
  issues.push(...validated.issues);

  const locales = loadLocales(discovery.documents, hints);
  issues.push(...locales.layout.issues);
  for (const language of locales.languages) issues.push(...language.issues);

  issues.push(...(await runCases(validated, discovery.documents)));
  issues.push(...crossCheck(validated, discovery.documents, locales));

  const { errors, warnings } = countIssues(issues);
  const game = discovery.info?.id || basename(folder);
  return {
    game,
    folder,
    issues,
    counts: countDocuments(discovery.documents, locales.layout.groups.length),
    errors,
    warnings,
  };
}
