// Validates each data document with the package that reads its format. A document that fails gives
// one issue per problem the reader reports; a document that passes is kept for the cross-checks.

import { ArtError, parseArtManifest, type ArtManifest } from "@velvet/assets";
import { ArtPrepError, parseIdentity, parseProfiles, parseResults, parseShots, parseStyle, type CharacterIdentity, type Shot } from "@velvet/artprep";
import { CharacterError, parseCharacter, parseStatSchema, type CharacterDefinition, type StatSchema } from "@velvet/characters";
import { CardsError, loadCatalog, type Catalog } from "@velvet/cards";
import { GuionLibrary } from "@velvet/guion";
import { makeIssue, WHOLE_FILE, type Issue } from "../issues";
import { compareText, type GameDocument } from "../game/discover";

/** A stacks catalog that passed: its file, its folder and the checked catalog. */
export interface CatalogInfo {
  readonly file: string;
  readonly folder: string;
  readonly catalog: Catalog;
}

/** A stat schema that passed. */
export interface SchemaInfo {
  readonly file: string;
  readonly folder: string;
  readonly schema: StatSchema;
}

/** A character sheet that passed against one of the game's schemas. */
export interface SheetInfo {
  readonly file: string;
  readonly definition: CharacterDefinition;
}

/** An art manifest that passed. */
export interface ArtInfo {
  readonly file: string;
  readonly manifest: ArtManifest;
}

/** An identity layer that passed. */
export interface IdentityInfo {
  readonly file: string;
  readonly identity: CharacterIdentity;
}

/** A shots document that passed. */
export interface ShotsInfo {
  readonly file: string;
  readonly shots: readonly Shot[];
}

/** Everything the validators checked, with the documents that passed, and the problems found. */
export interface ValidatedDocuments {
  readonly issues: Issue[];
  readonly catalogs: readonly CatalogInfo[];
  /** The stacks case documents. They are run after the catalogs are known. */
  readonly cases: readonly GameDocument[];
  readonly schemas: readonly SchemaInfo[];
  readonly sheets: readonly SheetInfo[];
  readonly art: readonly ArtInfo[];
  readonly identities: readonly IdentityInfo[];
  readonly shots: readonly ShotsInfo[];
}

/**
 * The code of a failure: the reader's own code when it has one, otherwise "internal" (a bug, or a
 * module that could not load).
 * @param error - what was thrown.
 */
export function codeOf(error: unknown): string {
  const code = typeof error === "object" && error !== null ? Reflect.get(error, "code") : undefined;
  return typeof code === "string" ? code : "internal";
}

/**
 * The message of a failure.
 * @param error - what was thrown.
 */
