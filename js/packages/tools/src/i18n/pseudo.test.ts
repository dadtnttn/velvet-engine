// Pseudo-texts: accented, about 35% longer, bracketed, with every hole and brace escape kept.

import { describe, expect, it } from "vitest";
import { parseCatalog, parseTemplate } from "@velvet/i18n";
import { pseudoText, pseudoValue, PSEUDO_GROWTH } from "./pseudo";

/** The holes of a template, as a sorted list. */
function holesOf(text: string): string[] {
  const parsed = parseTemplate(text);
  return parsed.ok ? [...parsed.template.holes].sort() : ["invalid"];
}

describe("pseudoText", () => {
  it("accents each letter, keeps the holes, and brackets the text", () => {
    expect(pseudoText("Hello")).toBe("[Ĥéĺĺó~~]");
    // 21 characters of source: the filler is 35% of that, rounded up, which is 8.
    expect(pseudoText("Buy {item} for {gold}")).toBe("[Ɓúý {item} ƒóŕ {gold}~~~~~~~~]");
  });

  it("grows the text by about 35%, inside the brackets", () => {
    const sources = [
      "Save the game now",
      "Day {day} of the village, at dusk",
      "Recipes ready: choose one of the three crafts below and press confirm",
    ];
    for (const source of sources) {
      const pseudo = pseudoText(source) ?? "";
      const inside = [...pseudo.slice(1, -1)].length;
      const ratio = inside / source.length;
      expect(ratio).toBeGreaterThanOrEqual(1 + PSEUDO_GROWTH - 0.05);
      expect(ratio).toBeLessThanOrEqual(1 + PSEUDO_GROWTH + 0.05);
    }
  });

  it("keeps every hole, and the brace escapes, so that the template still parses", () => {
    const source = "Literal {{braces}} and {count} items, {0} left";
    const pseudo = pseudoText(source) ?? "";
    expect(pseudo.startsWith("[")).toBe(true);
    expect(holesOf(pseudo)).toEqual(holesOf(source));
    expect(pseudo).toContain("{{");
    expect(pseudo).toContain("}}");
  });

  it("leaves a text with no letter of its own as it is", () => {
    expect(pseudoText("{0} / {1}")).toBe("{0} / {1}");
    expect(pseudoText("42")).toBe("42");
    expect(pseudoText("")).toBe("");
  });

  it("refuses a text that is not a valid template", () => {
    expect(pseudoText("{open")).toBeUndefined();
    expect(pseudoText("stray } brace")).toBeUndefined();
  });

  it("writes each form of a plural entry, and keeps the other form", () => {
    expect(pseudoValue({ one: "{count} coin", other: "{count} coins" })).toEqual({
      one: pseudoText("{count} coin"),
      other: pseudoText("{count} coins"),
    });
    expect(pseudoValue("Hello")).toBe(pseudoText("Hello"));
    expect(pseudoValue({ one: "{count} x", other: "{count" })).toBeUndefined();
  });

  it("produces a catalog that the i18n package reads without an error", () => {
    const entries = { "menu.start": pseudoText("Start") ?? "", "shop.coins": pseudoValue({ one: "{count} coin", other: "{count} coins" }) };
    const text = JSON.stringify({ format: "velvet-locale", version: 1, language: "en-XA", entries });
    const parsed = parseCatalog(text, "00_pseudo.json");
    expect(parsed.usable).toBe(true);
    expect(parsed.issues.filter((issue) => issue.severity === "error")).toEqual([]);
    expect([...parsed.entries.keys()]).toEqual(["menu.start", "shop.coins"]);
  });
});
