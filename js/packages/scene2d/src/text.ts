// Text: a block of text drawn with a canvas font. The block may wrap at a width, and it may have an
// outline and a shadow. The lines are laid out when the text or the font changes, and the layout is kept
// until one of those changes again. Measuring needs the context, so the scene measures for the block.

import { isPositiveFinite, type MutableRect } from "./math";
import { Node, type NodeOptions } from "./node";
import type { DrawPass, MeasuredText, TextMeasure } from "./pass";

/** Horizontal alignment of the lines inside the block. */
export type TextAlign = "left" | "center" | "right";

/** Where the glyphs of a line sit in its line box. */
export type TextBaseline = "top" | "middle" | "bottom" | "alphabetic";

/** An outline drawn around each glyph, outside the fill. */
export interface TextOutline {
  /** The colour of the outline, as a CSS colour. */
  readonly color: string;
  /** The width of the outline in logical pixels. */
  readonly width: number;
}

/** A shadow behind the glyphs. */
export interface TextShadow {
  /** The colour of the shadow, as a CSS colour. */
  readonly color: string;
  /** How much the shadow blurs, in logical pixels. Default 0. */
  readonly blur?: number;
  /** The offset of the shadow in the text's own units. Default 0. */
  readonly offsetX?: number;
  readonly offsetY?: number;
}

/** Options for {@link Text}. */
export interface TextOptions extends NodeOptions {
  /** The text. Line breaks (`\n`) start new lines. Default "". */
  text?: string;
  /** A CSS font family, such as `"Nunito, sans-serif"`. Default "sans-serif". */
  fontFamily?: string;
  /** The font size in logical pixels. Default 16. */
  fontSize?: number;
  /** The font weight, a number or a keyword. Default 400. */
  fontWeight?: number | string;
  /** Whether the font is italic. Default false. */
  italic?: boolean;
  /** The colour of the glyphs. Default "#ffffff". */
  fill?: string;
  /** The alignment of the lines. Default "left". */
  align?: TextAlign;
  /** The vertical place of the glyphs in a line. Default "top". */
  baseline?: TextBaseline;
  /** The width at which lines wrap, in logical pixels. Null, the default, does not wrap. */
  maxWidth?: number | null;
  /** The distance between line tops, in logical pixels. Null, the default, is 1.2 times the font size. */
  lineHeight?: number | null;
  /** An outline. Default none. */
  outline?: TextOutline | null;
  /** A shadow. Default none. */
  shadow?: TextShadow | null;
}

/** The result of laying out a text: its lines, the box that they fill, and the metrics of the font. */
interface TextLayout {
  readonly lines: readonly string[];
  readonly font: string;
  readonly width: number;
  readonly height: number;
  readonly lineHeight: number;
  readonly ascent: number;
  readonly descent: number;
}

/** The line height as a multiple of the font size, when no line height is given. */
const DEFAULT_LINE_FACTOR = 1.2;
/** The ascent and descent as multiples of the font size, when the context does not report the font's own. */
const FALLBACK_ASCENT = 0.8;
const FALLBACK_DESCENT = 0.2;

/** A block of text. Set its fields, and the scene lays it out and draws it. */
export class Text extends Node {
  /** The text. Line breaks start new lines. */
  text: string;
  /** The CSS font family. */
  fontFamily: string;
  /** The font size in logical pixels. */
  fontSize: number;
  /** The font weight, a number or a keyword. */
  fontWeight: number | string;
  /** Whether the font is italic. */
  italic: boolean;
  /** The colour of the glyphs. */
  fill: string;
  /** The alignment of the lines. */
  align: TextAlign;
  /** The vertical place of the glyphs in a line. */
  baseline: TextBaseline;
  /** The width at which lines wrap, or null for no wrap. */
  maxWidth: number | null;
  /** The distance between line tops, or null for 1.2 times the font size. */
  lineHeight: number | null;
  /** The outline, or null. */
  outline: TextOutline | null;
  /** The shadow, or null. */
  shadow: TextShadow | null;
  private layout: TextLayout | null = null;
  // The inputs of the current layout. A change to any of them lays the text out again.
  private laidText = "";
  private laidFamily = "";
  private laidSize = Number.NaN;
  private laidWeight: number | string = "";
  private laidItalic = false;
  private laidMaxWidth: number | null = null;
  private laidLineHeight: number | null = null;

