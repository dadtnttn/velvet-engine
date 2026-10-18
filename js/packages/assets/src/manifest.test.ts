// Tests for the manifest reader: the envelope, every field rule, the defaults, and the duplicate
// checks. Each refusal names its code, and a document with several problems reports all of them.

import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { ArtError, type ArtErrorCode } from "./error";
import { parseArtManifest } from "./manifest";
import { ART_KINDS, ART_MAX_SIDE } from "./types";

const FIXTURE = new URL("../../../../docs/formats/fixtures/art/manifest.json", import.meta.url);

/** A minimal valid document, with entries given by the test. */
function document(entries: unknown[], extra: Record<string, unknown> = {}): string {
  return JSON.stringify({ format: "velvet-art", version: 1, entries, ...extra });
}

/** Parses a document that must be refused, and returns the error. */
function refusal(text: string): ArtError {
  try {
    parseArtManifest(text);
  } catch (error) {
    if (error instanceof ArtError) return error;
    throw error;
  }
  throw new Error("the manifest was accepted but should have been refused");
}

/** The code of the first problem of a refused document. */
function codeOf(entries: unknown[]): ArtErrorCode {
  return refusal(document(entries)).code;
}

describe("parseArtManifest: the shipped fixture", () => {
  it("reads every entry, in the order of the file", () => {
    const manifest = parseArtManifest(readFileSync(FIXTURE, "utf8"));
    expect(manifest.format).toBe("velvet-art");
    expect(manifest.version).toBe(1);
    expect(manifest.entries.map((entry) => entry.id)).toEqual([
      "character.lyra.portrait",
      "character.lyra.fullbody",
      "card.wood",
      "icon.coin",
      "bg.forest",
      "prop.well",
    ]);
  });

  it("fills the defaults: anchor [0, 0], no variants, empty meta", () => {
    const manifest = parseArtManifest(readFileSync(FIXTURE, "utf8"));
    const card = manifest.entries.find((entry) => entry.id === "card.wood");
    expect(card).toEqual({ id: "card.wood", kind: "card", size: [180, 260], anchor: [0, 0], variants: [], meta: {} });
  });

  it("keeps the given anchor, variants and meta", () => {
    const manifest = parseArtManifest(readFileSync(FIXTURE, "utf8"));
    const portrait = manifest.entries[0];
    expect(portrait?.anchor).toEqual([0.5, 1]);
    expect(portrait?.variants).toEqual(["neutral", "happy", "worried"]);
    expect(portrait?.meta).toEqual({
      prompt: "a friendly young baker with flour on her apron, half length, looking at the viewer",
      style: ["soft lines", "warm light"],
    });
  });
});

describe("parseArtManifest: the envelope", () => {
  it("refuses text that is not JSON, with the parse error as its cause", () => {
    const error = refusal('{ "format": ');
    expect(error.code).toBe("json");
    expect(error.cause).toBeDefined();
  });

  it("refuses a document that is not a JSON object", () => {
    expect(refusal("[]").code).toBe("structure");
    expect(refusal("null").code).toBe("structure");
  });

  it("refuses another format tag and another version", () => {
    expect(refusal(JSON.stringify({ format: "velvet-stacks", version: 1, entries: [] })).code).toBe("format");
    expect(refusal(JSON.stringify({ format: "velvet-art", version: 2, entries: [] })).code).toBe("version");
    expect(refusal(JSON.stringify({ format: "velvet-art", entries: [] })).code).toBe("version");
  });

  it("refuses entries that are missing or not an array", () => {
    expect(refusal(JSON.stringify({ format: "velvet-art", version: 1 })).code).toBe("structure");
    expect(refusal(JSON.stringify({ format: "velvet-art", version: 1, entries: "none" })).code).toBe("structure");
  });

  it("accepts an empty manifest and ignores fields it does not know", () => {
    const manifest = parseArtManifest(document([], { note: "written by a later tool" }));
    expect(manifest).toEqual({ format: "velvet-art", version: 1, entries: [] });
  });

  it("ignores unknown fields inside an entry, and leaves them out of the result", () => {
    const manifest = parseArtManifest(document([{ id: "card.wood", kind: "card", size: [180, 260], rarity: 3 }]));
    expect(manifest.entries[0]).not.toHaveProperty("rarity");
  });
});

