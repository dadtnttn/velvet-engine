// Tests for the hash that picks placeholder colours, and for the colour helpers. The hash vectors are
// the published FNV-1a test values, plus two non-ASCII strings that Python computed from their UTF-8 bytes.

import { describe, expect, it } from "vitest";
import { hslToHex, inkFor, luma } from "./color";
import { fnv1a32 } from "./hash";

describe("fnv1a32", () => {
  it("matches the published FNV-1a 32-bit vectors", () => {
    expect(fnv1a32("")).toBe(0x811c9dc5);
    expect(fnv1a32("a")).toBe(0xe40c292c);
    expect(fnv1a32("foobar")).toBe(0xbf9cf968);
  });

  it("hashes the UTF-8 bytes, so accented and euro-sign text has its own values", () => {
    expect(fnv1a32("é")).toBe(0x1e9de8c1);
    expect(fnv1a32("€")).toBe(0x298f832b);
  });

  it("gives the same value for the same text, and an unsigned 32-bit integer", () => {
    expect(fnv1a32("card.wood")).toBe(0x454190a2);
    expect(fnv1a32("card.wood")).toBe(fnv1a32("card.wood"));
    expect(fnv1a32("card.wood")).toBeGreaterThanOrEqual(0);
  });
});

describe("hslToHex", () => {
  it("gives the primary and grey colours of CSS hsl()", () => {
    expect(hslToHex(0, 1, 0.5)).toBe("#ff0000");
    expect(hslToHex(120, 1, 0.25)).toBe("#008000");
    expect(hslToHex(240, 1, 0.5)).toBe("#0000ff");
    expect(hslToHex(0, 0, 0.5)).toBe("#808080");
    expect(hslToHex(0, 0, 1)).toBe("#ffffff");
    expect(hslToHex(0, 0, 0)).toBe("#000000");
  });

  it("wraps the hue into 0 to 360, in both directions", () => {
    expect(hslToHex(360, 1, 0.5)).toBe("#ff0000");
    expect(hslToHex(-120, 1, 0.5)).toBe("#0000ff");
    expect(hslToHex(480, 1, 0.5)).toBe(hslToHex(120, 1, 0.5));
  });

  it("writes lowercase hex with two digits per channel", () => {
    expect(hslToHex(290, 0.45, 0.42)).toMatch(/^#[0-9a-f]{6}$/);
  });

  it("matches the placeholder colours that the reference computes", () => {
    // Values from docs/formats/fixtures/art/cases.json (computed with the CSS Color 4 formula).
    expect(hslToHex(121, 0.4, 0.6)).toBe("#70c272");
    expect(hslToHex(290, 0.45, 0.42)).toBe("#8b3b9b");
  });
});

describe("luma and inkFor", () => {
  it("runs from 0 for black to 1 for white", () => {
    expect(luma("#000000")).toBe(0);
    expect(luma("#ffffff")).toBeCloseTo(1, 10);
  });

  it("picks near-black ink on light colours and white ink on dark ones", () => {
    expect(inkFor("#ffffff")).toBe("#1a1a1a");
    expect(inkFor("#000000")).toBe("#ffffff");
    expect(inkFor("#8b3b9b")).toBe("#ffffff");
    expect(inkFor("#c27083")).toBe("#1a1a1a");
  });
});
