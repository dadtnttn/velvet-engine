// Settings for a game: a few typed values with defaults, kept between runs. The values belong to a
// namespace, one per package or game area. The namespace is the key that holds them, so two packages
// cannot collide.
//
// The saved form is a versioned envelope:
//
//     { "format": "velvet-settings", "version": 2, "values": { "volume": 0.5 } }
//
// It is written to the bridge's settings under the namespace, or to localStorage under
// "velvet-settings:" followed by the namespace, or kept in memory when neither is there.
//
// Reading. `load` reads the saved values once. A saved value is used only when it has the type of
// its default: a number for a number, an array for an array, an object for an object. A default of
// null takes any value. A value of the wrong type is ignored, so a damaged setting never stops the
// game from starting. Values of an older version are brought up to date by the migrations, one step
// at a time, as save slots are. Values of a newer build are not read. A plain object without the
// envelope is the first layout, which is version 1.
//
// Choices. A setting that has a fixed set of values, such as a language or a speed, can name them in
// `choices`. A saved value outside its list is refused: the default stays, and `onInvalid` hears of it
// once, when the values are read. `set` throws a RangeError for a value outside the list.
//
// Writing. `set` changes a value at once, for its subscribers, and saves every value after a short
// debounce, so a burst of changes is one write. `save` writes now. A write waits for the load, so a
// save never replaces stored values that have not been read yet. A storage that refuses a write makes
// `save` resolve to false: a settings store never throws into the game because storage failed.

import { EventEmitter, type Listener, type Unsubscribe } from "./events";
import { isPositiveInteger, isRecord } from "./guards";
import type { MigrationMap } from "./save-format";
import type { StorageLike } from "./save-backends";
import {
  chooseSettingsBackend,
  type ChosenSettingsBackend,
  type SettingsBackend,
  type SettingsBackendKind,
} from "./settings-backend";

/** The format tag of a settings envelope. */
export const SETTINGS_FORMAT = "velvet-settings";

/** The prefix of the localStorage keys that hold settings. The namespace follows it. */
export const SETTINGS_STORAGE_PREFIX = "velvet-settings:";

/** How long `set` waits after the last change before it saves, in milliseconds, by default. */
const DEFAULT_DEBOUNCE_MS = 250;

/** A namespace: letters, digits, dots, dashes and underscores, starting with a letter or a digit, 64 at most. */
const NAMESPACE_PATTERN = /^[A-Za-z0-9][A-Za-z0-9._-]{0,63}$/;

/** Options for {@link createSettingsStore}. */
export interface SettingsStoreOptions<Values extends object> {
  /**
   * The namespace of the values: 1 to 64 letters, digits, dots, dashes or underscores, starting with a letter
   * or a digit. It is also the key of the values in the bridge's settings, so each package picks its own.
   */
  readonly namespace: string;
  /** The current version of the values: a positive integer. */
  readonly version: number;
  /** The values, with their defaults. A value that the saved data lacks takes its default. */
  readonly defaults: Values;
  /** Migrations keyed by the version they start from, as for save slots: `migrate[1]` turns version 1 values into version 2. */
  readonly migrate?: MigrationMap;
  /** The bridge whose settings hold the values. Default: the bridge of Velvet Desktop, when it is there. `null` skips it. */
  readonly bridge?: { readonly settings: SettingsBackend } | null;
  /** The web storage that holds the values in a browser. Default: localStorage, when the browser allows it. `null` skips it. */
  readonly storage?: StorageLike | null;
  /** A backend that holds the values. It is used before the bridge and the storage. For tests and custom hosts. */
  readonly backend?: SettingsBackend;
  /** Milliseconds that `set` waits after the last change before it saves. Default 250. */
  readonly debounceMs?: number;
  /**
   * The values that a setting may take, for a setting with a fixed set of values: a list for each such key, such as
   * `{ language: ["es", "en"] }`. A saved value that is not in its list is refused, and the default is used. `set`
   * throws a RangeError for a value that is not in the list. The default must be in its list: put null in the list
   * when the default is null.
   */
  readonly choices?: { readonly [Key in keyof Values]?: readonly Values[Key][] };
  /**
   * Runs when a saved value is refused by its `choices`, with the name of the setting and the value that was refused.
   * It runs once for each such value, when the saved values are read. A value of the wrong type is ignored as before,
   * without a call.
   */
  readonly onInvalid?: (key: string, value: unknown) => void;
}

/**
 * Typed settings for one namespace. Create one with {@link createSettingsStore}. Read the saved values with
 * `load` before the game relies on them; until then `get` gives the defaults.
 */
