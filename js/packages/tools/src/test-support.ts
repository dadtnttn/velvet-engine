// Helpers for the tests of this package: temporary folders that are removed after each test, a copy of
// a game, and the example that the checks must accept. Not part of the package's API.

import { cp, mkdir, mkdtemp, readFile, rm, stat, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { afterEach } from "vitest";
import { ENGINE_JS_DIR } from "./paths";

/** The hello-cards example: a complete game that every check must accept. */
export const HELLO_CARDS: string = resolve(ENGINE_JS_DIR, "examples", "hello-cards");

/** The folders made by the current test, removed when it ends. */
const made: string[] = [];

afterEach(async () => {
  while (made.length > 0) {
    const folder = made.pop();
    if (folder !== undefined) await rm(folder, { recursive: true, force: true });
  }
});

/**
 * Makes an empty folder in the system's temporary folder. It is removed after the test that asked for it.
 */
export async function tempFolder(): Promise<string> {
  const folder = await mkdtemp(join(tmpdir(), "velvet-js-test-"));
  made.push(folder);
  return folder;
}

/**
 * Writes text files, creating their folders. A value that is not a string is written as JSON.
 * @param root - the folder the paths are relative to.
 * @param files - the file contents, by path relative to the root.
 */
export async function writeFiles(root: string, files: Readonly<Record<string, unknown>>): Promise<void> {
  for (const [path, value] of Object.entries(files)) {
    const target = join(root, path);
    await mkdir(dirname(target), { recursive: true });
    const text = typeof value === "string" ? value : `${JSON.stringify(value, null, 2)}\n`;
    await writeFile(target, text, "utf8");
  }
}

/**
 * Copies the parts of a game (by default its manifest, its public data and its source) into a new folder.
 * @param from - the game to copy.
 * @param to - the folder to copy into; it is made if needed.
 * @param parts - the paths to copy, relative to the game.
 */
export async function copyGame(
  from: string,
  to: string,
  parts: readonly string[] = ["velvet-game.json", "public", "src"],
): Promise<void> {
  await mkdir(to, { recursive: true });
  for (const part of parts) {
    try {
      await stat(join(from, part));
    } catch {
      continue;
    }
    await cp(join(from, part), join(to, part), { recursive: true });
  }
}

/**
 * Reads a text file of a folder.
 * @param folder - the folder.
 * @param path - the file, relative to the folder.
 */
export async function readText(folder: string, path: string): Promise<string> {
  return readFile(join(folder, path), "utf8");
}

/**
 * True when a path exists on disk.
 * @param path - the path.
 */
export async function exists(path: string): Promise<boolean> {
  try {
    await stat(path);
    return true;
  } catch {
    return false;
  }
}
