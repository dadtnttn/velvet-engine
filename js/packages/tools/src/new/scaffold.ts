// Scaffolds a new game from the template in templates/game: it fills the placeholders, computes the
// link: dependencies from the folder of the game to the engine's packages, and writes the files.

import { mkdir, readdir, readFile, stat, writeFile } from "node:fs/promises";
import { dirname, join, relative, resolve } from "node:path";
import { gameIdProblem, titleProblem } from "../game/rules";
import { relativePosix, TEMPLATE_DIR, toPosix } from "../paths";

/** What the scaffold needs: the folder, the game's id and title, the engine, and whether to overwrite. */
export interface ScaffoldOptions {
  /** The folder of the new game, as given. It is created when it does not exist. */
  readonly folder: string;
  /** The game's id: it names the game's saves and its manifest id. */
  readonly id: string;
  /** The title that the window and the page show. */
  readonly title: string;
  /** The engine's js folder: the one that holds packages/ and tsconfig.base.json. */
  readonly engine: string;
  /** True to write into a folder that is not empty, and to overwrite files with the same names. */
  readonly force: boolean;
}

/** What the scaffold wrote. */
export interface ScaffoldResult {
  /** The folder, as the caller gave it. */
  readonly folder: string;
  /** The files written, relative to the folder, with forward slashes, in order. */
  readonly files: readonly string[];
}

/** The engine packages that a new game links. The tools package is linked too, for its check script. */
const LINKED_PACKAGES: readonly string[] = ["core", "i18n", "ui"];

/** The packages that must exist in the engine folder for it to count as the engine's js folder. */
const REQUIRED_PACKAGES: readonly string[] = ["core", "i18n", "ui", "tools"];

/**
 * Escapes a text for a place inside a JSON string or a double-quoted JavaScript string.
 * @param text - the text.
 */
export function jsonText(text: string): string {
  return JSON.stringify(text).slice(1, -1);
}

/**
 * Escapes a text for HTML element content and attribute values.
 * @param text - the text.
 */
export function htmlText(text: string): string {
  return text
    .replace(/&/gu, "&amp;")
    .replace(/</gu, "&lt;")
    .replace(/>/gu, "&gt;")
    .replace(/"/gu, "&quot;")
    .replace(/'/gu, "&#39;");
}

/**
 * Fills the `{{NAME}}` placeholders of a template. A placeholder with no value is a bug in the template,
 * so it is an error rather than a silent blank.
 * @param text - the template's text.
 * @param values - the value of each placeholder, already escaped for its place.
 */
export function renderTemplate(text: string, values: Readonly<Record<string, string>>): string {
  return text.replace(/\{\{([A-Z][A-Z0-9_]*)\}\}/gu, (_match: string, name: string) => {
    const value = values[name];
    if (value === undefined) throw new Error(`the template uses {{${name}}}, which the scaffold does not fill`);
    return value;
  });
}

/**
 * Lists the template's files, with the path each one is written to. A file named `_gitignore` is
 * written as `.gitignore`, because package managers drop dot files from a package, not from a template.
 * @param root - the template folder.
 */
async function templateFiles(root: string): Promise<{ readonly source: string; readonly target: string }[]> {
  const found: { source: string; target: string }[] = [];
  const walk = async (folder: string): Promise<void> => {
    const entries = await readdir(folder, { withFileTypes: true });
    for (const entry of entries.sort((a, b) => (a.name < b.name ? -1 : a.name > b.name ? 1 : 0))) {
      const abs = join(folder, entry.name);
      if (entry.isDirectory()) {
        await walk(abs);
        continue;
      }
      const target = toPosix(relative(root, abs)).replace(/(^|\/)_gitignore$/u, "$1.gitignore");
      found.push({ source: abs, target });
    }
  };
  await walk(root);
  return found;
}

/**
 * Checks that a folder is the js folder of the engine: it holds the packages that a game links.
 * @param engine - the folder to check, absolute.
 * @returns the problem, or undefined when the folder is the engine's js folder.
 */
async function engineProblem(engine: string): Promise<string | undefined> {
  for (const name of REQUIRED_PACKAGES) {
    const manifest = join(engine, "packages", name, "package.json");
    try {
      await stat(manifest);
    } catch {
      return `${engine} is not the js folder of the Velvet engine: ${toPosix(relative(engine, manifest))} is missing`;
    }
  }
  return undefined;
}

/**
 * Checks that the target folder can take a new game: it is missing, or an empty folder, or --force is set.
 * @param folder - the target, absolute.
 * @param shown - the folder as the caller gave it.
 * @param force - whether --force was given.
 */
async function targetProblem(folder: string, shown: string, force: boolean): Promise<string | undefined> {
  let entries: string[];
  try {
    const info = await stat(folder);
    if (!info.isDirectory()) return `${shown} is a file, not a folder`;
    entries = await readdir(folder);
  } catch {
    return undefined;
  }
  if (entries.length > 0 && !force) {
    return `${shown} is not empty: choose an empty folder, or pass --force to write into it`;
  }
  return undefined;
}

/**
 * Writes a new game folder from the template.
 * @param options - the folder, the id, the title, the engine and the overwrite flag.
 * @returns the files written.
 * @throws Error with a sentence for the user when an option is not valid, the engine is not found, or
 *   the folder cannot take the game.
 */
export async function scaffoldGame(options: ScaffoldOptions): Promise<ScaffoldResult> {
  const idProblem = gameIdProblem(options.id);
  if (idProblem !== undefined) throw new Error(idProblem);
  const problem = titleProblem(options.title);
  if (problem !== undefined) throw new Error(`the title is not valid: ${problem}`);

  const engine = resolve(options.engine);
  const missing = await engineProblem(engine);
  if (missing !== undefined) throw new Error(missing);

  const folder = resolve(options.folder);
  const blocked = await targetProblem(folder, options.folder, options.force);
  if (blocked !== undefined) throw new Error(blocked);

  const values: Record<string, string> = {
    GAME_ID: options.id,
    GAME_TITLE: options.title,
    GAME_TITLE_JSON: jsonText(options.title),
    GAME_TITLE_HTML: htmlText(options.title),
    ENGINE_REPO: dirname(engine),
    ENGINE_RELATIVE_JSON: jsonText(relativePosix(folder, engine)),
  };
  for (const name of LINKED_PACKAGES) {
    values[`LINK_${name.toUpperCase()}_JSON`] = jsonText(`link:${relativePosix(folder, join(engine, "packages", name))}`);
  }

  const files = await templateFiles(TEMPLATE_DIR);
  const written: string[] = [];
  for (const file of files) {
    const text = renderTemplate(await readFile(file.source, "utf8"), values);
    const destination = join(folder, file.target);
    await mkdir(dirname(destination), { recursive: true });
    await writeFile(destination, text, "utf8");
    written.push(file.target);
  }
  return { folder: options.folder, files: written };
}