export interface SettingsStore<Values extends object> {
  /** The namespace that holds the values. */
  readonly namespace: string;
  /** The current version of the values. */
  readonly version: number;
  /** Where the values are kept. */
  readonly backendKind: SettingsBackendKind;
  /** The value of a setting: its default until `load` or `set` gives it another. */
  get<Key extends keyof Values>(key: Key): Values[Key];
  /**
   * Sets a value. Its subscribers hear of the change at once, and every value is saved after the debounce.
   * @throws RangeError when `key` is not one of the defaults, or the value is not one of the choices of `key`.
   */
  set<Key extends keyof Values>(key: Key, value: Values[Key]): void;
  /**
   * Puts settings back to their defaults: the one setting `key` names, or every setting when `key` is left out.
   * The subscribers of each value that changes hear of it, and the values are saved after the debounce, as with `set`.
   * @throws RangeError when `key` is not one of the defaults.
   */
  reset(key?: keyof Values): void;
  /**
   * Runs `listener` with the new value each time that value changes, by `set` or by `load`.
   * @throws RangeError when `key` is not one of the defaults.
   * @returns a function that removes the listener.
   */
  subscribe<Key extends keyof Values & string>(key: Key, listener: Listener<Values[Key]>): Unsubscribe;
  /**
   * Reads the saved values. Repeated calls wait for the same read. Resolves when the values are in place. A
   * read that fails leaves the defaults in place. Rejects only when a subscriber or `onInvalid` throws while the
   * saved values are being applied.
   */
  load(): Promise<void>;
  /**
   * Writes every value now, without the debounce. Resolves true when the storage took them, and false when it
   * refused. Never rejects.
   */
  save(): Promise<boolean>;
}

/** The envelope that is written: the format tag, the version and the values. */
interface SettingsEnvelope {
  readonly format: typeof SETTINGS_FORMAT;
  readonly version: number;
  readonly values: Record<string, unknown>;
}

/**
 * Creates a settings store for one namespace. The values are kept in the bridge's settings in Velvet Desktop,
 * in localStorage in a browser, and in memory otherwise. One namespace is one store: two stores on the same
 * namespace share the saved values, and the last write wins.
 *
 * @throws RangeError when the namespace, the version, a migration key, the defaults, the choices or the debounce is
 * not valid.
 */
export function createSettingsStore<Values extends object>(options: SettingsStoreOptions<Values>): SettingsStore<Values> {
  const migrate = options.migrate ?? {};
  const debounceMs = options.debounceMs ?? DEFAULT_DEBOUNCE_MS;
  checkOptions(options, migrate, debounceMs);
  const rules: SettingRules = { choices: choiceLists(options), onInvalid: options.onInvalid };
  const chosen: ChosenSettingsBackend = chooseSettingsBackend({
    explicit: options.backend,
    bridge: options.bridge,
    storage: options.storage,
    prefix: SETTINGS_STORAGE_PREFIX,
  });
  return new Settings<Values>(options.namespace, options.version, options.defaults, migrate, chosen, debounceMs, rules);
}

/** The settings store that {@link createSettingsStore} makes. Not exported: the game only sees the interface. */
class Settings<Values extends object> implements SettingsStore<Values> {
  readonly namespace: string;
  readonly version: number;
  readonly backendKind: SettingsBackendKind;
  /** The default of every setting, by name. Also the list of the names that the store knows. */
  private readonly defaults: ReadonlyMap<string, unknown>;
  /** The current value of every setting, by name. */
  private readonly values: Map<string, unknown>;
  /** The settings that were set in this store, so that a saved value does not replace them on load. */
  private readonly touched = new Set<string>();
  private readonly events = new EventEmitter<Record<string, unknown>>();
  private readonly migrations: MigrationMap;
  private readonly backend: SettingsBackend;
  private readonly debounceMs: number;
  private readonly rules: SettingRules;
  private loading: Promise<void> | null = null;
  private timer: ReturnType<typeof setTimeout> | null = null;
  /** The writes in order: each one waits for the one before, so the last value written is the last one set. */
  private writing: Promise<void> = Promise.resolve();

  constructor(
    namespace: string,
    version: number,
    defaults: object,
    migrations: MigrationMap,
    chosen: ChosenSettingsBackend,
    debounceMs: number,
    rules: SettingRules,
  ) {
    this.namespace = namespace;
    this.version = version;
    this.backendKind = chosen.kind;
    this.backend = chosen.backend;
    this.migrations = migrations;
    this.debounceMs = debounceMs;
    this.rules = rules;
    this.defaults = new Map(Object.entries(defaults));
    this.values = new Map(this.defaults);
  }

