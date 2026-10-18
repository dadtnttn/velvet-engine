// Where things live on disk: the js/ folder of the engine, its packages, and the game template.
// Everything is found from this file's own location, so the tools work from any working folder.

import { relative, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";

/** The engine's js/ folder: the workspace that holds packages/ and examples/. */
export const ENGINE_JS_DIR: string = resolve(fileURLToPath(new URL("../../../", import.meta.url)));

/** The folder of the engine's packages (packages/core, packages/ui, and so on). */
export const PACKAGES_DIR: string = resolve(ENGINE_JS_DIR, "packages");

/** The folder of this package (packages/tools): the bin, the source and the templates. */
export const TOOLS_DIR: string = resolve(PACKAGES_DIR, "tools");

/** The folder of the game template that `velvet-js new` copies. */
export const TEMPLATE_DIR: string = resolve(TOOLS_DIR, "templates", "game");

/**
 * Writes a path with forward slashes, the form that JSON, URLs and the file lists of the tools use.
 * @param path - a path in the platform's form.
 */
export function toPosix(path: string): string {
  return path.split(sep).join("/");
}

/**
 * The path from one folder to another, with forward slashes. A path on another drive stays absolute.
 * @param from - the folder the path starts in.
 * @param to - the folder or file it ends at.
 */
export function relativePosix(from: string, to: string): string {
  return toPosix(relative(from, to));
}
