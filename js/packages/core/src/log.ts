// Logging for games. A logger has a name and a level. It keeps its entries in a ring buffer and passes
// each one to its sinks. A sink is where entries go: the console, or the Velvet Desktop log through the
// bridge's log call. An entry is plain data (its fields are JSON), so an error report can keep the latest
// entries and a player can read them.
//
// A logger and the children that `child` makes share one buffer, one list of sinks and one revision
// counter. Each keeps its own level, and a child starts at its parent's level. An entry below the level
// is dropped before anything is built, so a call that is filtered out costs one comparison.

import { getBridge } from "./bridge";

/** The levels, from the most detailed to the most serious. */
export const LOG_LEVELS = ["debug", "info", "warn", "error"] as const;

/** One of the log levels. */
export type LogLevel = (typeof LOG_LEVELS)[number];

/** A value that JSON can hold. */
export type JsonValue = string | number | boolean | null | JsonValue[] | JsonObject;

/** An object whose values are JSON values. */
export interface JsonObject {
  [key: string]: JsonValue;
}

/** The fields that a caller gives with a message. Their values are made JSON-safe when the entry is kept. */
export type LogFields = Readonly<Record<string, unknown>>;

/** The name, message and stack of an error, as plain text. */
export interface ErrorParts {
  /** The error's name, such as "TypeError". */
  readonly name: string;
  /** The error's message. Empty when it has none. */
  readonly message: string;
  /** The stack trace, or null when the error has none. */
  readonly stack: string | null;
}

/** One recorded entry. Its fields are JSON data, so the entry can be kept, sent or shown as it is. */
export interface LogEntry {
  /** When the entry was made, in milliseconds since the Unix epoch. */
  readonly time: number;
  /** The level it was made at. */
  readonly level: LogLevel;
  /** The name of the logger that made it, such as "game" or "game.combat". */
  readonly logger: string;
  /** The message, as the caller wrote it. */
  readonly message: string;
  /** The fields given with the message, as JSON. Empty when none were given. */
  readonly fields: Readonly<JsonObject>;
}

/** Where entries go. A sink that throws is skipped for that entry, so a broken sink never breaks the game. */
export interface LogSink {
  /** Receives one entry. */
  write(entry: LogEntry): void;
}

/** The part of the console that the console sink writes to. */
export type ConsoleLike = Pick<Console, "debug" | "info" | "warn" | "error">;

/** The part of the Velvet Desktop bridge that the bridge sink writes to: its log call. */
export interface LogTarget {
  /** Writes one line of text to the host's log at the given level. */
  log(level: LogLevel, message: string): void;
}

/** Options for {@link createLogger}. */
export interface LoggerOptions {
  /** The name, such as "game". Letters, digits, "-" and "_", with dots between the parts. */
  readonly name: string;
  /** Entries below this level are dropped. Default "info". */
  readonly level?: LogLevel;
  /** Where entries go. Default: the console sink. Pass an empty list to keep entries in the buffer only. */
  readonly sinks?: readonly LogSink[];
  /** How many entries the buffer keeps, across the logger and its children. Default 500. The oldest go first. */
  readonly capacity?: number;
}

/** A named logger. Make the root one with {@link createLogger}, and its children with `child`. */
export interface Logger {
  /** The full name, such as "game" or "game.combat". */
  readonly name: string;
  /** The level below which this logger drops entries. */
  readonly level: LogLevel;
  /** Goes up on every change to the family: an entry, a level or a clear. Views use it to know when to redraw. */
  readonly revision: number;
  /**
   * Changes this logger's level. Its parent and its children keep theirs.
   * @throws RangeError when the level is not one of the log levels.
   */
  setLevel(level: LogLevel): void;
  /** True when an entry at this level would be kept. Check it before building an expensive message. */
  isEnabled(level: LogLevel): boolean;
  /** Logs at the debug level. */
  debug(message: string, fields?: LogFields): void;
  /** Logs at the info level. */
  info(message: string, fields?: LogFields): void;
  /** Logs at the warn level. */
  warn(message: string, fields?: LogFields): void;
  /** Logs at the error level. */
  error(message: string, fields?: LogFields): void;
  /**
   * A child logger, named with a dot after this one's name: "game" gives "game.combat". The same name gives
   * the same logger. A child starts at this logger's level.
   * @throws RangeError when the name is not valid.
   */
  child(name: string): Logger;
  /** The kept entries of the family, oldest first. A copy: changing it changes nothing. */
  entries(): readonly LogEntry[];
  /** Empties the buffer, which the whole family shares. */
  clear(): void;
  /** Every logger of the family, this one included, in the order they were made. */
  loggers(): readonly Logger[];
}