  get<Key extends keyof Values>(key: Key): Values[Key] {
    return this.values.get(this.nameOf(key)) as Values[Key];
  }

  set<Key extends keyof Values>(key: Key, value: Values[Key]): void {
    const name = this.nameOf(key);
    // The check comes before the mark: a refused value must not stop a saved value from loading later.
    this.requireChoice(name, value);
    this.touched.add(name);
    if (!this.changeValue(name, value)) return;
    // The save is scheduled before the subscribers run, so a subscriber that throws cannot lose the change.
    this.scheduleSave();
    this.events.emit(name, value);
  }

  reset(key?: keyof Values): void {
    const names = key === undefined ? [...this.defaults.keys()] : [this.nameOf(key)];
    const changed: string[] = [];
    for (const name of names) {
      this.touched.add(name);
      if (this.changeValue(name, this.defaults.get(name))) changed.push(name);
    }
    if (changed.length === 0) return;
    // As in `set`, the save is scheduled before the subscribers run.
    this.scheduleSave();
    for (const name of changed) this.events.emit(name, this.values.get(name));
  }

  subscribe<Key extends keyof Values & string>(key: Key, listener: Listener<Values[Key]>): Unsubscribe {
    const name = this.nameOf(key);
    return this.events.on(name, (value) => listener(value as Values[Key]));
  }

  load(): Promise<void> {
    if (this.loading === null) {
      this.loading = this.readSaved().then((saved) => {
        if (saved !== null) this.applySaved(saved);
      });
    }
    return this.loading;
  }

  async save(): Promise<boolean> {
    this.cancelTimer();
    // A save must not replace the stored values before they have been read, so it waits for the load.
    // A load that rejects because a subscriber threw still lets the save go on with the values in place.
    await this.load().catch(() => undefined);
    const envelope: SettingsEnvelope = {
      format: SETTINGS_FORMAT,
      version: this.version,
      values: Object.fromEntries(this.values),
    };
    const write = this.writing.then(() => this.writeEnvelope(envelope));
    this.writing = write.then(() => undefined);
    return write;
  }

  /** The name of a setting, checked against the defaults. @throws RangeError when the store has no such setting. */
  private nameOf(key: PropertyKey): string {
    const name = String(key);
    if (!this.defaults.has(name)) throw new RangeError(`"${name}" is not a setting of "${this.namespace}"`);
    return name;
  }

  /** True when a value is one of the choices of a setting, or the setting has no list of choices. */
  private isChoice(name: string, value: unknown): boolean {
    const list = this.rules.choices.get(name);
    return list === undefined || list.includes(value);
  }

  /** Throws a RangeError when a setting has a list of choices and the value is not in it. */
  private requireChoice(name: string, value: unknown): void {
    if (this.isChoice(name, value)) return;
    const shown = (this.rules.choices.get(name) ?? []).map(describeValue).join(", ");
    throw new RangeError(
      `${describeValue(value)} is not a choice of "${name}" in "${this.namespace}": the choices are ${shown}`,
    );
  }

  /** Keeps a value. Returns false when it is the one already held, so that nothing changed. */
  private changeValue(name: string, value: unknown): boolean {
    if (Object.is(this.values.get(name), value)) return false;
    this.values.set(name, value);
    return true;
  }

  /** Applies the saved values, for the settings that were not set in the meantime and that have the right type. */
  private applySaved(saved: Record<string, unknown>): void {
    for (const [name, fallback] of this.defaults) {
      if (this.touched.has(name) || !Object.hasOwn(saved, name)) continue;
      const value = saved[name];
      if (!fitsDefault(value, fallback)) continue;
      if (!this.isChoice(name, value)) {
        // A value outside the choices keeps the default, and the owner of the store hears of it once.
        this.rules.onInvalid?.(name, value);
        continue;
      }
      if (!this.changeValue(name, value)) continue;
      this.events.emit(name, value);
    }
  }

  /** Reads the saved values, migrated to the current version. Never rejects: a read that fails gives null. */
  private async readSaved(): Promise<Record<string, unknown> | null> {
    try {
      const raw = await this.backend.get(this.namespace);
      return decodeSaved(raw, this.version, this.migrations);
    } catch {
      return null;
    }
  }

  /** Writes an envelope. Resolves false when the storage refuses it. */
  private async writeEnvelope(envelope: SettingsEnvelope): Promise<boolean> {
    try {
      await this.backend.set(this.namespace, envelope);
      return true;
    } catch {
      return false;
    }
  }

  /** Starts the timer of a save. A change that comes before it fires moves it on. */
  private scheduleSave(): void {
    this.cancelTimer();
    this.timer = setTimeout(() => {
      this.timer = null;
      void this.save();
    }, this.debounceMs);
  }

