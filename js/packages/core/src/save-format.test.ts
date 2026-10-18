// Tests for the save envelope: encoding, decoding and the migration steps.

import { describe, expect, it } from "vitest";
import { SaveError } from "./save-error";
import { SAVE_FORMAT, decodeEnvelope, encodeEnvelope, migrateData, type SaveEnvelope } from "./save-format";
import { errorOf } from "./test-support";

const envelope: SaveEnvelope = {
  format: SAVE_FORMAT,
  version: 2,
  game: "demo",
  savedAt: "2026-10-18T10:00:00.000Z",
  data: { gold: 5, flags: ["intro"] },
};

describe("encodeEnvelope and decodeEnvelope", () => {
  it("round-trips an envelope through JSON text", () => {
    expect(decodeEnvelope(encodeEnvelope(envelope))).toEqual(envelope);
  });

  it("keeps null as a valid save value", () => {
    expect(decodeEnvelope(encodeEnvelope({ ...envelope, data: null })).data).toBeNull();
  });

  it("refuses data that is undefined, a function or a symbol", async () => {
    for (const data of [undefined, () => 1, Symbol("not stored")]) {
      expect(await errorOf(() => encodeEnvelope({ ...envelope, data }))).toMatchObject({ code: "bad-data" });
    }
  });

  it("refuses data that JSON cannot represent", async () => {
    expect(await errorOf(() => encodeEnvelope({ ...envelope, data: 10n }))).toMatchObject({ code: "bad-data" });
  });

  it("reports text that is not JSON, or not an envelope, as corrupt", async () => {
    const notEnvelopes = [
      "{nope",
      "[]",
      "42",
      "null",
      JSON.stringify({ ...envelope, format: "other" }),
      JSON.stringify({ ...envelope, version: 0 }),
      JSON.stringify({ ...envelope, version: 1.5 }),
      JSON.stringify({ ...envelope, version: "2" }),
      JSON.stringify({ ...envelope, game: 7 }),
      JSON.stringify({ format: SAVE_FORMAT, version: 1, game: "demo", savedAt: "2026-10-18T10:00:00.000Z" }),
    ];
    for (const text of notEnvelopes) {
      expect(await errorOf(() => decodeEnvelope(text))).toMatchObject({ code: "corrupt" });
    }
  });

  it("keeps the parse error as the cause when the text is not JSON", async () => {
    const error = await errorOf(() => decodeEnvelope("{nope"));
    expect(error).toBeInstanceOf(SaveError);
    expect((error as SaveError).cause).toBeInstanceOf(SyntaxError);
  });
});

describe("migrateData", () => {
  it("applies each step from the saved version up to the target, in order", () => {
    const migrations = {
      1: (data: unknown) => (data as number) + 10,
      2: (data: unknown) => (data as number) * 2,
    };
    expect(migrateData(1, 1, 3, migrations)).toBe(22);
  });

  it("runs only the steps between the two versions", () => {
    const migrations = {
      1: () => 100,
      2: (data: unknown) => (data as number) + 1,
    };
    expect(migrateData(1, 2, 3, migrations)).toBe(2);
  });

  it("returns the data unchanged when the versions already match", () => {
    expect(migrateData({ a: 1 }, 2, 2, {})).toEqual({ a: 1 });
  });

  it("reports the first step that has no migration", async () => {
    const error = await errorOf(() => migrateData(1, 1, 3, { 2: (data) => data }));
    expect(error).toMatchObject({ code: "missing-migration" });
  });
});