/** The rank of each level: a logger keeps the entries whose rank is at least its own. */
const RANK: Readonly<Record<LogLevel, number>> = { debug: 0, info: 1, warn: 2, error: 3 };

/** Letters, digits, "-" and "_" in each part of a name, with dots between the parts. */
const NAME_PATTERN = /^[A-Za-z0-9_-]+(\.[A-Za-z0-9_-]+)*$/;

/** How many entries a buffer keeps when the options name no capacity. */
const DEFAULT_CAPACITY = 500;

/** How deep the objects of a field are written before a marker takes their place. */
const MAX_DEPTH = 6;

/** How many items of a list, or keys of an object, are written. The rest become one marker. */
const MAX_ITEMS = 100;

/** The fields of an entry that has none. It is shared and frozen, so an entry without fields allocates nothing. */
const EMPTY_FIELDS: Readonly<JsonObject> = Object.freeze({});

/** The state that a logger and its children share. */
interface Family {
  /** Where each entry goes. */
  readonly sinks: readonly LogSink[];
  /** The latest entries of the family. */
  readonly buffer: RingBuffer<LogEntry>;
  /** Every logger of the family, by its full name. */
  readonly members: Map<string, LoggerImpl>;
  /** Goes up on every change, so a view can tell when it must redraw. */
  revision: number;
}

/**
 * Creates the root logger of a family. Its children come from `child`, and they share its buffer and sinks.
 * @param options - the name, the level, the sinks and the buffer size.
 * @throws RangeError when the name, the level or the capacity is not valid.
 * @example
 * const log = createLogger({ name: "game", level: "debug", sinks: [createConsoleSink()] });
 * const combat = log.child("combat");
 * combat.warn("low health", { hp: 3 });
 */
export function createLogger(options: LoggerOptions): Logger {
  const name = requireName(options.name);
  const level = requireLevel(options.level ?? "info");
  const capacity = options.capacity ?? DEFAULT_CAPACITY;
  if (!Number.isInteger(capacity) || capacity < 1) {
    throw new RangeError(`capacity must be a whole number from 1 up, not ${String(capacity)}`);
  }
  const family: Family = {
    sinks: [...(options.sinks ?? [createConsoleSink()])],
    buffer: new RingBuffer<LogEntry>(capacity),
    members: new Map<string, LoggerImpl>(),
    revision: 0,
  };
  const root = new LoggerImpl(name, level, family);
  family.members.set(name, root);
  return root;
}

/** True when `value` is one of the log levels. */
export function isLogLevel(value: unknown): value is LogLevel {
  return typeof value === "string" && (LOG_LEVELS as readonly string[]).includes(value);
}

/**
 * A sink that writes each entry to a console method of its level. The line is "logger: message", and the
 * fields follow as an object when there are any, so a browser's console can open them.
 * @param output - the console to write to. Default: the global console.
 */
export function createConsoleSink(output: ConsoleLike = console): LogSink {
  return {
    write(entry: LogEntry): void {
      const line = `${entry.logger}: ${entry.message}`;
      if (Object.keys(entry.fields).length === 0) output[entry.level](line);
      else output[entry.level](line, entry.fields);
    },
  };
}

/**
 * A sink that writes each entry to the Velvet Desktop log, through the bridge's log call. The text is
 * "logger: message", followed by the fields as JSON when there are any. The host keeps at most 8 KiB of
 * a message, and writes it to velvet-desktop.log.
 * @param target - the bridge, or a function that finds it each time an entry is written. Default: the
 * bridge that `getBridge` finds, so a page outside Velvet Desktop writes nothing.
 */
export function createBridgeSink(target: LogTarget | (() => LogTarget | null) = getBridge): LogSink {
  return {
    write(entry: LogEntry): void {
      const destination: LogTarget | null = typeof target === "function" ? target() : target;
      if (destination === null) return;
      destination.log(entry.level, bridgeText(entry));
    },
  };
}

/**
 * The name, message and stack of an error, read without ever throwing. A value that is not an error
 * gives null.
 */
export function errorParts(value: unknown): ErrorParts | null {
  if (!isErrorLike(value)) return null;
  const error = value;
  return {
    name: readText(() => error.name) ?? "Error",
    message: readText(() => error.message) ?? "",
    stack: readText(() => error.stack),
  };
}

/**
 * A value as plain JSON data. An error becomes { name, message, stack }. A cycle, a function, a value
 * that is too deep and a list that is too long become short markers. Undefined stays undefined, so a
 * caller can drop it. A getter that throws becomes "[Unreadable]". Never throws.
 */
