// Text for people: a value as JSON for the clipboard, a short line for the event log and the console,
// and fixed-point numbers. Cycles, functions, unreadable properties and very deep values become short
// markers, so the text is always finite, and no getter is called more than once.

/** Deepest level that the JSON copy writes. */
const COPY_DEPTH = 40;

/** Deepest level that a one-line text writes. */
const LINE_DEPTH = 3;

/** Longest one-line text, in characters. */
const LINE_CHARS = 120;

/** Where a walk through a value is. */
interface Walk {
  readonly maxDepth: number;
  readonly depth: number;
  readonly ancestors: readonly object[];
}

/**
 * The value as indented JSON, for the clipboard. A cycle is written as "[Circular]", a function as its
 * name, and a Map or a Set as a list of its entries. An undefined value is written as null.
 */
export function copyJson(value: unknown): string {
  try {
    const plain = plainOf(value, { maxDepth: COPY_DEPTH, depth: 0, ancestors: [] });
    return JSON.stringify(plain === undefined ? null : plain, null, 2) ?? "null";
  } catch (error) {
    return `"[Unreadable: ${messageOf(error)}]"`;
  }
}

/**
 * The value on one line, for the event log and the console. Undefined reads as "undefined". This never throws,
 * because it runs inside the game's own event emit: a value that cannot be read becomes a short marker.
 */
export function briefText(value: unknown): string {
  let text: string;
  try {
    const plain = plainOf(value, { maxDepth: LINE_DEPTH, depth: 0, ancestors: [] });
    text = plain === undefined ? "undefined" : (JSON.stringify(plain) ?? String(plain));
  } catch (error) {
    text = `[unreadable: ${messageOf(error)}]`;
  }
  return text.length > LINE_CHARS ? `${text.slice(0, LINE_CHARS - 1)}…` : text;
}

/** A number with a fixed count of decimals, or a dash when there is no finite number. */
export function formatFixed(value: number | null, digits: number): string {
  return value === null || !Number.isFinite(value) ? "–" : value.toFixed(digits);
}

/** The message of an error, or the text of anything else that was thrown. */
export function messageOf(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

/** Turns a value into plain JSON data within the walk's limits. Undefined stays undefined, so the caller can drop it. */
function plainOf(value: unknown, walk: Walk): unknown {
  if (typeof value !== "object" || value === null) return primitiveOf(value);
  if (walk.ancestors.includes(value)) return "[Circular]";
  if (walk.depth >= walk.maxDepth) return "[Depth limit]";
  if (value instanceof Date) return Number.isNaN(value.getTime()) ? "Invalid Date" : value.toISOString();
  if (ArrayBuffer.isView(value)) return `[${value.constructor.name} ${value.byteLength} bytes]`;

  const inner: Walk = { maxDepth: walk.maxDepth, depth: walk.depth + 1, ancestors: [...walk.ancestors, value] };
  if (value instanceof Map) {
    return Array.from(value, ([key, item]) => [plainOf(key, inner), plainOf(item, inner)]);
  }
  if (value instanceof Set) {
    return Array.from(value, (item: unknown) => plainOf(item, inner));
  }
  if (Array.isArray(value)) {
    const items: readonly unknown[] = value;
    return items.map((item) => plainOf(item, inner) ?? null);
  }

  const record: Record<string, unknown> = {};
  for (const key of Object.keys(value)) {
    let item: unknown;
    try {
      item = Reflect.get(value, key);
    } catch {
      record[key] = "[Unreadable]";
      continue;
    }
    const plain = plainOf(item, inner);
    if (plain !== undefined) record[key] = plain;
  }
  return record;
}

/** The JSON form of a primitive. Functions and symbols become text, since JSON has neither. */
function primitiveOf(value: unknown): unknown {
  switch (typeof value) {
    case "string":
    case "boolean":
      return value;
    case "number":
      return Number.isFinite(value) ? value : String(value);
    case "bigint":
      return `${value}n`;
    case "symbol":
      return value.toString();
    case "function":
      return `[Function ${value.name || "anonymous"}]`;
    case "undefined":
      return undefined;
    default:
      return null;
  }
}
