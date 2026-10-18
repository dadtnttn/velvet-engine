// Save slots for a game: versioned envelopes kept in a backend.
//
// A slot holds one envelope: { format, version, game, savedAt, data } as JSON text. `version`
// is the version of the game's save data. When data is loaded, the migrations bring older
// data up to date: migrations[n] turns version-n data into version n + 1, applied in order.
// A save from a newer build than this one is refused, not guessed at.
//
// The store writes through the first backend it finds: the Velvet Desktop bridge (the desktop
// release), then localStorage (browser development), then memory. A backend can also be
// passed in, for tests or a custom host.

import { getBridge, isValidSlot } from "./bridge";
import {
  createBridgeBackend,
  createLocalStorageBackend,
  createMemoryBackend,
  getLocalStorage,
  type SaveBackend,
} from "./save-backends";
import { SaveError } from "./save-error";
import { SAVE_FORMAT, decodeEnvelope, encodeEnvelope, migrateData, type MigrationMap } from "./save-format";

/** Which storage a store uses: the bridge, localStorage, memory, or a backend that was passed in. */
export type SaveBackendKind = "bridge" | "localStorage" | "memory" | "custom";

/** Options for {@link createSaveStore}. */
export interface SaveStoreOptions {
  /** The game's id. Saves that belong to another game are refused. */
  game: string;
  /** The current version of the save data: a positive integer. */
  version: number;
  /** Migrations keyed by the version they start from. See {@link MigrationMap}. */
  migrations?: MigrationMap;
  /** Storage to use instead of the automatic choice. */
  backend?: SaveBackend;
}

/** A save as returned by `load`. */
export interface LoadedSave {
  /** The slot that the save came from. */
  readonly slot: string;
  /** When the save was written, as an ISO 8601 string. */
  readonly savedAt: string;
  /** The store's current version. The data has already been migrated to it. */
  readonly version: number;
  /** The save data, migrated to the current version. The game should check its shape. */
  readonly data: unknown;
}

/** Versioned save slots. */
export interface SaveStore {
  /** The game id that this store reads and writes. */
  readonly game: string;
  /** The current version of the save data. */
  readonly version: number;
  /** The storage in use. */
  readonly backendKind: SaveBackendKind;
  /** The names of the slots that hold saves, sorted. */
  list(): Promise<string[]>;
  /**
   * Writes `data` to `slot`, replacing any save there. The data must be JSON-serializable.
   * @throws SaveError with code "bad-slot" or "bad-data".
   */
  save(slot: string, data: unknown): Promise<void>;
  /**
   * Reads `slot`. Resolves to null when the slot is empty.
   * @throws SaveError with code "bad-slot", "corrupt", "wrong-game", "newer-version" or
   * "missing-migration".
   */
  load(slot: string): Promise<LoadedSave | null>;
  /** Deletes `slot`. Deleting an empty slot is not an error. */
  remove(slot: string): Promise<void>;
}

/**
 * Creates a save store.
 *
 * @throws RangeError when the game id is empty, the version is not a positive integer, or a
 * migration key is not a version from 1 to version - 1.
 */
export function createSaveStore(options: SaveStoreOptions): SaveStore {
  const { game, version } = options;
  const migrations = options.migrations ?? {};
  validateOptions(game, version, migrations);
  const chosen = chooseBackend(game, options.backend);
  const backend = chosen.backend;

  return {
    game,
    version,
    backendKind: chosen.kind,

    async list() {
      return (await backend.list()).filter((slot) => isValidSlot(slot)).sort();
    },

    async save(slot, data) {
      assertSlot(slot);
      const text = encodeEnvelope({
        format: SAVE_FORMAT,
        version,
        game,
        savedAt: new Date().toISOString(),
        data,
      });
      await backend.write(slot, text);
    },

    async load(slot) {
      assertSlot(slot);
      const text = await backend.read(slot);
      if (text === null) return null;
      const envelope = decodeEnvelope(text);
      if (envelope.game !== game) {
        throw new SaveError("wrong-game", `slot "${slot}" holds a save from "${envelope.game}", not "${game}"`);
      }
      if (envelope.version > version) {
        throw new SaveError(
          "newer-version",
          `slot "${slot}" was written by a newer version (${envelope.version}); this build reads up to ${version}`,
        );
      }
      const data = migrateData(envelope.data, envelope.version, version, migrations);
      return { slot, savedAt: envelope.savedAt, version, data };
    },

    async remove(slot) {
      assertSlot(slot);
      await backend.remove(slot);
    },
  };
}

/** Checks the store options. Throws a RangeError for the first problem found. */
function validateOptions(game: string, version: number, migrations: MigrationMap): void {
  if (game.trim() === "") throw new RangeError("game must be a non-empty string");
  if (!Number.isInteger(version) || version < 1) {
    throw new RangeError(`version must be a positive integer, got ${version}`);
  }
  for (const key of Object.keys(migrations)) {
    const from = Number(key);
    if (!Number.isInteger(from) || from < 1 || from >= version) {
      throw new RangeError(`migration "${key}" must start from a version between 1 and ${version - 1}`);
    }
    const migrate: unknown = migrations[from];
    if (typeof migrate !== "function") throw new RangeError(`migration "${key}" must be a function`);
  }
}

/** Picks the backend: the explicit one, else the bridge, else localStorage, else memory. */
function chooseBackend(game: string, explicit: SaveBackend | undefined): { kind: SaveBackendKind; backend: SaveBackend } {
  if (explicit !== undefined) return { kind: "custom", backend: explicit };
  const bridge = getBridge();
  if (bridge !== null) return { kind: "bridge", backend: createBridgeBackend(bridge) };
  const storage = getLocalStorage();
  if (storage !== null) return { kind: "localStorage", backend: createLocalStorageBackend(storage, game) };
  return { kind: "memory", backend: createMemoryBackend() };
}

/** Throws a SaveError when the slot name is not valid. */
function assertSlot(slot: string): void {
  if (!isValidSlot(slot)) {
    throw new SaveError("bad-slot", `invalid slot ${JSON.stringify(slot)}: use 1 to 64 letters, digits, "_" or "-"`);
  }
}
