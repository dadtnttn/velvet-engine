// The state inspector's tree. buildTree turns a value into rows for its open part only: a closed object is
// not read, so its getters are never called. Cycles show as a marker, arrays and collections show one page
// of children at a time, and the depth is capped. Building the tree never throws, so a strange value
// cannot break the game.

import { briefText } from "./text";

/** Children shown per row before its "show more" row appears. */
export const PAGE_SIZE = 100;

/** The deepest level that can be opened. */
export const MAX_DEPTH = 8;

/** Longest string preview, in characters. */
const STRING_CHARS = 200;

/** Joins the keys of a row id. A game does not use it in its own keys. */
const ID_SEPARATOR = "\u001f";

/** What a row holds, which decides how it is styled. */
export type RowKind = "value" | "object" | "array" | "map" | "set" | "circular" | "getter" | "more";

/** One row of the tree. */
export interface TreeRow {
  /** Identifies the row across refreshes: the keys from the root, joined. The root is "". */
  readonly id: string;
  /** The nesting level. The root is 0. */
  readonly depth: number;
  /** The key, index or map key that labels the row. Empty for the root. */
  readonly label: string;
  /** What the row holds. */
  readonly kind: RowKind;
  /** The value as one short line. */
  readonly preview: string;
  /** True when the row can be opened: it has children, and it is not too deep. */
  readonly expandable: boolean;
  /** True when the row is open, so its children follow it. */
  readonly expanded: boolean;
  /** For a "more" row: the id of the row whose children are paged. */
  readonly parentId?: string;
  /** For a "more" row: how many children are shown. */
  readonly shown?: number;
  /** For a "more" row: how many children there are in all. */
  readonly total?: number;
}

/** Options for {@link buildTree}. */
export interface TreeOptions {
  /** The ids of the rows that are open. The root is "". */
  readonly expanded: ReadonlySet<string>;
  /** How many children each row shows, by row id, once its "show more" row was used. */
  readonly pages: ReadonlyMap<string, number>;
}

/** The result of a regular inspection of one value, without its children. */
interface Described {
  readonly kind: RowKind;
  readonly preview: string;
  readonly hasChildren: boolean;
}

/** One child of a row. A getter is listed but not read. */
interface Child {
  readonly key: string;
  readonly label: string;
  readonly value: unknown;
  readonly isGetter: boolean;
}

/** The children that a row shows now, and how many it has in all. */
interface Children {
  readonly items: Child[];
  readonly total: number;
}

/**
 * The rows of the tree for a value: the root, then the children of each open row, in order. A row is open
 * when its id is in `options.expanded`.
 */
export function buildTree(root: unknown, options: TreeOptions): TreeRow[] {
  const rows: TreeRow[] = [];

  const visit = (value: unknown, id: string, depth: number, label: string, ancestors: readonly object[]): void => {
    const isCycle = isObjectValue(value) && ancestors.includes(value);
    let described: Described;
    try {
      described = isCycle ? { kind: "circular", preview: "[Circular]", hasChildren: false } : describe(value);
    } catch (error) {
      rows.push({ id, depth, label, kind: "value", preview: `[unreadable: ${String(error)}]`, expandable: false, expanded: false });
      return;
    }

    // Children are read only for a row that can open and is open, so a closed row costs nothing.
    const canOpen = !isCycle && described.hasChildren && depth < MAX_DEPTH;
    let children: Children | null = null;
    let unreadable = false;
    if (canOpen && options.expanded.has(id) && isObjectValue(value)) {
      try {
        children = childrenOf(value, options.pages.get(id) ?? PAGE_SIZE);
      } catch {
        unreadable = true;
      }
    }
    let preview = described.preview;
    if (!isCycle && described.hasChildren && !canOpen) preview += " …";
    if (unreadable) preview += " [unreadable]";
    rows.push({ id, depth, label, kind: described.kind, preview, expandable: canOpen, expanded: children !== null });
    if (children === null || !isObjectValue(value)) return;

    const inner = [...ancestors, value];
    for (const child of children.items) {
      const childId = `${id}${ID_SEPARATOR}${child.key}`;
      if (child.isGetter) {
        rows.push({
          id: childId,
          depth: depth + 1,
          label: child.label,
          kind: "getter",
          preview: "(getter, not read)",
          expandable: false,
          expanded: false,
        });
      } else {
        visit(child.value, childId, depth + 1, child.label, inner);
      }
    }
    if (children.total > children.items.length) {
      rows.push({
        id: `${id}${ID_SEPARATOR}${ID_SEPARATOR}more`,
        depth: depth + 1,
        label: "",
        kind: "more",
        preview: "",
        expandable: false,
        expanded: false,
        parentId: id,
        shown: children.items.length,
        total: children.total,
      });
    }
  };

  visit(root, "", 0, "", []);
  return rows;
}

