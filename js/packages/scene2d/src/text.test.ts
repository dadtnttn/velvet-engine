// Tests for text: the lines of a wrapped paragraph (with the fake measure of half a font size per character), a
// word too wide for the box, the alignment and the baselines of the lines, the outline and the shadow, and the
// layout that is kept until the text or the font changes. The fake context gives every character half the font
// size, so a 20 pixel font gives 10 pixels per character, and a line box is 24 pixels (1.2 times the size).

import { describe, expect, it, vi } from "vitest";
import { createScene2D } from "./scene";
import { Text, type TextBaseline } from "./text";
import { FakeCanvas } from "./test-support";

/** A scene with a fixed layer that holds one text. Returns the scene and the canvas it draws into. */
function stageWith(text: Text) {
  const canvas = new FakeCanvas(1920, 1080);
  const scene = createScene2D({ canvas });
  scene.createLayer({ parallax: 0 }).add(text);
  return { canvas, scene };
}

describe("word wrap", () => {
  it("wraps words so that each line fits the width", () => {
    // A 5 character width: "one" and "two" each fit alone but not together.
    const text = new Text({ text: "one two three four", fontSize: 20, maxWidth: 50 });
    const { scene } = stageWith(text);
    scene.update();
    expect(text.lines).toEqual(["one", "two", "three", "four"]);
  });

  it("packs as many words on a line as fit", () => {
    // "ab cd" is 5 characters, so it fits in 50, and "ab cd ef" (8 characters) does not.
    const text = new Text({ text: "ab cd ef gh", fontSize: 20, maxWidth: 50 });
    const { scene } = stageWith(text);
    scene.update();
    expect(text.lines).toEqual(["ab cd", "ef gh"]);
  });

  it("breaks a line at each line break, even without a wrap width", () => {
    const text = new Text({ text: "one two\nthree", fontSize: 20 });
    const { scene } = stageWith(text);
    scene.update();
    expect(text.lines).toEqual(["one two", "three"]);
  });

  it("breaks a word that is wider than the box between its characters", () => {
    const text = new Text({ text: "abcdefgh", fontSize: 20, maxWidth: 30 });
    const { scene } = stageWith(text);
    scene.update();
    expect(text.lines).toEqual(["abc", "def", "gh"]);
  });

  it("keeps an empty line for a blank paragraph", () => {
    const text = new Text({ text: "a\n\nb", fontSize: 20 });
    const { scene } = stageWith(text);
    scene.update();
    expect(text.lines).toEqual(["a", "", "b"]);
  });
});

describe("alignment and the box", () => {
  it("centres the lines inside the wrap width", () => {
    const text = new Text({ text: "ab", fontSize: 20, maxWidth: 100, align: "center" });
    const { canvas, scene } = stageWith(text);
    scene.render();
    expect(canvas.context.nodeDraws[0]).toMatchObject({ op: "fillText", text: "ab", args: [50, 0] });
  });

  it("aligns to the right edge of the wrap width", () => {
    const text = new Text({ text: "ab", fontSize: 20, maxWidth: 100, align: "right" });
    const { canvas, scene } = stageWith(text);
    scene.render();
    expect(canvas.context.nodeDraws[0]?.args).toEqual([100, 0]);
  });

  it("uses the longest line as the width when it does not wrap", () => {
    const text = new Text({ text: "ab\nabcd", fontSize: 20, align: "right" });
    const { canvas, scene } = stageWith(text);
    scene.render();
    // "abcd" is 40 wide, so the right edge is at 40 for both lines.
    expect(canvas.context.nodeDraws.map((draw) => draw.args[0])).toEqual([40, 40]);
  });

  it("measures the anchor from the height of its lines", () => {
    // Three lines of 24 pixels make a box 72 tall. An anchor in the middle is 36 down, so the content origin
    // sits 36 above the pivot, which is at (0, 0).
    const text = new Text({ text: "a\nb\nc", fontSize: 20, anchorY: 0.5 });
    const { scene } = stageWith(text);
    scene.update();
    const origin = text.layerMatrix.apply(0, 0);
    expect(origin.x).toBeCloseTo(0, 9);
    expect(origin.y).toBeCloseTo(-36, 9);
  });

  it("puts the line tops at the given line height apart", () => {
    const text = new Text({ text: "a\nb", fontSize: 20, lineHeight: 30 });
    const { canvas, scene } = stageWith(text);
    scene.render();
    expect(canvas.context.nodeDraws.map((draw) => draw.args[1])).toEqual([0, 30]);
  });
});

