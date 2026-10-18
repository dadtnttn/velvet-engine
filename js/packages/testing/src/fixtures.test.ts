// The replay conformance fixtures (docs/formats/fixtures/replay). Each case in cases.json names a file, says
// whether the reader must accept it, and for a refusal gives the start of the message. The same files are
// the reference for any other reader of the format, so this test only checks what REPLAY.md says.

import { readFile, readdir } from "node:fs/promises";
import { describe, expect, it } from "vitest";
import { TestingError } from "./errors";
import { parseReplayText, serializeReplay } from "./replay-format";

/** The folder of the replay fixtures, found from this file (src, testing, packages, js, root). */
const FIXTURES = new URL("../../../../docs/formats/fixtures/replay/", import.meta.url);

/** One case of the fixtures index. */
interface FixtureCase {
  readonly file: string;
  readonly valid: boolean;
  readonly error?: string;
}

/** The fixture cases, read from the index. */
async function cases(): Promise<FixtureCase[]> {
  const text = await readFile(new URL("cases.json", FIXTURES), "utf8");
  return JSON.parse(text) as FixtureCase[];
}

describe("the replay fixtures", () => {
  it("has a case for every file in the folder", async () => {
    const indexed = (await cases()).map((item) => item.file).sort();
    const found: string[] = [];
    for (const folder of ["valid", "invalid"]) {
      const names = await readdirSafe(folder);
      for (const name of names) found.push(`${folder}/${name}`);
    }
    expect(indexed).toEqual(found.sort());
  });

  it("accepts every valid case, and reads back to the same replay", async () => {
    const valid = (await cases()).filter((item) => item.valid);
    expect(valid.length).toBeGreaterThan(0);
    for (const item of valid) {
      const text = await readFile(new URL(item.file, FIXTURES), "utf8");
      const replay = parseReplayText(text);
      expect(parseReplayText(serializeReplay(replay)), item.file).toEqual(replay);
    }
  });

  it("refuses every invalid case with the bad-replay code and the message that the index names", async () => {
    const invalid = (await cases()).filter((item) => !item.valid);
    expect(invalid.length).toBeGreaterThan(0);
    for (const item of invalid) {
      const text = await readFile(new URL(item.file, FIXTURES), "utf8");
      let refused: unknown = null;
      try {
        parseReplayText(text);
      } catch (error) {
        refused = error;
      }
      expect(refused, item.file).toBeInstanceOf(TestingError);
      const error = refused as TestingError;
      expect(error.code, item.file).toBe("bad-replay");
      expect(error.message.startsWith(item.error ?? "?"), `${item.file}: ${error.message}`).toBe(true);
    }
  });
});

/** The names in a folder of the fixtures, sorted. */
async function readdirSafe(folder: string): Promise<string[]> {
  return (await readdir(new URL(`${folder}/`, FIXTURES))).sort();
}
