// @vitest-environment happy-dom
// Tests for artElement in happy-dom: the img or the inline SVG it returns, the logical size and the
// pivot, and the fallback when the browser refuses a file.

import { describe, expect, it } from "vitest";
import { artElement } from "./dom";
import { ArtError } from "./error";
import { parseArtManifest } from "./manifest";
import { ArtResolver } from "./resolver";

const SVG_NAMESPACE = "http://www.w3.org/2000/svg";

const MANIFEST = parseArtManifest(
  JSON.stringify({
    format: "velvet-art",
    version: 1,
    entries: [
      { id: "character.lyra.portrait", kind: "portrait", size: [480, 640], anchor: [0.5, 1], variants: ["happy"] },
      { id: "card.wood", kind: "card", size: [180, 260] },
    ],
  }),
);

/** The element as an image. Fails the test when it is not one. */
function asImage(element: Element): HTMLImageElement {
  if (element.tagName !== "IMG") throw new Error(`expected an img, got ${element.tagName}`);
  return element as HTMLImageElement;
}

describe("artElement for a file", () => {
  it("returns an img that points at the file, sized in logical pixels", () => {
    const resolver = new ArtResolver(MANIFEST, ["card/card.wood.webp"]);
    const image = asImage(artElement(resolver, "card.wood"));
    expect(image.getAttribute("src")).toBe("art/card/card.wood.webp");
    expect(image.style.width).toBe("180px");
    expect(image.style.height).toBe("260px");
    expect(image.alt).toBe("wood");
    expect(image.dataset["artId"]).toBe("card.wood");
    expect(image.dataset["artSource"]).toBe("webp");
  });

  it("is not draggable, and does not use nearest-neighbour scaling", () => {
    const resolver = new ArtResolver(MANIFEST, ["card/card.wood.png"]);
    const image = asImage(artElement(resolver, "card.wood"));
    expect(image.getAttribute("draggable")).toBe("false");
    expect(image.style.imageRendering).not.toBe("pixelated");
  });

  it("uses the anchor as the transform origin, so a scale grows from the pivot", () => {
    const resolver = new ArtResolver(MANIFEST, ["portrait/character.lyra.portrait.happy.png"]);
    const image = asImage(artElement(resolver, "character.lyra.portrait", "happy"));
    expect(image.style.transformOrigin).toBe("50% 100%");
    expect(image.dataset["artSource"]).toBe("png");
  });

  it("shows the base picture when the variant has no file", () => {
    const resolver = new ArtResolver(MANIFEST, ["portrait/character.lyra.portrait.webp"]);
    const image = asImage(artElement(resolver, "character.lyra.portrait", "happy"));
    expect(image.getAttribute("src")).toBe("art/portrait/character.lyra.portrait.webp");
  });
});

describe("artElement for a placeholder", () => {
  it("returns an inline SVG in the SVG namespace, sized in logical pixels", () => {
    const resolver = new ArtResolver(MANIFEST, []);
    const element = artElement(resolver, "card.wood");
    expect(element.namespaceURI).toBe(SVG_NAMESPACE);
    expect(element.localName).toBe("svg");
    expect(element.getAttribute("aria-label")).toBe("wood");
    expect(element.getAttribute("width")).toBe("180");
    expect(element.getAttribute("height")).toBe("260");
    expect(element.dataset["artSource"]).toBe("placeholder");
    expect(element.getAttribute("style")).toContain("width: 180px");
  });
});

describe("artElement when the browser refuses a file", () => {
  it("moves to the next format, then to the placeholder, and marks each refused file", () => {
    const resolver = new ArtResolver(MANIFEST, ["card/card.wood.avif", "card/card.wood.webp"]);
    const image = asImage(artElement(resolver, "card.wood"));
    expect(image.dataset["artSource"]).toBe("avif");

    image.dispatchEvent(new Event("error"));
    expect(image.getAttribute("src")).toBe("art/card/card.wood.webp");
    expect(image.dataset["artSource"]).toBe("webp");

    image.dispatchEvent(new Event("error"));
    expect(image.dataset["artSource"]).toBe("placeholder");
    expect(image.getAttribute("src")?.startsWith("data:image/svg+xml;charset=utf-8,")).toBe(true);

    image.dispatchEvent(new Event("error"));
    expect(image.dataset["artSource"]).toBe("placeholder");
  });

  it("does not show the refused file to the next element that asks for it", () => {
    const resolver = new ArtResolver(MANIFEST, ["card/card.wood.avif", "card/card.wood.webp"]);
    asImage(artElement(resolver, "card.wood")).dispatchEvent(new Event("error"));
    expect(asImage(artElement(resolver, "card.wood")).dataset["artSource"]).toBe("webp");
  });
});

describe("artElement errors", () => {
  it("throws for an id that the manifest does not have (strict)", () => {
    const resolver = new ArtResolver(MANIFEST, [], { strict: true });
    expect(() => artElement(resolver, "character.mara.portrait")).toThrow(ArtError);
  });
});
