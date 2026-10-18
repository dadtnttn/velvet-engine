// Helpers shared by the tests of this package. Not part of the package's API.

import { readFile, readdir } from "node:fs/promises";
import { expect } from "vitest";
import { CharacterError, type CharacterErrorCode } from "./error";

/** The folder of the shared character fixtures, found from this file (src, characters, packages, js, root). */
const FIXTURES = new URL("../../../../docs/formats/fixtures/characters/", import.meta.url);

/** Reads a file of the shared fixtures, by its path inside the folder (for example `sheets/mara.json`). */
export async function readFixture(path: string): Promise<string> {
  return readFile(new URL(path, FIXTURES), "utf8");
}

/** The names of the files in a folder of the shared fixtures, sorted by name. */
export async function fixtureFiles(folder: string): Promise<string[]> {
  const names = await readdir(new URL(`${folder}/`, FIXTURES));
  return names.filter((name) => name.endsWith(".json")).sort();
}

/**
 * Asserts that `call` throws a CharacterError whose code is `code`.
 * @param call - the call that should throw.
 * @param code - the code that the error must carry.
 */
export function expectCode(call: () => unknown, code: CharacterErrorCode): void {
  let thrown: unknown;
  try {
    call();
  } catch (error) {
    thrown = error;
  }
  expect(thrown, `expected a ${code} error`).toBeInstanceOf(CharacterError);
  expect(thrown).toMatchObject({ code });
}