export function toJsonValue(value: unknown): JsonValue | undefined {
  return safePlain(value, [], 0);
}

/**
 * Fields as a plain JSON object, with the same rules as {@link toJsonValue}. Never throws: fields that
 * cannot be read at all become a single marker.
 */
export function toJsonObject(fields: LogFields): Readonly<JsonObject> {
  try {
    return recordOf(fields, [fields], 0);
  } catch {
    return { fields: "[Unreadable]" };
  }
}

/** The entry's fields and message, in one line, as the bridge's log call takes them. */
function bridgeText(entry: LogEntry): string {
  const head = `${entry.logger}: ${entry.message}`;
  return Object.keys(entry.fields).length === 0 ? head : `${head} ${JSON.stringify(entry.fields)}`;
}

/** Checks a level, and returns it. Throws a RangeError for anything else. */
function requireLevel(level: unknown): LogLevel {
  if (isLogLevel(level)) return level;
  throw new RangeError(`a log level must be debug, info, warn or error, not ${String(level)}`);
}

/** Checks a logger name, and returns it. Throws a RangeError for a name that the console cannot target. */
function requireName(name: string): string {
  if (NAME_PATTERN.test(name)) return name;
  throw new RangeError(`a logger name is letters, digits, "-" and "_", with dots between parts, not "${name}"`);
}

/** True for an Error, or for an error from another page, which is not an instance of this page's Error class. */
function isErrorLike(value: unknown): value is Error {
  return value instanceof Error || Object.prototype.toString.call(value) === "[object Error]";
}

/** The text that a read gives, or null when the read gives something else or throws. */
function readText(read: () => unknown): string | null {
  try {
    const value = read();
    return typeof value === "string" ? value : null;
  } catch {
    return null;
  }
}

/** A value as JSON data, or a marker when reading it fails. Used for every value inside a field. */
function safePlain(value: unknown, ancestors: readonly object[], depth: number): JsonValue | undefined {
  try {
    return plainOf(value, ancestors, depth);
  } catch {
    return "[Unreadable]";
  }
}

/** The JSON form of a value of any type. Objects go to {@link objectOf}. */
function plainOf(value: unknown, ancestors: readonly object[], depth: number): JsonValue | undefined {
  if (value === null) return null;
  switch (typeof value) {
    case "string":
    case "boolean":
      return value;
    case "number":
      return Number.isFinite(value) ? value : String(value);
    case "bigint":
    case "symbol":
      return value.toString();
    case "function":
      return `[Function ${value.name || "anonymous"}]`;
    case "undefined":
      return undefined;
    default:
      return objectOf(value as object, ancestors, depth);
  }
}

/** The JSON form of an object: a marker for a cycle or a value that is too deep, or the object's own form. */
function objectOf(object: object, ancestors: readonly object[], depth: number): JsonValue {
  if (ancestors.includes(object)) return "[Circular]";
  if (depth >= MAX_DEPTH) return "[Depth limit]";
  if (object instanceof Date) return Number.isNaN(object.getTime()) ? null : object.toISOString();
  if (object instanceof ArrayBuffer) return `[ArrayBuffer ${object.byteLength} bytes]`;
  if (ArrayBuffer.isView(object)) return `[${object.constructor.name} ${object.byteLength} bytes]`;
  const inner = [...ancestors, object];
  if (isErrorLike(object)) {
    const parts = errorParts(object);
    if (parts !== null) return { name: parts.name, message: parts.message, stack: parts.stack };
  }
  if (Array.isArray(object) || object instanceof Map || object instanceof Set) {
    return listOf(object, inner, depth);
  }
  return recordOf(object, inner, depth);
}

/** The items of an array, a Map (as pairs) or a Set, up to the limit. */
function listOf(items: Iterable<unknown>, ancestors: readonly object[], depth: number): JsonValue[] {
  const list: JsonValue[] = [];
  for (const item of items) {
    if (list.length === MAX_ITEMS) {
      list.push(`[more than ${MAX_ITEMS} items]`);
      break;
    }
    list.push(safePlain(item, ancestors, depth + 1) ?? null);
  }
  return list;
}