describe("parseArtManifest: entry fields", () => {
  it("accepts every kind in the format", () => {
    for (const kind of ART_KINDS) {
      const manifest = parseArtManifest(document([{ id: `${kind}.sample`, kind, size: [64, 64] }]));
      expect(manifest.entries[0]?.kind).toBe(kind);
    }
  });

  it("refuses a kind that the format does not list", () => {
    expect(codeOf([{ id: "card.wood", kind: "sprite", size: [180, 260] }])).toBe("value");
    expect(codeOf([{ id: "card.wood", kind: 7, size: [180, 260] }])).toBe("structure");
  });

  it("accepts the ids the grammar allows", () => {
    for (const id of ["a", "a.b_c-d", "bg.forest2", "a".repeat(128)]) {
      expect(parseArtManifest(document([{ id, kind: "prop", size: [1, 1] }])).entries[0]?.id).toBe(id);
    }
  });

  it("refuses ids the grammar does not allow", () => {
    for (const id of ["", "a..b", ".a", "a.", "A", "a b", "a/b", "a\\b", "x".repeat(129)]) {
      expect(codeOf([{ id, kind: "prop", size: [1, 1] }])).toBe("value");
    }
    expect(codeOf([{ kind: "prop", size: [1, 1] }])).toBe("structure");
    expect(codeOf([{ id: 3, kind: "prop", size: [1, 1] }])).toBe("structure");
  });

  it("accepts sizes of whole numbers from 1 to the maximum", () => {
    const manifest = parseArtManifest(document([{ id: "bg.wide", kind: "background", size: [ART_MAX_SIDE, 1] }]));
    expect(manifest.entries[0]?.size).toEqual([ART_MAX_SIDE, 1]);
  });

  it("refuses sizes that are not two whole numbers in range", () => {
    for (const size of [[0, 64], [64, 0], [1.5, 64], [64, ART_MAX_SIDE + 1], [64], [64, 64, 64], ["64", 64]]) {
      expect(codeOf([{ id: "icon.coin", kind: "icon", size }])).toBe("value");
    }
    expect(codeOf([{ id: "icon.coin", kind: "icon", size: 64 }])).toBe("structure");
    expect(codeOf([{ id: "icon.coin", kind: "icon" }])).toBe("structure");
  });

  it("accepts anchors from 0 to 1, both included, and refuses others", () => {
    expect(parseArtManifest(document([{ id: "a", kind: "prop", size: [1, 1], anchor: [0, 1] }])).entries[0]?.anchor).toEqual([0, 1]);
    for (const anchor of [[-0.1, 0], [0, 1.01], [0.5], [0.5, 0.5, 0.5], ["0.5", 0]]) {
      expect(codeOf([{ id: "a", kind: "prop", size: [1, 1], anchor }])).toBe("value");
    }
    expect(codeOf([{ id: "a", kind: "prop", size: [1, 1], anchor: 0.5 }])).toBe("structure");
  });

  it("accepts variant names of letters, digits, underscores and dashes", () => {
    const manifest = parseArtManifest(document([{ id: "card.wood", kind: "card", size: [1, 1], variants: ["day_1", "night-2"] }]));
    expect(manifest.entries[0]?.variants).toEqual(["day_1", "night-2"]);
    expect(parseArtManifest(document([{ id: "card.wood", kind: "card", size: [1, 1], variants: [] }])).entries[0]?.variants).toEqual([]);
  });

  it("refuses variant names that break the grammar, and names that are not strings", () => {
    for (const variants of [[""], ["happy.png"], ["Happy"], ["x".repeat(33)]]) {
      expect(codeOf([{ id: "card.wood", kind: "card", size: [1, 1], variants }])).toBe("value");
    }
    expect(codeOf([{ id: "card.wood", kind: "card", size: [1, 1], variants: [1] }])).toBe("structure");
    expect(codeOf([{ id: "card.wood", kind: "card", size: [1, 1], variants: "happy" }])).toBe("structure");
  });

  it("refuses meta that is not an object", () => {
    expect(codeOf([{ id: "card.wood", kind: "card", size: [1, 1], meta: "text" }])).toBe("structure");
    expect(codeOf([{ id: "card.wood", kind: "card", size: [1, 1], meta: null }])).toBe("structure");
  });

  it("refuses an entry that is not an object", () => {
    expect(codeOf(["card.wood"])).toBe("structure");
  });
});

describe("parseArtManifest: duplicates", () => {
  it("refuses two entries with the same id", () => {
    const error = refusal(
      document([
        { id: "card.wood", kind: "card", size: [1, 1] },
        { id: "card.wood", kind: "card", size: [2, 2] },
      ]),
    );
    expect(error.code).toBe("duplicate");
    expect(error.issues[0]?.path).toBe("entries[1].id");
  });

  it("refuses a variant listed twice", () => {
    expect(codeOf([{ id: "card.wood", kind: "card", size: [1, 1], variants: ["red", "red"] }])).toBe("duplicate");
  });

  it("refuses an entry whose file name is another entry's file name", () => {
    const clash = document([
      { id: "card.wood", kind: "card", size: [1, 1], variants: ["red"] },
      { id: "card.wood.red", kind: "card", size: [1, 1] },
    ]);
    expect(refusal(clash).code).toBe("duplicate");
  });

  it("allows the same file name in two different kinds, since they are different folders", () => {
    const manifest = parseArtManifest(
      document([
        { id: "coin", kind: "icon", size: [64, 64], variants: ["gold"] },
        { id: "coin.gold", kind: "prop", size: [64, 64] },
      ]),
    );
    expect(manifest.entries).toHaveLength(2);
  });
});

describe("parseArtManifest: problems are reported together", () => {
  it("lists every problem, each with its path, and names the first one's code", () => {
    const error = refusal(
      document([
        { id: "Bad.Id", kind: "card", size: [1, 1] },
        { id: "ok", kind: "sprite", size: [1, 1] },
        { id: "fine", kind: "card", size: [0, 1] },
      ]),
    );
    expect(error.code).toBe("value");
    expect(error.issues.map((issue) => issue.path)).toEqual(["entries[0].id", "entries[1].kind", "entries[2].size"]);
    expect(error.message).toContain("3 problems");
  });

  it("gives a single issue for a single failure", () => {
    const error = refusal(JSON.stringify({ format: "velvet-stacks", version: 1, entries: [] }));
    expect(error.issues).toEqual([{ code: "format", message: 'the format tag is not "velvet-art"', path: "format" }]);
  });
});
