// Tests for ArtResolver beyond the shared fixtures: the base URL, caching, frozen results, the
// accepted formats, the broken-file rule, and the errors for unknown ids and variants.

import { describe, expect, it } from "vitest";
import { ArtError } from "./error";
import { parseArtManifest } from "./manifest";
import { ArtResolver } from "./resolver";
import type { ArtManifest } from "./types";

const MANIFEST: ArtManifest = parseArtManifest(
  JSON.stringify({
    format: "velvet-art",
    version: 1,
    entries: [
      { id: "character.lyra.portrait", kind: "portrait", size: [480, 640], anchor: [0.5, 1], variants: ["happy"] },
      { id: "card.wood", kind: "card", size: [180, 260] },
    ],
  }),
);

describe("ArtResolver: the URL of a file", () => {
  it("adds the folder to the path, with art/ as the default base", () => {
    const resolver = new ArtResolver(MANIFEST, ["card/card.wood.png"]);
    expect(resolver.resolve("card.wood").url).toBe("art/card/card.wood.png");
  });

  it("uses the base URL it is given, with or without a trailing slash", () => {
    const withSlash = new ArtResolver(MANIFEST, ["card/card.wood.png"], { baseUrl: "velvet://game/art/" });
    const without = new ArtResolver(MANIFEST, ["card/card.wood.png"], { baseUrl: "velvet://game/art" });
    expect(withSlash.resolve("card.wood").url).toBe("velvet://game/art/card/card.wood.png");
    expect(without.resolve("card.wood").url).toBe("velvet://game/art/card/card.wood.png");
  });

  it("gives a bare relative path when the base URL is empty", () => {
    const resolver = new ArtResolver(MANIFEST, ["card/card.wood.png"], { baseUrl: "" });
    expect(resolver.resolve("card.wood").url).toBe("card/card.wood.png");
  });
});

describe("ArtResolver: what a resolution holds", () => {
  it("reports the kind, size, anchor, source and path of the file", () => {
    const resolver = new ArtResolver(MANIFEST, ["portrait/character.lyra.portrait.happy.webp"]);
    expect(resolver.resolve("character.lyra.portrait", "happy")).toEqual({
      id: "character.lyra.portrait",
      variant: "happy",
      kind: "portrait",
      size: [480, 640],
      anchor: [0.5, 1],
      source: "webp",
      url: "art/portrait/character.lyra.portrait.happy.webp",
      path: "portrait/character.lyra.portrait.happy.webp",
    });
  });

  it("returns the same frozen object on repeated calls, and nothing that the caller can change", () => {
    const resolver = new ArtResolver(MANIFEST, ["card/card.wood.png"]);
    const first = resolver.resolve("card.wood");
    expect(resolver.resolve("card.wood")).toBe(first);
    expect(Object.isFrozen(first)).toBe(true);
  });

  it("gives a placeholder a data URL of SVG, with no file path", () => {
    const picture = new ArtResolver(MANIFEST, []).resolve("card.wood");
    expect(picture.source).toBe("placeholder");
    expect(picture.path).toBeNull();
    expect(picture.variant).toBeNull();
    expect(picture.url.startsWith("data:image/svg+xml;charset=utf-8,")).toBe(true);
    expect(decodeURIComponent(picture.url.slice("data:image/svg+xml;charset=utf-8,".length))).toContain('aria-label="wood"');
  });
});

describe("ArtResolver: accepted formats", () => {
  it("skips the formats that are not accepted, and keeps the order of the rest", () => {
    const files = ["card/card.wood.avif", "card/card.wood.png"];
    expect(new ArtResolver(MANIFEST, files, { accept: ["webp", "png"] }).resolve("card.wood").source).toBe("png");
    expect(new ArtResolver(MANIFEST, files).resolve("card.wood").source).toBe("avif");
  });

  it("shows only placeholders when no format is accepted", () => {
    const resolver = new ArtResolver(MANIFEST, ["card/card.wood.png"], { accept: [] });
    expect(resolver.resolve("card.wood").source).toBe("placeholder");
  });
});

