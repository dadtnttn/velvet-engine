// Tests for the state inspector's tree and the JSON copy. The tree must open lazily, never follow a
// cycle, page long lists, cap the depth, and never read a getter of a row that is closed.

import { describe, expect, it } from "vitest";
import { MAX_DEPTH, PAGE_SIZE, buildTree, type TreeRow } from "./inspect";
import { copyJson } from "./text";

/** Opens every row that can open, until no more rows open. */
function openAll(value: unknown): Set<string> {
  const expanded = new Set<string>([""]);
  for (;;) {
    const rows = buildTree(value, { expanded, pages: new Map() });
    const before = expanded.size;
    for (const row of rows) if (row.expandable) expanded.add(row.id);
    if (expanded.size === before) return expanded;
  }
}

/** The row whose label is the given text. */
function rowLabeled(rows: readonly TreeRow[], label: string): TreeRow {
  const row = rows.find((candidate) => candidate.label === label);
  if (row === undefined) throw new Error(`no row labeled ${label}`);
  return row;
}

describe("buildTree", () => {
  it("shows the root and its direct children, and leaves the children closed", () => {
    const rows = buildTree({ a: 1, b: { c: 2 } }, { expanded: new Set([""]), pages: new Map() });
    expect(rows.map((row) => row.label)).toEqual(["", "a", "b"]);
    expect(rowLabeled(rows, "b")).toMatchObject({ kind: "object", expandable: true, expanded: false });
    expect(rowLabeled(rows, "a")).toMatchObject({ kind: "value", preview: "1", expandable: false });
  });

  it("reads the children of a row only when that row is open, and never calls a getter", () => {
    let reads = 0;
    const data = {
      nested: {
        get hidden() {
          reads += 1;
          return 1;
        },
      },
    };
    const closed = buildTree(data, { expanded: new Set([""]), pages: new Map() });
    expect(reads).toBe(0);
    const nestedId = rowLabeled(closed, "nested").id;

    const open = buildTree(data, { expanded: new Set(["", nestedId]), pages: new Map() });
    expect(rowLabeled(open, "hidden")).toMatchObject({ kind: "getter", preview: "(getter, not read)" });
    expect(reads).toBe(0);
  });

  it("marks a cycle and does not follow it", () => {
    const node: Record<string, unknown> = { name: "root" };
    node["self"] = node;
    const rows = buildTree(node, { expanded: openAll(node), pages: new Map() });
    expect(rowLabeled(rows, "self")).toMatchObject({ kind: "circular", preview: "[Circular]", expandable: false });
    expect(rows.length).toBeLessThan(10);
  });

  it("shows one page of a long list, then a row that says how many are left", () => {
    const list = Array.from({ length: 250 }, (_, index) => index);
    const data = { list };
    const listId = rowLabeled(buildTree(data, { expanded: new Set([""]), pages: new Map() }), "list").id;
    const expanded = new Set(["", listId]);

    // The "more" row sits at the depth of the list's items, so it is counted apart from them.
    const itemsOf = (rows: readonly TreeRow[]): TreeRow[] => rows.filter((row) => row.depth === 2 && row.kind !== "more");
    const firstPage = buildTree(data, { expanded, pages: new Map() });
    const more = firstPage.find((row) => row.kind === "more");
    expect(itemsOf(firstPage)).toHaveLength(PAGE_SIZE);
    expect(more).toMatchObject({ parentId: listId, shown: PAGE_SIZE, total: 250 });

    const secondPage = buildTree(data, { expanded, pages: new Map([[listId, 2 * PAGE_SIZE]]) });
    expect(itemsOf(secondPage)).toHaveLength(2 * PAGE_SIZE);
    expect(secondPage.find((row) => row.kind === "more")).toMatchObject({ shown: 2 * PAGE_SIZE, total: 250 });
  });

  it("caps the depth, and says so on the deepest row", () => {
    let value: Record<string, unknown> = { leaf: true };
    for (let level = 0; level < MAX_DEPTH + 4; level += 1) value = { next: value };
    const rows = buildTree(value, { expanded: openAll(value), pages: new Map() });
    const deepest = Math.max(...rows.map((row) => row.depth));
    expect(deepest).toBe(MAX_DEPTH);
    const last = rows[rows.length - 1];
    expect(last?.preview.endsWith("…")).toBe(true);
    expect(last?.expandable).toBe(false);
  });

  it("gives each kind of value a short, readable preview", () => {
    class Thing {
      readonly value = 1;
    }
    function namedFn(): void {
      return;
    }
    const cases: Array<[unknown, string]> = [
      ["hi", '"hi"'],
      [3, "3"],
      [true, "true"],
      [null, "null"],
      [undefined, "undefined"],
      [Number.NaN, "NaN"],
      [namedFn, "ƒ namedFn()"],
      [new Map([["a", 1]]), "Map(1)"],
      [new Set([1]), "Set(1)"],
      [new Thing(), "Thing {1 key}"],
      [new Date(0), "1970-01-01T00:00:00.000Z"],
      [new Error("boom"), "Error: boom"],
      [{ a: 1, b: 2 }, "{2 keys}"],
    ];
    for (const [value, preview] of cases) {
      expect(buildTree(value, { expanded: new Set(), pages: new Map() })[0]?.preview).toBe(preview);
    }
  });

  it("shows a value that cannot be read as an error row, and does not throw", () => {
    const hostile = new Proxy(
      {},
      {
        ownKeys() {
          throw new Error("no keys for you");
        },
      },
    );
    const rows = buildTree(hostile, { expanded: new Set([""]), pages: new Map() });
    expect(rows).toHaveLength(1);
    expect(rows[0]?.preview).toContain("unreadable");
  });
});

