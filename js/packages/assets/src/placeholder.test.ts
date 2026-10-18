// Tests for the generated placeholders: determinism, the size and label of the SVG, escaping, the
// kind silhouettes, the colour rule and the short label rule.

import { describe, expect, it } from "vitest";
import { ArtError } from "./error";
import { placeholderColor, placeholderSvg, shortLabel } from "./placeholder";
import { ART_KINDS, type ArtKind } from "./types";

/** A kind that the format does not have, typed so that the tests can pass it on purpose. */
function notAKind(text: string): ArtKind {
  const value: unknown = text;
  return value as ArtKind;
}

describe("placeholderSvg: the document", () => {
  it("is the same text for the same id, kind, size and label", () => {
    const first = placeholderSvg("character.lyra.portrait", "portrait", [480, 640]);
    const second = placeholderSvg("character.lyra.portrait", "portrait", [480, 640]);
    expect(second).toBe(first);
  });

  it("is a standalone SVG of the size it is given, with the id as its title", () => {
    const svg = placeholderSvg("card.wood", "card", [180, 260]);
    expect(svg.startsWith('<svg xmlns="http://www.w3.org/2000/svg" width="180" height="260" viewBox="0 0 180 260"')).toBe(true);
    expect(svg).toContain('role="img"');
    expect(svg).toContain("<title>card.wood</title>");
    expect(svg.endsWith("</svg>")).toBe(true);
    expect(svg).not.toMatch(/<script|href=|url\(http/);
  });

  it("uses the short label as its text and aria label by default", () => {
    const svg = placeholderSvg("character.lyra.portrait", "portrait", [480, 640]);
    expect(svg).toContain('aria-label="lyra"');
    expect(svg).toContain(">lyra</text>");
  });

  it("shows the label it is given instead of the short label", () => {
    const svg = placeholderSvg("card.wood", "card", [180, 260], "Oak");
    expect(svg).toContain('aria-label="Oak"');
    expect(svg).not.toContain('aria-label="wood"');
  });

  it("escapes markup in the label and the id", () => {
    const svg = placeholderSvg('card.<b>&"\'', "card", [180, 260], '<b>&"\'');
    expect(svg).not.toContain("<b>");
    expect(svg).toContain("&lt;b&gt;&amp;&quot;&apos;");
    expect(svg).toContain("<title>card.&lt;b&gt;&amp;&quot;&apos;</title>");
  });

  it("cuts a label longer than 24 characters to 21 characters and an ellipsis", () => {
    const svg = placeholderSvg("prop.well", "prop", [128, 160], "abcdefghijklmnopqrstuvwxyz");
    expect(svg).toContain('aria-label="abcdefghijklmnopqrstu..."');
  });

  it("gives every kind a silhouette of its own", () => {
    const figure = placeholderSvg("character.lyra.portrait", "portrait", [480, 640]);
    expect(figure).toContain("<circle");
    expect(figure).not.toContain("linearGradient");

    const card = placeholderSvg("card.wood", "card", [180, 260]);
    expect(card).not.toContain("<circle");
    expect(card).toContain('stroke-opacity="0.7"');

    const icon = placeholderSvg("icon.coin", "icon", [64, 64]);
    expect(icon).toContain("<circle");
    expect(icon).toContain(">C</text>");

    const scene = placeholderSvg("bg.forest", "background", [1920, 1080]);
    expect(scene).toContain("linearGradient");
    expect(scene).toMatch(/url\(#sky-[0-9a-f]+\)/);

    const prop = placeholderSvg("prop.well", "prop", [128, 160]);
    expect(prop).toContain("stroke-dasharray");
    expect(prop).not.toContain("linearGradient");
  });

  it("gives two background placeholders on one page their own gradient ids", () => {
    const first = placeholderSvg("bg.forest", "background", [1920, 1080]);
    const second = placeholderSvg("bg.lake", "background", [1920, 1080]);
    const idOf = (svg: string): string | undefined => /id="(sky-[0-9a-f]+)"/.exec(svg)?.[1];
    expect(idOf(first)).toBeDefined();
    expect(idOf(first)).not.toBe(idOf(second));
  });

  it("keeps every coordinate and length a whole number, even at odd sizes", () => {
    const svg = placeholderSvg("prop.odd", "prop", [131, 97]);
    // Opacities may be fractions; positions, sizes and the path data may not.
    expect(svg).not.toMatch(/\s(x|y|cx|cy|r|rx|width|height|font-size|stroke-width|d)="[^"]*\./);
    expect(svg).toContain('viewBox="0 0 131 97"');
  });

  it("draws a tiny box without negative lengths", () => {
    const svg = placeholderSvg("card.tiny", "card", [1, 1]);
    expect(svg).not.toMatch(/="-/);
  });

  it("refuses a size that is not whole sides from 1 to 7680", () => {
    for (const size of [
      [0, 10],
      [10, 0],
      [1.5, 10],
      [7681, 10],
    ] as const) {
      expect(() => placeholderSvg("card.wood", "card", size)).toThrow(ArtError);
    }
  });

  it("refuses a kind that the format does not have", () => {
    expect(() => placeholderSvg("card.wood", notAKind("sprite"), [180, 260])).toThrow(ArtError);
    expect(() => placeholderColor("card.wood", notAKind("sprite"))).toThrow(ArtError);
  });

  it("draws every kind", () => {
    for (const kind of ART_KINDS) {
      expect(placeholderSvg(`${kind}.sample`, kind, [64, 96])).toContain('width="64" height="96"');
    }
  });
});

describe("placeholderColor", () => {
  it("is a lowercase #rrggbb colour, the same for the same id and kind", () => {
    const color = placeholderColor("card.wood", "card");
    expect(color).toMatch(/^#[0-9a-f]{6}$/);
    expect(placeholderColor("card.wood", "card")).toBe(color);
  });

  it("takes its hue from the id, so different ids get different colours", () => {
    expect(placeholderColor("card.wood", "card")).not.toBe(placeholderColor("card.stone", "card"));
  });

  it("takes its lightness and saturation from the kind", () => {
    expect(placeholderColor("bg.forest", "background")).not.toBe(placeholderColor("bg.forest", "prop"));
  });

  it("matches the reference colours in the conformance fixtures", () => {
    expect(placeholderColor("character.lyra.portrait", "portrait")).toBe("#70c272");
    expect(placeholderColor("card.wood", "card")).toBe("#8b3b9b");
    expect(placeholderColor("bg.forest", "background")).toBe("#9b814b");
  });
});

describe("shortLabel", () => {
  it("drops the namespace and a last segment that repeats the kind", () => {
    expect(shortLabel("character.lyra.portrait", "portrait")).toBe("lyra");
    expect(shortLabel("character.lyra.fullbody", "fullbody")).toBe("lyra");
    expect(shortLabel("card.wood", "card")).toBe("wood");
    expect(shortLabel("bg.forest", "background")).toBe("forest");
  });

  it("joins the remaining segments with spaces, and turns underscores and dashes into spaces", () => {
    expect(shortLabel("prop.old_well", "prop")).toBe("old well");
    expect(shortLabel("prop.hay-bale", "prop")).toBe("hay bale");
    expect(shortLabel("icon.gold.coin", "icon")).toBe("gold coin");
    expect(shortLabel("prop.a__b--c", "prop")).toBe("a b c");
  });

  it("keeps a single segment, even when it repeats the kind", () => {
    expect(shortLabel("portrait", "portrait")).toBe("portrait");
    expect(shortLabel("character.portrait", "portrait")).toBe("portrait");
  });

  it("gives the whole id when the label would be empty", () => {
    expect(shortLabel("card.", "card")).toBe("card.");
  });

  it("cuts a long label to 21 characters and an ellipsis", () => {
    expect(shortLabel("prop.the_old_stone_well_at_the_edge_of_town", "prop")).toBe("the old stone well at...");
  });
});