  /**
   * @param options - see {@link TextOptions}.
   * @throws RangeError when the font size, the wrap width or the line height is not a positive number.
   */
  constructor(options: TextOptions = {}) {
    super(options);
    this.text = options.text ?? "";
    this.fontFamily = options.fontFamily ?? "sans-serif";
    this.fontSize = options.fontSize ?? 16;
    if (!isPositiveFinite(this.fontSize)) {
      throw new RangeError(`fontSize must be a positive number, got ${this.fontSize}`);
    }
    this.fontWeight = options.fontWeight ?? 400;
    this.italic = options.italic ?? false;
    this.fill = options.fill ?? "#ffffff";
    this.align = options.align ?? "left";
    this.baseline = options.baseline ?? "top";
    this.maxWidth = options.maxWidth ?? null;
    if (this.maxWidth !== null && !isPositiveFinite(this.maxWidth)) {
      throw new RangeError(`maxWidth must be a positive number or null, got ${this.maxWidth}`);
    }
    this.lineHeight = options.lineHeight ?? null;
    if (this.lineHeight !== null && !isPositiveFinite(this.lineHeight)) {
      throw new RangeError(`lineHeight must be a positive number or null, got ${this.lineHeight}`);
    }
    this.outline = options.outline ?? null;
    this.shadow = options.shadow ?? null;
  }

  /** The lines of the current layout, or none before the first update. */
  get lines(): readonly string[] {
    return this.layout?.lines ?? [];
  }

  protected override prepare(measure: TextMeasure): void {
    if (this.layout !== null && this.inputsMatchLayout()) return;
    const font = this.fontString();
    this.layout = layoutText(
      this.text,
      font,
      this.fontSize,
      this.maxWidth,
      this.lineHeight ?? this.fontSize * DEFAULT_LINE_FACTOR,
      measure,
    );
    this.laidText = this.text;
    this.laidFamily = this.fontFamily;
    this.laidSize = this.fontSize;
    this.laidWeight = this.fontWeight;
    this.laidItalic = this.italic;
    this.laidMaxWidth = this.maxWidth;
    this.laidLineHeight = this.lineHeight;
  }

  protected override localBox(out: MutableRect): void {
    out.x = 0;
    out.y = 0;
    out.width = this.layout?.width ?? 0;
    out.height = this.layout?.height ?? 0;
  }

  protected override contentRect(out: MutableRect): boolean {
    const layout = this.layout;
    if (layout === null) return false;
    const pad = this.padding();
    out.x = -pad;
    out.y = -pad;
    out.width = layout.width + pad * 2;
    out.height = layout.height + pad * 2;
    return true;
  }

  protected override hitGeometry(x: number, y: number): boolean {
    const layout = this.layout;
    if (layout === null) return false;
    return x >= 0 && x <= layout.width && y >= 0 && y <= layout.height;
  }

  protected override drawSelf(pass: DrawPass, alpha: number, _pixelArt: boolean): void {
    const layout = this.layout;
    if (layout === null) return;
    const ctx = pass.context;
    pass.place(this.surfaceMatrix);
    pass.alpha(alpha);
    ctx.font = layout.font;
    ctx.textAlign = this.align;
    ctx.textBaseline = this.baseline;
    ctx.lineJoin = "round";
    if (this.shadow !== null) this.applyShadow(pass, this.shadow);

    const anchorX = this.align === "center" ? layout.width / 2 : this.align === "right" ? layout.width : 0;
    const baselineOffset = this.baselineOffset(layout);
    const outline = this.outline;
    for (let index = 0; index < layout.lines.length; index += 1) {
      const line = layout.lines[index] ?? "";
      if (line === "") continue;
      const y = index * layout.lineHeight + baselineOffset;
      if (outline !== null && outline.width > 0) {
        ctx.strokeStyle = outline.color;
        ctx.lineWidth = outline.width;
        ctx.strokeText(line, anchorX, y);
        pass.count();
      }
      ctx.fillStyle = this.fill;
      ctx.fillText(line, anchorX, y);
      pass.count();
    }

    if (this.shadow !== null) {
      // The shadow is state of the context, so it is switched off again for the next node.
      ctx.shadowColor = "rgba(0, 0, 0, 0)";
      ctx.shadowBlur = 0;
      ctx.shadowOffsetX = 0;
      ctx.shadowOffsetY = 0;
    }
  }

  /** The font as a CSS font string. */
  private fontString(): string {
    const style = this.italic ? "italic " : "";
    return `${style}${this.fontWeight} ${this.fontSize}px ${this.fontFamily}`;
  }