describe("baselines", () => {
  /** The y coordinate of each text draw of the two lines "a" and "b", in order. */
  function glyphHeights(baseline: TextBaseline, reportsMetrics = false): number[] {
    const text = new Text({ text: "a\nb", fontSize: 20, baseline });
    const { canvas, scene } = stageWith(text);
    canvas.context.reportsMetrics = reportsMetrics;
    scene.render();
    return canvas.context.nodeDraws.map((draw) => draw.args[1] ?? Number.NaN);
  }

  it("puts the glyphs at the top of each line box", () => {
    expect(glyphHeights("top")).toEqual([0, 24]);
  });

  it("puts the glyphs at the middle of each line box", () => {
    expect(glyphHeights("middle")).toEqual([12, 36]);
  });

  it("puts the glyphs at the bottom of each line box", () => {
    expect(glyphHeights("bottom")).toEqual([24, 48]);
  });

  it("puts the alphabetic baseline by the font's ascent, centred in the line box", () => {
    // The fake reports an ascent of 16 and a descent of 4, so the 4 spare pixels give 2 above the ascent.
    expect(glyphHeights("alphabetic", true)).toEqual([18, 42]);
    // Without metrics the same numbers are estimated from the font size.
    expect(glyphHeights("alphabetic")).toEqual([18, 42]);
  });
});

describe("outline and shadow", () => {
  it("strokes the outline before it fills the glyphs", () => {
    const text = new Text({ text: "ab", fontSize: 20, fill: "#ff0000", outline: { color: "#000000", width: 3 } });
    const { canvas, scene } = stageWith(text);
    scene.render();
    expect(canvas.context.nodeDraws.map((draw) => draw.op)).toEqual(["strokeText", "fillText"]);
    expect(canvas.context.nodeDraws[0]).toMatchObject({ strokeStyle: "#000000", lineWidth: 3 });
    expect(canvas.context.nodeDraws[1]).toMatchObject({ fillStyle: "#ff0000" });
  });

  it("sets the shadow in device pixels, with the offset carried by the node's transform", () => {
    const text = new Text({ text: "ab", fontSize: 20, shadow: { color: "#222222", blur: 4, offsetX: 2, offsetY: 3 } });
    const { canvas, scene } = stageWith(text);
    scene.render();
    expect(canvas.context.nodeDraws[0]).toMatchObject({ shadowBlur: 4, shadowOffsetX: 2, shadowOffsetY: 3 });
  });

  it("switches the shadow off again after the text, so that the next draw has none", () => {
    const text = new Text({ text: "ab", fontSize: 20, shadow: { color: "#222222", blur: 4 } });
    const { canvas, scene } = stageWith(text);
    scene.render();
    expect(canvas.context.shadowBlur).toBe(0);
    expect(canvas.context.shadowOffsetX).toBe(0);
    expect(canvas.context.shadowColor).toBe("rgba(0, 0, 0, 0)");
  });
});

describe("the font and the layout cache", () => {
  it("builds the CSS font from the style, the weight, the size and the family", () => {
    const text = new Text({ text: "ab", fontSize: 20, fontWeight: 700, italic: true, fontFamily: "Nunito" });
    const { canvas, scene } = stageWith(text);
    scene.render();
    expect(canvas.context.nodeDraws[0]?.font).toBe("italic 700 20px Nunito");
  });

  it("lays the text out once, and again only when the text changes", () => {
    const text = new Text({ text: "ab", fontSize: 20 });
    const { canvas, scene } = stageWith(text);
    const measure = vi.spyOn(canvas.context, "measureText");
    scene.render();
    const firstLayout = measure.mock.calls.length;
    expect(firstLayout).toBeGreaterThan(0);
    scene.render();
    expect(measure.mock.calls.length).toBe(firstLayout);
    text.text = "abc";
    scene.render();
    expect(measure.mock.calls.length).toBeGreaterThan(firstLayout);
    expect(text.lines).toEqual(["abc"]);
  });

  it("draws nothing for an empty text", () => {
    const text = new Text({ text: "", fontSize: 20 });
    const { canvas, scene } = stageWith(text);
    scene.render();
    expect(canvas.context.nodeDraws).toEqual([]);
    expect(text.lines).toEqual([""]);
  });

  it("refuses a font size or a wrap width that is not positive", () => {
    expect(() => new Text({ fontSize: 0 })).toThrow(RangeError);
    expect(() => new Text({ maxWidth: -5 })).toThrow(RangeError);
  });
});