  private cancelTimer(): void {
    if (this.timer === null) return;
    clearTimeout(this.timer);
    this.timer = null;
  }
}

/**
 * Reads the values out of a saved value, migrated to the current version. Returns null when there is nothing
 * that can be used: no value, a value that is not an object, a value from a newer build, or a version step
 * that has no migration.
 */
function decodeSaved(raw: unknown, version: number, migrations: MigrationMap): Record<string, unknown> | null {
  if (!isRecord(raw)) return null;
  let savedVersion = 1;
  let values: unknown = raw;
  if (raw["format"] !== undefined) {
    if (raw["format"] !== SETTINGS_FORMAT) return null;
    if (!isPositiveInteger(raw["version"]) || !isRecord(raw["values"])) return null;
    savedVersion = raw["version"];
    values = raw["values"];
  }
  if (savedVersion > version) return null;
  for (let step = savedVersion; step < version; step += 1) {
    const migrate = migrations[step];
    if (migrate === undefined) return null;
    values = migrate(values);
  }
  return isRecord(values) ? values : null;
}

/** A value for an error message: a string in quotes, anything else as it prints. */
function describeValue(value: unknown): string {
  return typeof value === "string" ? JSON.stringify(value) : String(value);
}

/** True when a saved value has the type of its default. A default of null takes any value. */
function fitsDefault(value: unknown, fallback: unknown): boolean {
  if (value === undefined) return false;
  if (fallback === null) return true;
  if (Array.isArray(fallback)) return Array.isArray(value);
  if (typeof fallback === "number") return typeof value === "number" && Number.isFinite(value);
  if (typeof fallback === "object") return isRecord(value);
  return typeof value === typeof fallback;
}

/** What the store checks a value against, from the options: the choices of each setting, and the report of a refused value. */
interface SettingRules {
  /** The values that each setting may take, by name, for the settings that have a list. */
  readonly choices: ReadonlyMap<string, readonly unknown[]>;
  /** Runs when a saved value is refused by its choices. */
  readonly onInvalid: ((key: string, value: unknown) => void) | undefined;
}

/**
 * The lists of choices, by setting name. A setting with no list is left out of the map.
 * @throws RangeError when a list names no setting, is empty, or does not hold the default of its setting.
 */
function choiceLists<Values extends object>(
  options: SettingsStoreOptions<Values>,
): ReadonlyMap<string, readonly unknown[]> {
  const defaults = new Map<string, unknown>(Object.entries(options.defaults));
  const lists = new Map<string, readonly unknown[]>();
  for (const [key, list] of Object.entries<unknown>(options.choices ?? {})) {
    if (list === undefined) continue;
    if (!defaults.has(key)) {
      throw new RangeError(`choices name "${key}", which is not a setting of "${options.namespace}"`);
    }
    if (!Array.isArray(list) || list.length === 0) {
      throw new RangeError(`the choices of "${key}" must be a non-empty list`);
    }
    if (!list.includes(defaults.get(key))) throw new RangeError(`the default of "${key}" is not one of its choices`);
    lists.set(key, list);
  }
  return lists;
}

/**
 * Checks the options of a settings store. Throws a RangeError for the first problem.
 * @throws RangeError when a field is not valid.
 */
function checkOptions<Values extends object>(options: SettingsStoreOptions<Values>, migrate: MigrationMap, debounceMs: number): void {
  if (!NAMESPACE_PATTERN.test(options.namespace)) {
    throw new RangeError(
      `namespace must be 1 to 64 letters, digits, ".", "-" or "_", starting with a letter or a digit: ${JSON.stringify(options.namespace)}`,
    );
  }
  if (!isPositiveInteger(options.version)) {
    throw new RangeError(`version must be a positive integer, got ${options.version}`);
  }
  if (!isRecord(options.defaults)) throw new RangeError("defaults must be an object of setting values");
  for (const key of Object.keys(migrate)) {
    const from = Number(key);
    if (!Number.isInteger(from) || from < 1 || from >= options.version) {
      throw new RangeError(`migration "${key}" must start from a version between 1 and ${options.version - 1}`);
    }
    const step: unknown = migrate[from];
    if (typeof step !== "function") throw new RangeError(`migration "${key}" must be a function`);
  }
  if (options.onInvalid !== undefined && typeof options.onInvalid !== "function") {
    throw new RangeError("onInvalid must be a function");
  }
  if (!Number.isFinite(debounceMs) || debounceMs < 0) {
    throw new RangeError(`debounceMs must be zero or a positive number, got ${debounceMs}`);
  }
}
