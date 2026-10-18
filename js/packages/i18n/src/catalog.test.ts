// Tests for reading one catalog file: its header, its meta block and its entries.

import { describe, expect, it } from "vitest";
import { parseCatalog } from "./catalog";

/** Builds the text of a catalog file from its entries and optional extra fields. */
function file(entries: unknown, extra: Record<string, unknown> = {}): string {
  return JSON.stringify({ format: "velvet-locale", version: 1, language: "en", ...extra, entries });
}

describe("parseCatalog: the file as a whole", () => {
  it("reads a valid file with its meta block and its entries, and ignores the note", () => {
    const parsed = parseCatalog(
      file({ "menu.start": "Start", "shop.coins": { one: "{count} coin", other: "{count} coins" } }, {
        note: "anything",
        meta: { name: "English", script: "Latn", direction: "ltr", font: "sans", shaping: false },
      }),
      "00_menus.json",
    );
    expect(parsed.usable).toBe(true);
    expect(parsed.language).toBe("en");
    expect(parsed.meta).toEqual({ name: "English", script: "Latn", direction: "ltr", font: "sans", shaping: false });
    expect([...parsed.entries.keys()].sort()).toEqual(["menu.start", "shop.coins"]);
    expect(parsed.issues).toEqual([]);
  });

  it("refuses text that is not JSON, including text that starts with a byte order mark", () => {
    for (const text of ["{ nope", "﻿" + file({})]) {
      const parsed = parseCatalog(text, "a.json");
      expect(parsed.usable).toBe(false);
      expect(parsed.issues.map((issue) => issue.code)).toEqual(["json"]);
    }
  });

  it("refuses the wrong format, the wrong version and a missing or bad language, one at a time", () => {
    const cases: [string, string][] = [
      ['{"format":"other","version":1,"language":"en","entries":{}}', "format"],
      ['{"format":"velvet-locale","version":"1","language":"en","entries":{}}', "version"],
      ['{"format":"velvet-locale","version":1,"entries":{}}', "language"],
      ['{"format":"velvet-locale","version":1,"language":"en-gb","entries":{}}', "language"],
      ['{"format":"velvet-locale","version":1,"language":"en"}', "structure"],
      ["[]", "structure"],
    ];
    for (const [text, code] of cases) {
      const parsed = parseCatalog(text, "a.json");
      expect(parsed.usable, text).toBe(false);
      expect(parsed.issues.map((issue) => issue.code), text).toEqual([code]);
      expect(parsed.entries.size).toBe(0);
    }
  });

  it("warns about unknown top-level fields and keeps the file", () => {
    const parsed = parseCatalog(file({ "a.b": "x" }, { extra: 1 }));
    expect(parsed.usable).toBe(true);
    expect(parsed.issues).toEqual([expect.objectContaining({ severity: "warning", code: "unknown-field" })]);
  });
});

describe("parseCatalog: the meta block", () => {
  it("reports each bad field as an error and ignores only that field", () => {
    const parsed = parseCatalog(
      file({ "a.b": "x" }, { meta: { direction: "up", script: "latin", shaping: "yes", name: "Ok", flavour: 1 } }),
    );
    expect(parsed.meta).toEqual({ name: "Ok" });
    const codes = parsed.issues.map((issue) => `${issue.severity}:${issue.code}`).sort();
    expect(codes).toEqual(["error:meta", "error:meta", "error:meta", "warning:unknown-field"]);
  });

  it("reports a meta block that is not an object, and goes on without it", () => {
    const parsed = parseCatalog(file({ "a.b": "x" }, { meta: ["en"] }));
    expect(parsed.usable).toBe(true);
    expect(parsed.meta).toEqual({});
    expect(parsed.issues.map((issue) => issue.code)).toEqual(["meta"]);
  });
});

describe("parseCatalog: the entries", () => {
  it("keeps a plain string as an entry with only the other form", () => {
    const entry = parseCatalog(file({ "a.b": "Hi {name}" })).entries.get("a.b");
    expect(entry?.value).toBe("Hi {name}");
    expect([...(entry?.forms.keys() ?? [])]).toEqual(["other"]);
    expect([...(entry?.holes ?? [])]).toEqual(["name"]);
  });

  it("keeps a plural object with its forms, and the holes of all forms together", () => {
    const entry = parseCatalog(file({ "a.b": { one: "{count} item", other: "{count} items of {kind}" } })).entries.get("a.b");
    expect([...(entry?.forms.keys() ?? [])].sort()).toEqual(["one", "other"]);
    expect([...(entry?.holes ?? [])].sort()).toEqual(["count", "kind"]);
  });

  it("drops an entry of the wrong type, a bad plural object or a bad template, and keeps the others", () => {
    const parsed = parseCatalog(
      file({
        good: "Good",
        number: 4,
        noOther: { one: "x" },
        category: { other: "x", some: "y" },
        broken: "{0",
      }),
    );
    expect([...parsed.entries.keys()]).toEqual(["good"]);
    expect(parsed.issues.map((issue) => `${issue.code}:${issue.key}`).sort()).toEqual([
      "plural:category",
      "plural:noOther",
      "template:broken",
      "value:number",
    ]);
  });

  it("drops an empty text for a key with letters, but keeps an empty text for punctuation", () => {
    const parsed = parseCatalog(file({ "menu.empty": "", "¡": "", "menu.full": "Full" }));
    expect([...parsed.entries.keys()].sort()).toEqual(["menu.full", "¡"]);
    expect(parsed.issues).toEqual([expect.objectContaining({ severity: "warning", code: "empty", key: "menu.empty" })]);
  });

  it("drops a plural object whose form is empty for a key with letters", () => {
    const parsed = parseCatalog(file({ "a.b": { one: "", other: "{count} items" } }));
    expect(parsed.entries.has("a.b")).toBe(false);
    expect(parsed.issues.map((issue) => issue.code)).toEqual(["empty"]);
  });
});
