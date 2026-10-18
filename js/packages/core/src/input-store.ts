// The player's bindings on disk: a versioned JSON file, the parsing and migration of that file, and
// a store that keeps it in the settings of the Velvet Desktop bridge, or in localStorage, or in
// memory. The format is written down in docs/formats/INPUT_BINDINGS.md.
//
// A file holds every slot of every action that the map knows: a slot is either a binding or null
// for an empty slot, so loading it reproduces the slots as they were saved. Loading touches only the
// actions that the game lists; the rest keep their defaults. A file from a newer build is refused,
// and an older one is migrated with the migrations that the game passes in, exactly as save slots are.

import { readControl } from "./input-controls";
import { DEFAULT_CONTEXT } from "./input-rebind";
import { SaveError } from "./save-error";
import { isPositiveInteger, isRecord } from "./guards";
import { migrateData, type MigrationMap } from "./save-format";
import { chooseSettingsBackend, type SettingsBackend, type SettingsBackendKind } from "./settings-backend";
import type { ActionMap, Binding, Control } from "./input";

export type { SettingsBackend } from "./settings-backend";

/** The format tag of a bindings file. */
export const BINDINGS_FORMAT = "velvet-input-bindings";

/** The version that this build writes and reads. */
export const BINDINGS_VERSION = 1;

/** The settings key that the default store uses. */
export const BINDINGS_KEY = "input.bindings";

/** The longest action name, and the longest context name, that a file may hold. */
const MAX_NAME_LENGTH = 64;

/** The prefix of the localStorage keys that hold settings. */
const SETTINGS_PREFIX = "velvet-input:";

/** One slot of a bindings file: a control and the optional fields that a binding can have. */
export interface BindingsFileSlot {
  readonly control: Control;
  /** The scale. Omitted when it is 1. */
  readonly scale?: number;
  /** The axis component. Omitted for a digital binding. */
  readonly axisComponent?: "x" | "y";
  /** The context. Omitted when it is "default". */
  readonly context?: string;
}

/** One action of a bindings file: its slots in order. A slot number is the position in the list. */
export interface BindingsFileAction {
  readonly action: string;
  readonly slots: readonly (BindingsFileSlot | null)[];
}

/** A bindings file, as the map saves it. */
export interface BindingsFile {
  readonly format: typeof BINDINGS_FORMAT;
  readonly version: number;
  readonly actions: readonly BindingsFileAction[];
}

/** Options for {@link parseBindings}. */
export interface ParseBindingsOptions {
  /** The version that the reader supports. Default {@link BINDINGS_VERSION}. */
  readonly version?: number;
  /** Migrations keyed by the version they start from. See {@link MigrationMap}. */
  readonly migrations?: MigrationMap;
}

/** What {@link applyBindings} did. */
export interface ApplyBindingsResult {
  /** The slots that were set. */
  readonly applied: number;
  /** The slots of actions that the game does not list, which were left out. */
  readonly ignored: number;
}

/** Where a store keeps its file: the bridge's settings, localStorage, memory, or a backend that was passed in. */
export type BindingsBackendKind = SettingsBackendKind;

/** Options for {@link createBindingsStore}. */
export interface BindingsStoreOptions {
  /** The settings key. Default {@link BINDINGS_KEY}. */
  readonly key?: string;
  /** Settings storage to use instead of the automatic choice. */
  readonly settings?: SettingsBackend;
  /** The version that this store writes and reads. Default {@link BINDINGS_VERSION}. */
  readonly version?: number;
  /** Migrations keyed by the version they start from. See {@link MigrationMap}. */
  readonly migrations?: MigrationMap;
}

/** Saves and loads one player's bindings. */
export interface BindingsStore {
  /** The settings key that holds the file. */
  readonly key: string;
  /** The version that the store writes. */
  readonly version: number;
  /** The storage in use. */
  readonly backendKind: BindingsBackendKind;
  /** Writes every slot of the map. */
  save<Action extends string>(map: ActionMap<Action>): Promise<void>;
  /**
   * Reads the saved bindings into the map, for the actions in `knownActions`. Resolves to null when
   * nothing is saved.
   * @throws SaveError with code "corrupt", "newer-version" or "missing-migration" for a file that
   * cannot be read.
   */
  load<Action extends string>(map: ActionMap<Action>, knownActions: readonly Action[]): Promise<ApplyBindingsResult | null>;
  /** Forgets the saved bindings. The map keeps its current bindings. */
  clear(): Promise<void>;
}