describe("ArtResolver: a file that the browser cannot show", () => {
  it("is skipped by later resolutions, which fall to the next format", () => {
    const resolver = new ArtResolver(MANIFEST, ["card/card.wood.avif", "card/card.wood.webp"]);
    expect(resolver.resolve("card.wood").source).toBe("avif");
    resolver.markBroken("card/card.wood.avif");
    expect(resolver.resolve("card.wood").source).toBe("webp");
  });

  it("falls to the placeholder when no other file is left", () => {
    const resolver = new ArtResolver(MANIFEST, ["card/card.wood.png"]);
    resolver.markBroken("card/card.wood.png");
    expect(resolver.resolve("card.wood").source).toBe("placeholder");
  });

  it("is named by the same path, however the caller spells it", () => {
    const resolver = new ArtResolver(MANIFEST, ["card/card.wood.png"]);
    resolver.markBroken("card\\card.wood.png");
    expect(resolver.resolve("card.wood").source).toBe("placeholder");
  });

  it("counts as missing in the report, as if the file were never there", () => {
    const resolver = new ArtResolver(MANIFEST, ["card/card.wood.png", "portrait/character.lyra.portrait.png"]);
    resolver.markBroken("card/card.wood.png");
    expect(resolver.missingReport().missing).toEqual([
      { id: "card.wood", kind: "card", size: [180, 260], base: false, missing_variants: [] },
      { id: "character.lyra.portrait", kind: "portrait", size: [480, 640], base: true, missing_variants: ["happy"] },
    ]);
  });
});

describe("ArtResolver: the report envelope", () => {
  it("carries the report format and version, and lists nothing when everything is present", () => {
    const report = new ArtResolver(MANIFEST, [
      "portrait/character.lyra.portrait.png",
      "portrait/character.lyra.portrait.happy.png",
      "card/card.wood.png",
    ]).missingReport();
    expect(report).toEqual({ format: "velvet-art-report", version: 1, missing: [] });
  });

  it("gives the size and the kind of each id that needs art", () => {
    const report = new ArtResolver(MANIFEST, []).missingReport();
    expect(report.missing[0]).toEqual({
      id: "card.wood",
      kind: "card",
      size: [180, 260],
      base: false,
      missing_variants: [],
    });
  });
});

/** The code of the ArtError that a call throws. Fails the test when the call throws anything else, or nothing. */
function codeThrown(call: () => unknown): string {
  try {
    call();
  } catch (error) {
    if (error instanceof ArtError) return error.code;
    throw error;
  }
  throw new Error("the call did not throw");
}

describe("ArtResolver: errors", () => {
  it("throws unknown-id for an id that the manifest does not have (strict)", () => {
    const resolver = new ArtResolver(MANIFEST, [], { strict: true });
    expect(codeThrown(() => resolver.resolve("character.mara.portrait"))).toBe("unknown-id");
  });

  it("throws unknown-variant for a variant the id does not declare, including an empty name (strict)", () => {
    const resolver = new ArtResolver(MANIFEST, [], { strict: true });
    expect(codeThrown(() => resolver.resolve("character.lyra.portrait", "sad"))).toBe("unknown-variant");
    expect(codeThrown(() => resolver.resolve("character.lyra.portrait", ""))).toBe("unknown-variant");
  });

  it("by default draws a placeholder for an unknown id and tells onUnknown once", () => {
    const told: string[] = [];
    const resolver = new ArtResolver(MANIFEST, [], { onUnknown: (error) => told.push(error.code) });
    const first = resolver.resolve("character.mara.portrait");
    resolver.resolve("character.mara.portrait");
    expect(first.source).toBe("placeholder");
    expect(first.size).toEqual([128, 128]);
    expect(told).toEqual(["unknown-id"]);
  });

  it("by default falls back to the base picture for an unknown variant and tells onUnknown once", () => {
    const told: string[] = [];
    const resolver = new ArtResolver(MANIFEST, ["portrait/character.lyra.portrait.webp"], { onUnknown: (error) => told.push(error.code) });
    const art = resolver.resolve("character.lyra.portrait", "sad");
    resolver.resolve("character.lyra.portrait", "sad");
    expect(art.source).toBe("webp");
    expect(art.variant).toBeNull();
    expect(told).toEqual(["unknown-variant"]);
  });

  it("refuses a hand-built manifest that repeats an id", () => {
    const [card] = MANIFEST.entries.slice(1);
    if (card === undefined) throw new Error("the test manifest lost its card entry");
    const repeated: ArtManifest = { format: "velvet-art", version: 1, entries: [card, card] };
    expect(codeThrown(() => new ArtResolver(repeated, []))).toBe("duplicate");
  });
});
