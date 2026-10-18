// The save file format: the envelope that every slot holds, and the migrations that bring
// older save data up to date.
//
// An envelope is JSON text. Its shape is
//   { "format": "velvet-save", "version": 2, "game": "my-game",
//     "savedAt": "2026-10-18T10:00:00.000Z", "data": { ... } }
// The format tag and the version let a reader recognise the file and decide what to do with it.

import { isPositiveInteger, isRecord } from "./guards";
import { SaveError } from "./save-error";

/** The format tag that every save envelope carries. */
export const SAVE_FORMAT = "velvet-save";

/** Turns save data of one version into data of the next version. */
export type Migration = (data: unknown) => unknown;

/**
 * Migrations keyed by the version they start from: `migrations[1]` turns version 1 data
 * into version 2 data.
 */
export type MigrationMap = Readonly<Record<number, Migration>>;

/** The contents of one save slot. */
export interface SaveEnvelope {
  readonly format: typeof SAVE_FORMAT;
  /** The version of the save data. */
  readonly version: number;
  /** The game that wrote the save. */
  readonly game: string;
  /** When the save was written, as an ISO 8601 string. */
  readonly savedAt: string;
  /** The game's own save data. */
  readonly data: unknown;
}

/**
 * Writes an envelope as JSON text.
 * @throws SaveError with code "bad-data" when the data is undefined, a function or a symbol,
 * or cannot be written as JSON (for example a BigInt or an object that refers to itself).
 */
export function encodeEnvelope(envelope: SaveEnvelope): string {
  const { data } = envelope;
  if (data === undefined || typeof data === "function" || typeof data === "symbol") {
    throw new SaveError("bad-data", "save data must be a JSON value; undefined, functions and symbols are not");
  }
  try {
    return JSON.stringify(envelope);
  } catch (cause) {
    throw new SaveError("bad-data", "save data cannot be written as JSON", { cause });
  }
}

/**
 * Reads an envelope from JSON text.
 * @throws SaveError with code "corrupt" when the text is not JSON, or is not a save envelope.
 */
export function decodeEnvelope(text: string): SaveEnvelope {
  let parsed: unknown;
  try {
    parsed = JSON.parse(text);
  } catch (cause) {
    throw new SaveError("corrupt", "save is not valid JSON", { cause });
  }
  if (!isRecord(parsed)) throw new SaveError("corrupt", "save is not a JSON object");
  const { format, version, game, savedAt } = parsed;
  if (
    format !== SAVE_FORMAT ||
    !isPositiveInteger(version) ||
    typeof game !== "string" ||
    typeof savedAt !== "string" ||
    !("data" in parsed)
  ) {
    throw new SaveError("corrupt", `save is not a ${SAVE_FORMAT} envelope`);
  }
  return { format: SAVE_FORMAT, version, game, savedAt, data: parsed.data };
}

/**
 * Applies the migrations from version `from` up to version `to`, one step at a time.
 * @throws SaveError with code "missing-migration" when one of the steps has no migration.
 */
export function migrateData(data: unknown, from: number, to: number, migrations: MigrationMap): unknown {
  let current = data;
  for (let version = from; version < to; version += 1) {
    const migrate = migrations[version];
    if (migrate === undefined) {
      throw new SaveError("missing-migration", `no migration from save version ${version} to ${version + 1}`);
    }
    current = migrate(current);
  }
  return current;
}