/** Writes every action and slot of a map as a bindings file. */
export function serializeBindings<Action extends string>(
  map: ActionMap<Action>,
  version: number = BINDINGS_VERSION,
): BindingsFile {
  return {
    format: BINDINGS_FORMAT,
    version,
    actions: map.actions().map((action) => ({
      action,
      slots: map.slotsOf(action).map((binding) => (binding === null ? null : toFileSlot(binding))),
    })),
  };
}

/**
 * Reads a bindings file. The value can be the JSON text or the parsed object. A file of an older
 * version is migrated first. The result has the current version.
 * @throws SaveError with code "corrupt" when the file is not JSON or does not have the file's shape,
 * "newer-version" when it is from a newer build, and "missing-migration" when a step has no migration.
 */
export function parseBindings(value: unknown, options: ParseBindingsOptions = {}): BindingsFile {
  const current = options.version ?? BINDINGS_VERSION;
  const data = typeof value === "string" ? parseJson(value) : value;
  if (!isRecord(data)) throw new SaveError("corrupt", "bindings are not a JSON object");
  if (data["format"] !== BINDINGS_FORMAT) throw new SaveError("corrupt", `bindings are not a ${BINDINGS_FORMAT} file`);
  const version: unknown = data["version"];
  if (!isPositiveInteger(version)) throw new SaveError("corrupt", "bindings have no valid version");
  if (version > current) {
    throw new SaveError(
      "newer-version",
      `bindings were written by a newer version (${version}); this build reads up to ${current}`,
    );
  }
  const migrated = version < current ? migrateData(data, version, current, options.migrations ?? {}) : data;
  return readShape(migrated, current);
}

/**
 * Sets the slots of a map from a bindings file. For each action that `knownActions` lists, the
 * action's current slots are cleared and the file's slots are set in their numbers. The conflicts
 * that the file brings are kept, not resolved: see `ActionMap.conflicts`.
 */
export function applyBindings<Action extends string>(
  map: ActionMap<Action>,
  file: BindingsFile,
  knownActions: readonly Action[],
): ApplyBindingsResult {
  let applied = 0;
  let ignored = 0;
  for (const entry of file.actions) {
    const action = knownActions.find((name) => name === entry.action);
    if (action === undefined) {
      ignored += entry.slots.filter((slot) => slot !== null).length;
      continue;
    }
    map.slotsOf(action).forEach((binding, slot) => {
      if (binding !== null) map.clearSlot(action, slot);
    });
    entry.slots.forEach((slot, index) => {
      if (slot === null) return;
      const binding: Binding<Action> = {
        action,
        control: slot.control,
        scale: slot.scale,
        axisComponent: slot.axisComponent,
        context: slot.context,
      };
      map.rebind(action, index, binding, { policy: "allow" });
      applied += 1;
    });
  }
  return { applied, ignored };
}

/**
 * A store for bindings. It keeps the file in the bridge's settings when the desktop host is present,
 * otherwise in localStorage, otherwise in memory. A `settings` backend can be passed to choose it.
 * @throws RangeError when the key is empty or the version is not a whole number from 1 up.
 */
export function createBindingsStore(options: BindingsStoreOptions = {}): BindingsStore {
  const key = options.key ?? BINDINGS_KEY;
  if (key.trim() === "") throw new RangeError("the settings key must not be empty");
  const version = options.version ?? BINDINGS_VERSION;
  if (!Number.isInteger(version) || version < 1) throw new RangeError(`version must be a whole number from 1 up, got ${version}`);
  const migrations = options.migrations ?? {};
  const chosen = chooseSettingsBackend({ explicit: options.settings, prefix: SETTINGS_PREFIX });
  const backend = chosen.backend;
  return {
    key,
    version,
    backendKind: chosen.kind,
    async save(map) {
      await backend.set(key, serializeBindings(map, version));
    },
    async load(map, knownActions) {
      const stored = await backend.get(key);
      if (stored === null || stored === undefined) return null;
      return applyBindings(map, parseBindings(stored, { version, migrations }), knownActions);
    },
    async clear() {
      await backend.set(key, null);
    },
  };
}