  /** True when the current layout was made from the present text and font. */
  private inputsMatchLayout(): boolean {
    return (
      this.laidText === this.text &&
      this.laidFamily === this.fontFamily &&
      this.laidSize === this.fontSize &&
      this.laidWeight === this.fontWeight &&
      this.laidItalic === this.italic &&
      this.laidMaxWidth === this.maxWidth &&
      this.laidLineHeight === this.lineHeight
    );
  }

  /** How far the outline and the shadow reach past the box, in logical pixels. */
  private padding(): number {
    let pad = this.outline !== null ? this.outline.width / 2 : 0;
    if (this.shadow !== null) {
      const blur = Math.max(0, this.shadow.blur ?? 0);
      const offset = Math.max(Math.abs(this.shadow.offsetX ?? 0), Math.abs(this.shadow.offsetY ?? 0));
      pad += blur + offset;
    }
    return pad;
  }

  /** Where the glyphs sit in a line box, measured from the top of the box. */
  private baselineOffset(layout: TextLayout): number {
    if (this.baseline === "middle") return layout.lineHeight / 2;
    if (this.baseline === "bottom") return layout.lineHeight;
    if (this.baseline === "alphabetic") {
      return (layout.lineHeight - (layout.ascent + layout.descent)) / 2 + layout.ascent;
    }
    return 0;
  }

  /**
   * Sets the shadow on the context. The canvas does not apply the transform to the shadow, so the offset
   * and the blur are converted here to device pixels, with the node's own transform applied to the offset.
   */
  private applyShadow(pass: DrawPass, shadow: TextShadow): void {
    const ctx = pass.context;
    const m = this.surfaceMatrix;
    const k = pass.scale;
    const offsetX = shadow.offsetX ?? 0;
    const offsetY = shadow.offsetY ?? 0;
    ctx.shadowColor = shadow.color;
    ctx.shadowBlur = Math.max(0, shadow.blur ?? 0) * k * m.meanScale();
    ctx.shadowOffsetX = k * (m.a * offsetX + m.c * offsetY);
    ctx.shadowOffsetY = k * (m.b * offsetX + m.d * offsetY);
  }
}

/**
 * Lays a text out: splits it into lines at its line breaks, wraps each paragraph at the width, and measures
 * the box that the lines fill. Wrapping works on words; a word wider than the width is broken between its
 * characters.
 */
function layoutText(
  text: string,
  font: string,
  fontSize: number,
  maxWidth: number | null,
  lineHeight: number,
  measure: TextMeasure,
): TextLayout {
  const widthOf = (line: string): number => measure(line, font).width;
  const sample: MeasuredText = measure("Mg", font);
  const lines: string[] = [];
  for (const paragraph of text.split(/\r?\n/)) {
    if (maxWidth === null) lines.push(paragraph);
    else wrapParagraph(paragraph, maxWidth, widthOf, lines);
  }
  // A wrapped block keeps the wrap width, so that the alignment has a box to use. An unwrapped block is as wide as its longest line.
  let width = maxWidth ?? 0;
  if (maxWidth === null) {
    for (const line of lines) width = Math.max(width, widthOf(line));
  }
  return {
    lines,
    font,
    width,
    height: lines.length * lineHeight,
    lineHeight,
    ascent: sample.ascent ?? fontSize * FALLBACK_ASCENT,
    descent: sample.descent ?? fontSize * FALLBACK_DESCENT,
  };
}

/** Wraps one paragraph into lines no wider than `maxWidth`, adding them to `out`. */
function wrapParagraph(
  paragraph: string,
  maxWidth: number,
  widthOf: (line: string) => number,
  out: string[],
): void {
  const words = paragraph.split(/\s+/).filter((word) => word.length > 0);
  if (words.length === 0) {
    out.push("");
    return;
  }
  let current = "";
  for (const word of words) {
    const joined = current === "" ? word : `${current} ${word}`;
    if (widthOf(joined) <= maxWidth) {
      current = joined;
      continue;
    }
    if (current !== "") {
      out.push(current);
      current = "";
    }
    if (widthOf(word) <= maxWidth) {
      current = word;
      continue;
    }
    // A word wider than the box is broken between its characters.
    for (const character of word) {
      const next = current + character;
      if (current !== "" && widthOf(next) > maxWidth) {
        out.push(current);
        current = character;
      } else {
        current = next;
      }
    }
  }
  if (current !== "") out.push(current);
}
