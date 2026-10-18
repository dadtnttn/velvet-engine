// Checks the names that the package root exports, so that a missing export fails a test.

import { describe, expect, it } from "vitest";
import * as assets from "./index";

describe("@velvet/assets exports", () => {
  it("exports the parser, the resolver, the placeholders, the DOM helper and the format constants", () => {
    expect(typeof assets.parseArtManifest).toBe("function");
    expect(typeof assets.ArtResolver).toBe("function");
    expect(typeof assets.placeholderSvg).toBe("function");
    expect(typeof assets.placeholderColor).toBe("function");
    expect(typeof assets.shortLabel).toBe("function");
    expect(typeof assets.artElement).toBe("function");
    expect(typeof assets.ArtError).toBe("function");
    expect(assets.ART_FORMAT).toBe("velvet-art");
    expect(assets.ART_VERSION).toBe(1);
    expect(assets.ART_REPORT_FORMAT).toBe("velvet-art-report");
    expect(assets.ART_KINDS).toEqual(["portrait", "fullbody", "card", "icon", "background", "prop"]);
    expect(assets.ART_FORMATS).toEqual(["avif", "webp", "png", "svg"]);
  });
});