/** The file form of one binding. The optional fields are left out when they hold their defaults. */
function toFileSlot(binding: Binding<string>): BindingsFileSlot {
  return {
    control: binding.control,
    ...(binding.scale === undefined || binding.scale === 1 ? {} : { scale: binding.scale }),
    ...(binding.axisComponent === undefined ? {} : { axisComponent: binding.axisComponent }),
    ...(binding.context === undefined || binding.context === DEFAULT_CONTEXT ? {} : { context: binding.context }),
  };
}

/** Parses JSON text. A text that is not JSON is corrupt. */
function parseJson(text: string): unknown {
  try {
    const parsed: unknown = JSON.parse(text);
    return parsed;
  } catch (cause) {
    throw new SaveError("corrupt", "bindings are not valid JSON", { cause });
  }
}

/** Checks the shape of a bindings file of the given version, and returns it as a typed file. */
function readShape(value: unknown, version: number): BindingsFile {
  if (!isRecord(value)) throw new SaveError("corrupt", "bindings are not a JSON object");
  const list: unknown = value["actions"];
  if (!Array.isArray(list)) throw new SaveError("corrupt", "bindings have no list of actions");
  const actions = list.map(readAction);
  const names = actions.map((entry) => entry.action);
  if (new Set(names).size !== names.length) throw new SaveError("corrupt", "bindings list an action twice");
  return { format: BINDINGS_FORMAT, version, actions };
}

/** Checks one action entry: its name and its slots. */
function readAction(entry: unknown): BindingsFileAction {
  if (!isRecord(entry)) throw new SaveError("corrupt", "an action entry is not an object");
  const action: unknown = entry["action"];
  if (typeof action !== "string" || action === "" || action.length > MAX_NAME_LENGTH) {
    throw new SaveError("corrupt", `an action name must be 1 to ${MAX_NAME_LENGTH} characters`);
  }
  const slots: unknown = entry["slots"];
  if (!Array.isArray(slots)) throw new SaveError("corrupt", `the slots of "${action}" are not a list`);
  return { action, slots: slots.map((slot: unknown) => (slot === null ? null : readSlot(slot, action))) };
}

/** Checks one slot: its control, and the optional scale, axis and context. */
function readSlot(entry: unknown, action: string): BindingsFileSlot {
  if (!isRecord(entry)) throw new SaveError("corrupt", `a slot of "${action}" is not an object`);
  let control: Control;
  try {
    control = readControl(entry["control"]);
  } catch (cause) {
    throw new SaveError("corrupt", `a slot of "${action}" has a bad control`, { cause });
  }
  const scale: unknown = entry["scale"];
  if (scale !== undefined && (typeof scale !== "number" || !Number.isFinite(scale))) {
    throw new SaveError("corrupt", `the scale of a slot of "${action}" must be a finite number`);
  }
  const axisComponent: unknown = entry["axisComponent"];
  if (axisComponent !== undefined && axisComponent !== "x" && axisComponent !== "y") {
    throw new SaveError("corrupt", `the axis of a slot of "${action}" must be "x" or "y"`);
  }
  const context: unknown = entry["context"];
  if (context !== undefined && (typeof context !== "string" || context === "" || context.length > MAX_NAME_LENGTH)) {
    throw new SaveError("corrupt", `the context of a slot of "${action}" must be 1 to ${MAX_NAME_LENGTH} characters`);
  }
  return {
    control,
    ...(typeof scale === "number" ? { scale } : {}),
    ...(axisComponent === "x" || axisComponent === "y" ? { axisComponent } : {}),
    ...(typeof context === "string" ? { context } : {}),
  };
}