/** The own enumerable properties of an object, up to the limit. A getter that throws is marked. */
function recordOf(object: object, ancestors: readonly object[], depth: number): JsonObject {
  const record: JsonObject = {};
  const keys = Object.keys(object);
  for (const [index, key] of keys.entries()) {
    if (index === MAX_ITEMS) {
      setOwn(record, "[more]", `${keys.length - MAX_ITEMS} more keys`);
      break;
    }
    let item: unknown;
    try {
      item = Reflect.get(object, key);
    } catch {
      setOwn(record, key, "[Unreadable]");
      continue;
    }
    const plain = safePlain(item, ancestors, depth + 1);
    if (plain !== undefined) setOwn(record, key, plain);
  }
  return record;
}

/** Writes an own property, so that a key such as "__proto__" stays a plain key of the record. */
function setOwn(record: JsonObject, key: string, value: JsonValue): void {
  Object.defineProperty(record, key, { value, enumerable: true, writable: true, configurable: true });
}

/** The fields of an entry as JSON: the shared empty object when the caller gave none, or null. */
function fieldsOf(fields: LogFields | null | undefined): Readonly<JsonObject> {
  return fields === undefined || fields === null ? EMPTY_FIELDS : toJsonObject(fields);
}

/** A fixed-size buffer that keeps the newest items. Pushing into a full buffer drops the oldest one. */
class RingBuffer<T> {
  private readonly slots: Array<T | undefined>;
  private readonly capacity: number;
  private next = 0;
  private size = 0;

  /** @param capacity - how many items are kept. */
  constructor(capacity: number) {
    this.capacity = capacity;
    this.slots = new Array<T | undefined>(capacity);
  }

  /** Adds an item at the end, dropping the oldest when the buffer is full. */
  push(item: T): void {
    this.slots[this.next] = item;
    this.next = (this.next + 1) % this.capacity;
    if (this.size < this.capacity) this.size += 1;
  }

  /** The items, oldest first, as a new array. */
  toArray(): T[] {
    const items: T[] = [];
    const start = (this.next - this.size + this.capacity) % this.capacity;
    for (let offset = 0; offset < this.size; offset += 1) {
      // Every slot inside the counted range has been written, so the cast cannot hide an empty slot.
      items.push(this.slots[(start + offset) % this.capacity] as T);
    }
    return items;
  }

  /** Empties the buffer. */
  clear(): void {
    this.slots.fill(undefined);
    this.next = 0;
    this.size = 0;
  }
}

/** A logger. Loggers of one family share the family's state; each keeps its own level. */
class LoggerImpl implements Logger {
  readonly name: string;
  private currentLevel: LogLevel;
  private readonly family: Family;

  /**
   * @param name - the full name of the logger.
   * @param level - its level.
   * @param family - the state it shares with the other loggers of its family.
   */
  constructor(name: string, level: LogLevel, family: Family) {
    this.name = name;
    this.currentLevel = level;
    this.family = family;
  }

  get level(): LogLevel {
    return this.currentLevel;
  }

  get revision(): number {
    return this.family.revision;
  }

  setLevel(level: LogLevel): void {
    this.currentLevel = requireLevel(level);
    this.family.revision += 1;
  }

  isEnabled(level: LogLevel): boolean {
    return RANK[level] >= RANK[this.currentLevel];
  }

  debug(message: string, fields?: LogFields): void {
    this.write("debug", message, fields);
  }

  info(message: string, fields?: LogFields): void {
    this.write("info", message, fields);
  }

  warn(message: string, fields?: LogFields): void {
    this.write("warn", message, fields);
  }

  error(message: string, fields?: LogFields): void {
    this.write("error", message, fields);
  }

  child(name: string): Logger {
    const full = requireName(`${this.name}.${name}`);
    const known = this.family.members.get(full);
    if (known !== undefined) return known;
    const child = new LoggerImpl(full, this.currentLevel, this.family);
    this.family.members.set(full, child);
    return child;
  }

  entries(): readonly LogEntry[] {
    return this.family.buffer.toArray();
  }

  clear(): void {
    this.family.buffer.clear();
    this.family.revision += 1;
  }

  loggers(): readonly Logger[] {
    return [...this.family.members.values()];
  }

  /** Keeps an entry if its level is enabled: builds it, stores it, and passes it to every sink. */
  private write(level: LogLevel, message: string, fields: LogFields | null | undefined): void {
    if (!this.isEnabled(level)) return;
    const entry: LogEntry = {
      time: Date.now(),
      level,
      logger: this.name,
      message: typeof message === "string" ? message : String(message),
      fields: fieldsOf(fields),
    };
    this.family.buffer.push(entry);
    this.family.revision += 1;
    for (const sink of this.family.sinks) {
      try {
        sink.write(entry);
      } catch {
        // A sink that fails must not break the game. The entry is still in the buffer.
      }
    }
  }
}