/** True for a non-null object, which is the only kind of value that can have children or a cycle. */
function isObjectValue(value: unknown): value is object {
  return typeof value === "object" && value !== null;
}

/** What a value looks like on its row, and whether it has children, without reading any of them. */
function describe(value: unknown): Described {
  if (!isObjectValue(value)) {
    return { kind: "value", preview: primitivePreview(value), hasChildren: false };
  }
  if (Array.isArray(value)) {
    return { kind: "array", preview: `Array(${value.length})`, hasChildren: value.length > 0 };
  }
  if (value instanceof Map) {
    return { kind: "map", preview: `Map(${value.size})`, hasChildren: value.size > 0 };
  }
  if (value instanceof Set) {
    return { kind: "set", preview: `Set(${value.size})`, hasChildren: value.size > 0 };
  }
  if (value instanceof Date) {
    return { kind: "value", preview: Number.isNaN(value.getTime()) ? "Invalid Date" : value.toISOString(), hasChildren: false };
  }
  if (value instanceof Error) {
    return { kind: "value", preview: `${value.name}: ${value.message}`, hasChildren: false };
  }
  if (ArrayBuffer.isView(value)) {
    return { kind: "value", preview: `${value.constructor.name}(${value.byteLength} bytes)`, hasChildren: false };
  }
  const keys = Object.keys(value);
  const name = constructorName(value);
  const count = `{${keys.length} ${keys.length === 1 ? "key" : "keys"}}`;
  return { kind: "object", preview: name === "Object" ? count : `${name} ${count}`, hasChildren: keys.length > 0 };
}

/** The one-line text of a primitive value. */
function primitivePreview(value: unknown): string {
  switch (typeof value) {
    case "undefined":
      return "undefined";
    case "string":
      return stringPreview(value);
    case "bigint":
      return `${value}n`;
    case "symbol":
      return value.toString();
    case "function":
      return `ƒ ${value.name || "anonymous"}()`;
    default:
      return value === null ? "null" : String(value);
  }
}

/** A string in quotes, shortened when it is long, with its length when it was cut. */
function stringPreview(text: string): string {
  if (text.length <= STRING_CHARS) return JSON.stringify(text);
  return `${JSON.stringify(text.slice(0, STRING_CHARS))}… (${text.length} characters)`;
}

/** The name of the value's class, or "Object" for a plain object or one without a prototype. */
function constructorName(value: object): string {
  const proto: unknown = Object.getPrototypeOf(value);
  if (typeof proto !== "object" || proto === null) return "Object";
  const ctor: unknown = Reflect.get(proto, "constructor");
  return typeof ctor === "function" && ctor.name !== "" ? ctor.name : "Object";
}

/** The first `limit` children of a value: elements, entries or own properties. */
function childrenOf(value: object, limit: number): Children {
  if (Array.isArray(value)) {
    const list: readonly unknown[] = value;
    const shown = Math.min(list.length, limit);
    const items: Child[] = [];
    for (let index = 0; index < shown; index += 1) {
      items.push({ key: String(index), label: String(index), value: list[index], isGetter: false });
    }
    return { items, total: list.length };
  }
  if (value instanceof Map) {
    const items: Child[] = [];
    for (const [key, item] of value) {
      if (items.length >= limit) break;
      items.push({ key: `#${items.length}`, label: briefText(key), value: item, isGetter: false });
    }
    return { items, total: value.size };
  }
  if (value instanceof Set) {
    const items: Child[] = [];
    for (const item of value) {
      if (items.length >= limit) break;
      items.push({ key: `#${items.length}`, label: `#${items.length}`, value: item, isGetter: false });
    }
    return { items, total: value.size };
  }
  const keys = Object.keys(value);
  const items = keys.slice(0, limit).map((key) => propertyChild(value, key));
  return { items, total: keys.length };
}

/** A property as a child. An accessor property is not read: its value would run the getter. */
function propertyChild(object: object, key: string): Child {
  const descriptor = Object.getOwnPropertyDescriptor(object, key);
  const isGetter = descriptor !== undefined && ("get" in descriptor || "set" in descriptor);
  const value: unknown = descriptor?.value;
  return { key, label: key, value, isGetter };
}