describe("copyJson", () => {
  it("writes indented JSON", () => {
    expect(copyJson({ a: 1 })).toBe('{\n  "a": 1\n}');
  });

  it("writes a cycle as a marker, and a function as its name", () => {
    const node: Record<string, unknown> = {};
    node["self"] = node;
    expect(JSON.parse(copyJson(node))).toEqual({ self: "[Circular]" });
    expect(JSON.parse(copyJson({ run: function go() { return 1; } }))).toEqual({ run: "[Function go]" });
  });

  it("returns text, and does not throw, for a value whose keys cannot be listed", () => {
    const hostile = new Proxy(
      {},
      {
        ownKeys() {
          throw new Error("no keys");
        },
      },
    );
    expect(() => copyJson(hostile)).not.toThrow();
    expect(copyJson(hostile)).toContain("Unreadable");
  });

  it("writes a Map and a Set as lists, and NaN as text", () => {
    expect(JSON.parse(copyJson(new Map([["k", 1]])))).toEqual([["k", 1]]);
    expect(JSON.parse(copyJson(new Set([1, 2])))).toEqual([1, 2]);
    expect(JSON.parse(copyJson({ n: Number.NaN }))).toEqual({ n: "NaN" });
  });

  it("leaves out undefined properties, and writes an undefined top-level value as null", () => {
    expect(JSON.parse(copyJson({ a: undefined, b: 1 }))).toEqual({ b: 1 });
    expect(copyJson(undefined)).toBe("null");
  });

  it("writes a getter that throws as unreadable, and stops at the depth limit", () => {
    const throwing = {
      get bad(): number {
        throw new Error("no");
      },
    };
    expect(JSON.parse(copyJson(throwing))).toEqual({ bad: "[Unreadable]" });

    let deep: Record<string, unknown> = {};
    const top = deep;
    for (let level = 0; level < 45; level += 1) {
      const next: Record<string, unknown> = {};
      deep["next"] = next;
      deep = next;
    }
    expect(copyJson(top)).toContain("[Depth limit]");
  });
});
