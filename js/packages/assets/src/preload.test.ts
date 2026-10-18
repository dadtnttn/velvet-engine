// Tests for preloading. Node has no Image, so the browser path runs against two fake image classes:
// one that decodes with decode(), as modern browsers do, and one that reports through the load and
// error events, as older ones do.

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { parseArtManifest } from "./manifest";
import type { PreloadProgress } from "./preload";
import { ArtResolver } from "./resolver";

const MANIFEST = parseArtManifest(
  JSON.stringify({
    format: "velvet-art",
    version: 1,
    entries: [
      { id: "card.wood", kind: "card", size: [180, 260] },
      { id: "prop.well", kind: "prop", size: [128, 160] },
      { id: "bg.forest", kind: "background", size: [1920, 1080], variants: ["night"] },
    ],
  }),
);

/** What the fake images share with a test: the URLs they were asked to decode, and the ones that fail. */
const decoder = {
  requested: [] as string[],
  failing: new Set<string>(),
};

/** An image that decodes with decode(). */
class DecodingImage {
  src = "";

  decode(): Promise<void> {
    decoder.requested.push(this.src);
    return decoder.failing.has(this.src) ? Promise.reject(new Error("cannot decode")) : Promise.resolve();
  }
}

/** An image that reports through the load and error events only. */
class EventImage {
  onload: (() => void) | null = null;
  onerror: (() => void) | null = null;
  private url = "";

  get src(): string {
    return this.url;
  }

  set src(value: string) {
    this.url = value;
    decoder.requested.push(value);
    queueMicrotask(() => {
      if (decoder.failing.has(value)) {
        this.onerror?.();
      } else {
        this.onload?.();
      }
    });
  }
}

beforeEach(() => {
  decoder.requested.length = 0;
  decoder.failing.clear();
});

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("preload without a browser", () => {
  it("decodes nothing and reports each distinct request as done", async () => {
    const resolver = new ArtResolver(MANIFEST, ["card/card.wood.webp"]);
    const progress: PreloadProgress[] = [];
    const result = await resolver.preload(["card.wood", "card.wood", { id: "prop.well" }], (step) => progress.push(step));
    expect(result).toEqual({ broken: [] });
    expect(progress).toEqual([
      { done: 1, total: 2, id: "card.wood", variant: null, source: "webp" },
      { done: 2, total: 2, id: "prop.well", variant: null, source: "placeholder" },
    ]);
  });

  it("reports the variant of a request", async () => {
    const resolver = new ArtResolver(MANIFEST, ["background/bg.forest.night.png"]);
    const progress: PreloadProgress[] = [];
    await resolver.preload([{ id: "bg.forest", variant: "night" }], (step) => progress.push(step));
    expect(progress).toEqual([{ done: 1, total: 1, id: "bg.forest", variant: "night", source: "png" }]);
  });

  it("checks every request before it reports anything (strict)", async () => {
    const resolver = new ArtResolver(MANIFEST, [], { strict: true });
    const progress: PreloadProgress[] = [];
    await expect(resolver.preload(["card.wood", "character.mara.portrait"], (step) => progress.push(step))).rejects.toMatchObject({
      code: "unknown-id",
    });
    expect(progress).toEqual([]);
  });

  it("resolves at once for an empty list", async () => {
    const resolver = new ArtResolver(MANIFEST, []);
    expect(await resolver.preload([])).toEqual({ broken: [] });
  });
});

describe("preload in a browser", () => {
  it("decodes the files that its requests resolve to, and no placeholder", async () => {
    vi.stubGlobal("Image", DecodingImage);
    const resolver = new ArtResolver(MANIFEST, ["card/card.wood.webp", "background/bg.forest.night.png"]);
    const result = await resolver.preload(["card.wood", "prop.well", { id: "bg.forest", variant: "night" }]);
    expect(decoder.requested).toEqual(["art/card/card.wood.webp", "art/background/bg.forest.night.png"]);
    expect(result.broken).toEqual([]);
  });

  it("decodes a repeated request once", async () => {
    vi.stubGlobal("Image", DecodingImage);
    const resolver = new ArtResolver(MANIFEST, ["card/card.wood.webp"]);
    const progress: PreloadProgress[] = [];
    await resolver.preload(["card.wood", { id: "card.wood" }, "card.wood"], (step) => progress.push(step));
    expect(decoder.requested).toEqual(["art/card/card.wood.webp"]);
    expect(progress).toHaveLength(1);
    expect(progress[0]?.total).toBe(1);
  });

  it("moves on to the next format when a file cannot be decoded, and reports the file", async () => {
    vi.stubGlobal("Image", DecodingImage);
    decoder.failing.add("art/card/card.wood.avif");
    const resolver = new ArtResolver(MANIFEST, ["card/card.wood.avif", "card/card.wood.webp"]);
    const progress: PreloadProgress[] = [];
    const result = await resolver.preload(["card.wood"], (step) => progress.push(step));
    expect(decoder.requested).toEqual(["art/card/card.wood.avif", "art/card/card.wood.webp"]);
    expect(result.broken).toEqual(["card/card.wood.avif"]);
    expect(progress[0]?.source).toBe("webp");
    expect(resolver.resolve("card.wood").source).toBe("webp");
  });

  it("ends at a placeholder when every file of a request fails", async () => {
    vi.stubGlobal("Image", DecodingImage);
    decoder.failing.add("art/card/card.wood.png");
    const resolver = new ArtResolver(MANIFEST, ["card/card.wood.png"]);
    const progress: PreloadProgress[] = [];
    const result = await resolver.preload(["card.wood"], (step) => progress.push(step));
    expect(result.broken).toEqual(["card/card.wood.png"]);
    expect(progress[0]?.source).toBe("placeholder");
    expect(resolver.missingReport().missing).toContainEqual({
      id: "card.wood",
      kind: "card",
      size: [180, 260],
      base: false,
      missing_variants: [],
    });
  });

  it("uses the load and error events when the browser has no decode()", async () => {
    vi.stubGlobal("Image", EventImage);
    decoder.failing.add("art/card/card.wood.avif");
    const resolver = new ArtResolver(MANIFEST, ["card/card.wood.avif", "card/card.wood.png"]);
    const result = await resolver.preload(["card.wood"]);
    expect(result.broken).toEqual(["card/card.wood.avif"]);
    expect(resolver.resolve("card.wood").source).toBe("png");
  });

  it("reports the broken files in sorted order", async () => {
    vi.stubGlobal("Image", DecodingImage);
    decoder.failing.add("art/prop/prop.well.png");
    decoder.failing.add("art/card/card.wood.png");
    const resolver = new ArtResolver(MANIFEST, ["prop/prop.well.png", "card/card.wood.png"]);
    const result = await resolver.preload(["prop.well", "card.wood"]);
    expect(result.broken).toEqual(["card/card.wood.png", "prop/prop.well.png"]);
  });
});