export function messageOf(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

/**
 * The issues of a failure from a reader. A reader that lists its problems gives one issue for each;
 * any other failure gives one issue with the code and the message.
 * @param file - the document's file.
 * @param error - what the reader threw.
 */
function failureIssues(file: string, error: unknown): Issue[] {
  if (error instanceof ArtError || error instanceof ArtPrepError) {
    return error.issues.map((item) =>
      makeIssue("error", file, item.path ?? WHOLE_FILE, item.code, item.message),
    );
  }
  if (error instanceof CardsError || error instanceof CharacterError) {
    return [makeIssue("error", file, WHOLE_FILE, error.code, error.message)];
  }
  return [makeIssue("error", file, WHOLE_FILE, codeOf(error), messageOf(error))];
}

/**
 * Validates the data documents of a game. Locale catalogs are not here: `loadLocales` reads them.
 * @param documents - the game's documents.
 * @returns the documents that passed, grouped by kind, and every problem found.
 */
export async function validateDocuments(documents: readonly GameDocument[]): Promise<ValidatedDocuments> {
  const issues: Issue[] = [];
  const catalogs: CatalogInfo[] = [];
  const cases: GameDocument[] = [];
  const schemas: SchemaInfo[] = [];
  const sheetDocuments: GameDocument[] = [];
  const art: ArtInfo[] = [];
  const identities: IdentityInfo[] = [];
  const shots: ShotsInfo[] = [];
  const guiones: GameDocument[] = [];

  for (const doc of documents) {
    const text = doc.text;
    try {
      switch (doc.kind) {
        case "stacks-catalog":
          catalogs.push({ file: doc.file, folder: doc.folder, catalog: await loadCatalog(text) });
          break;
        case "stacks-case":
          cases.push(doc);
          break;
        case "character-schema":
          schemas.push({ file: doc.file, folder: doc.folder, schema: parseStatSchema(text) });
          break;
        case "character-sheet":
          sheetDocuments.push(doc);
          break;
        case "art-manifest":
          art.push({ file: doc.file, manifest: parseArtManifest(text) });
          break;
        case "artprep-profiles":
          parseProfiles(text);
          break;
        case "artprep-style":
          parseStyle(text);
          break;
        case "artprep-identity":
          identities.push({ file: doc.file, identity: parseIdentity(text) });
          break;
        case "artprep-shots":
          shots.push({ file: doc.file, shots: parseShots(text) });
          break;
        case "artprep-results":
          parseResults(text);
          break;
        case "guion":
          guiones.push(doc);
          break;
        case "locale":
          break;
      }
    } catch (error) {
      issues.push(...failureIssues(doc.file, error));
    }
  }

  const sheets = checkSheets(sheetDocuments, schemas, issues);
  issues.push(...(await checkGuiones(guiones)));
  return { issues, catalogs, cases, schemas, sheets, art, identities, shots };
}

/**
 * Reads each character sheet against the game's schemas. A sheet is tried first against the schemas
 * in its own folder, then against the others; it passes when one of them reads it.
 * @param documents - the character sheets.
 * @param schemas - the schemas that passed.
 * @param issues - where a problem is reported.
 */
function checkSheets(documents: readonly GameDocument[], schemas: readonly SchemaInfo[], issues: Issue[]): SheetInfo[] {
  const passed: SheetInfo[] = [];
  for (const doc of documents) {
    if (schemas.length === 0) {
      issues.push(
        makeIssue("error", doc.file, WHOLE_FILE, "no-schema", "this character sheet needs a stat schema (kind \"schema\") in the game"),
      );
      continue;
    }
    const preferred = [...schemas].sort((a, b) => {
      const rank = (folder: string): number => (folder === doc.folder ? 0 : 1);
      return rank(a.folder) - rank(b.folder) || compareText(a.file, b.file);
    });
    let definition: CharacterDefinition | undefined;
    let firstFailure: unknown;
    for (const schema of preferred) {
      try {
        definition = parseCharacter(doc.text, schema.schema);
        break;
      } catch (error) {
        firstFailure ??= error;
      }
    }
    if (definition !== undefined) {
      passed.push({ file: doc.file, definition });
    } else {
      issues.push(...failureIssues(doc.file, firstFailure));
    }
  }
  return passed;
}

/**
 * Reads every guion with one library, so that the problems of each file are found and the ids that two
 * files share are reported too.
 * @param documents - the guion documents.
 */
async function checkGuiones(documents: readonly GameDocument[]): Promise<Issue[]> {
  const issues: Issue[] = [];
  if (documents.length === 0) return issues;
  const library = await GuionLibrary.create();
  const fileOfId = new Map<string, string>();
  for (const doc of documents) {
    const summary = library.add(doc.name, doc.text);
    for (const problem of summary.problems) {
      issues.push(makeIssue("error", doc.file, `line ${problem.line}`, "guion", problem.message));
    }
    const first = fileOfId.get(summary.id);
    if (first === undefined) {
      fileOfId.set(summary.id, doc.file);
    } else {
      issues.push(makeIssue("error", doc.file, WHOLE_FILE, "guion-id", `the id "${summary.id}" is also used by ${first}`));
    }
  }
  return issues;
}
