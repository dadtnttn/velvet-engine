// Generated placeholders: the picture that an art id shows while it has no file. The SVG is made from
// the id alone, so the same id, kind, size and label always give the same text. The colour and the
// label are fixed by docs/formats/ART_MANIFEST.md; the drawing is not byte-exact across ports.

import { hslToHex, inkFor } from "./color";
import { ArtError } from "./error";
import { fnv1a32 } from "./hash";
import { ART_MAX_SIDE, isArtKind, type ArtKind, type ArtSize } from "./types";

/** The longest label shown. A longer one keeps its first 21 characters, then "...". */
const LABEL_LIMIT = 24;

/** The saturation and lightness of a colour, each from 0 to 1. */
interface Palette {
  readonly saturation: number;
  readonly lightness: number;
}

/** The colour of each kind's placeholder. The hue comes from the id, so only these two numbers depend on the kind. */
const PALETTE: Readonly<Record<ArtKind, Palette>> = {
  portrait: { saturation: 0.4, lightness: 0.6 },
  fullbody: { saturation: 0.4, lightness: 0.55 },
  card: { saturation: 0.45, lightness: 0.42 },
  icon: { saturation: 0.55, lightness: 0.48 },
  background: { saturation: 0.35, lightness: 0.45 },
  prop: { saturation: 0.4, lightness: 0.45 },
};

/** How much lighter the top of a background's sky is than its colour, and how much darker the bottom is. */
const SKY_TOP_STEP = 0.25;
const SKY_BOTTOM_STEP = -0.2;

/** What the drawing functions need: the box, the colours, the label and the line sizes. */
interface Frame {
  readonly width: number;
  readonly height: number;
  /** The shorter side of the box. Sizes of lines and text are taken from it. */
  readonly side: number;
  /** The 32-bit hash of the id. */
  readonly hash: number;
  readonly color: string;
  /** The colour of text and lines on `color`. */
  readonly ink: string;
  /** The label, not yet escaped. */
  readonly text: string;
  readonly stroke: number;
  readonly inset: number;
  readonly fontSize: number;
  readonly palette: Palette;
}

/** Rounds to the nearest whole number. Every coordinate in a placeholder is one. */
function whole(value: number): number {
  return Math.round(value);
}

/** Keeps a drawn length from going below zero. */
function nonNegative(value: number): number {
  return Math.max(0, whole(value));
}

/** Escapes text for XML, for element text and for attribute values. */
function escapeXml(text: string): string {
  return text.replace(/[&<>"']/g, (char) => {
    switch (char) {
      case "&":
        return "&amp;";
      case "<":
        return "&lt;";
      case ">":
        return "&gt;";
      case '"':
        return "&quot;";
      default:
        return "&apos;";
    }
  });
}

/** Cuts a label that is longer than LABEL_LIMIT: its first 21 characters, then "...". */
function clipLabel(text: string): string {
  return text.length > LABEL_LIMIT ? `${text.slice(0, LABEL_LIMIT - 3)}...` : text;
}

/** The palette of a kind. Throws for a string that is not a kind. */
function paletteOf(kind: string): Palette {
  if (!isArtKind(kind)) {
    throw new ArtError("value", `"${kind}" is not an art kind`);
  }
  return PALETTE[kind];
}

/** Checks that a size has whole sides from 1 to ART_MAX_SIDE. */
function checkSize(size: ArtSize): ArtSize {
  for (const side of size) {
    if (!Number.isInteger(side) || side < 1 || side > ART_MAX_SIDE) {
      throw new ArtError("value", `a placeholder needs whole sides from 1 to ${ART_MAX_SIDE}`);
    }
  }
  return size;
}

/** The hue of an id, in degrees: its hash modulo 360. */
function hueOf(hash: number): number {
  return hash % 360;
}

/**
 * The short label of an id, which a placeholder shows when no label is given:
 * 1. the first segment (the namespace) is dropped, when the id has more than one segment;
 * 2. the last segment is dropped too when it repeats the kind, and a segment is left;
 * 3. the remaining segments are joined with spaces, each run of "_" or "-" becomes one space, and the
 *    text is trimmed;
 * 4. a label longer than 24 characters is cut to 21 characters and "...".
 * For example `character.lyra.portrait` (kind portrait) gives "lyra", and `prop.old_well` gives "old well".
 * An id whose label would be empty gives the id itself.
 * @param id - the art id.
 * @param kind - the kind of the id.
 */
export function shortLabel(id: string, kind: ArtKind): string {
  const segments = id.split(".");
  if (segments.length > 1) segments.shift();
  if (segments.length > 1 && segments[segments.length - 1] === kind) segments.pop();
  const label = segments.join(" ").replace(/[_-]+/g, " ").trim();
  return clipLabel(label === "" ? id : label);
}

/**
 * The fill colour of an id's placeholder: the hue is the hash of the id, and the saturation and
 * lightness come from the kind.
 * @param id - the art id.
 * @param kind - the kind of the id.
 * @returns the colour as `#rrggbb`.
 */
export function placeholderColor(id: string, kind: ArtKind): string {
  const palette = paletteOf(kind);
  return hslToHex(hueOf(fnv1a32(id)), palette.saturation, palette.lightness);
}

/** The label text as an SVG text element, with its own position and extra attributes. */
function labelText(frame: Frame, x: number, y: number, fill: string, extra = ""): string {
  return `<text x="${x}" y="${y}" text-anchor="middle" font-family="sans-serif" font-size="${frame.fontSize}" fill="${fill}"${extra}>${escapeXml(frame.text)}</text>`;
}

/** A standing figure: a head, a torso and two legs, in the ink colour at low opacity. Portraits and full bodies share it. */
function drawFigure(frame: Frame): string {
  const { width: w, height: h } = frame;
  const shade = `fill="${frame.ink}" fill-opacity="0.3"`;
  return [
    `<rect width="${w}" height="${h}" fill="${frame.color}"/>`,
    `<circle cx="${whole(w / 2)}" cy="${whole(h * 0.165)}" r="${whole(h * 0.075)}" ${shade}/>`,
    `<rect x="${whole(w * 0.27)}" y="${whole(h * 0.27)}" width="${whole(w * 0.46)}" height="${whole(h * 0.36)}" rx="${whole(w * 0.12)}" ${shade}/>`,
    `<rect x="${whole(w * 0.27)}" y="${whole(h * 0.6)}" width="${whole(w * 0.17)}" height="${whole(h * 0.32)}" rx="${whole(w * 0.06)}" ${shade}/>`,
    `<rect x="${whole(w * 0.56)}" y="${whole(h * 0.6)}" width="${whole(w * 0.17)}" height="${whole(h * 0.32)}" rx="${whole(w * 0.06)}" ${shade}/>`,
    labelText(frame, whole(w / 2), whole(h * 0.975), frame.ink),
  ].join("");
}

/** A framed card: a filled face, a line frame, a band at the top and the label in the middle. */
function drawCard(frame: Frame): string {
  const { width: w, height: h, inset, stroke } = frame;
  const radius = whole(frame.side * 0.06);
  return [
    `<rect width="${w}" height="${h}" rx="${radius}" fill="${frame.color}"/>`,
    `<rect x="${inset}" y="${inset}" width="${nonNegative(w - 2 * inset)}" height="${nonNegative(h - 2 * inset)}" rx="${radius}" fill="none" stroke="${frame.ink}" stroke-opacity="0.7" stroke-width="${stroke}"/>`,
    `<rect x="${2 * inset}" y="${2 * inset}" width="${nonNegative(w - 4 * inset)}" height="${nonNegative(h * 0.14)}" fill="${frame.ink}" fill-opacity="0.2"/>`,
    labelText(frame, whole(w / 2), whole(h * 0.55 + frame.fontSize * 0.35), frame.ink),
  ].join("");
}

/** A circle icon: a filled disc with a ring, and the first letter of the label in the middle. */
function drawIcon(frame: Frame): string {
  const { width: w, height: h, stroke } = frame;
  const radius = nonNegative(frame.side / 2 - stroke);
  const initial = frame.text.charAt(0).toUpperCase() || "?";
  return [
    `<circle cx="${whole(w / 2)}" cy="${whole(h / 2)}" r="${radius}" fill="${frame.color}" stroke="${frame.ink}" stroke-opacity="0.7" stroke-width="${stroke}"/>`,
    `<text x="${whole(w / 2)}" y="${whole(h / 2)}" text-anchor="middle" dominant-baseline="central" font-family="sans-serif" font-size="${whole(frame.side * 0.5)}" fill="${frame.ink}">${escapeXml(initial)}</text>`,
  ].join("");
}

/**
 * A gradient landscape: a sky from a lighter tint to a darker one, two hills in the ink colour, and
 * the label in white with a dark outline, so that it reads on any sky.
 */
function drawBackground(frame: Frame): string {
  const { width: w, height: h, palette, hash } = frame;
  const hue = hueOf(hash);
  const top = hslToHex(hue, palette.saturation, palette.lightness + SKY_TOP_STEP);
  const bottom = hslToHex(hue, palette.saturation, palette.lightness + SKY_BOTTOM_STEP);
  // The gradient id comes from the full hash, so two placeholders on one page keep their own colours.
  const sky = `sky-${hash.toString(16)}`;
  const outline = ` stroke="#000000" stroke-opacity="0.55" stroke-width="${whole(frame.fontSize / 4)}" paint-order="stroke"`;
  return [
    `<defs><linearGradient id="${sky}" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="${top}"/><stop offset="1" stop-color="${bottom}"/></linearGradient></defs>`,
    `<rect width="${w}" height="${h}" fill="url(#${sky})"/>`,
    `<path d="M0 ${whole(h * 0.66)} Q ${whole(w * 0.25)} ${whole(h * 0.5)} ${whole(w * 0.5)} ${whole(h * 0.64)} T ${w} ${whole(h * 0.62)} L ${w} ${h} L 0 ${h} Z" fill="${frame.ink}" fill-opacity="0.18"/>`,
    `<path d="M0 ${whole(h * 0.8)} Q ${whole(w * 0.6)} ${whole(h * 0.7)} ${w} ${whole(h * 0.78)} L ${w} ${h} L 0 ${h} Z" fill="${frame.ink}" fill-opacity="0.3"/>`,
    labelText(frame, whole(w / 2), whole(h * 0.93), "#ffffff", outline),
  ].join("");
}

/** A prop: a rounded tile with a dashed outline, and the label in the middle. */
function drawProp(frame: Frame): string {
  const { width: w, height: h, inset, stroke } = frame;
  return [
    `<rect x="${inset}" y="${inset}" width="${nonNegative(w - 2 * inset)}" height="${nonNegative(h - 2 * inset)}" rx="${whole(frame.side * 0.12)}" fill="${frame.color}" stroke="${frame.ink}" stroke-opacity="0.7" stroke-width="${stroke}" stroke-dasharray="${stroke * 4} ${stroke * 3}"/>`,
    labelText(frame, whole(w / 2), whole(h / 2 + frame.fontSize * 0.35), frame.ink),
  ].join("");
}

/**
 * The placeholder picture of an id, as SVG text.
 *
 * The text depends only on its arguments: the same id, kind, size and label always give the same
 * string. The SVG has no scripts and no outside references, so it can be shown as an image or
 * inlined in a page.
 * @param id - the art id. Its hash gives the colour, and its short label gives the text.
 * @param kind - the kind, which picks the silhouette and the palette.
 * @param size - the box in logical pixels. The SVG is drawn at this size.
 * @param label - the text to show, instead of the short label. It is cut and escaped like a label.
 * @returns the SVG document as text.
 * @throws ArtError with code "value" when the kind or the size is not valid.
 */
export function placeholderSvg(id: string, kind: ArtKind, size: ArtSize, label?: string): string {
  const [width, height] = checkSize(size);
  const palette = paletteOf(kind);
  const hash = fnv1a32(id);
  const color = hslToHex(hueOf(hash), palette.saturation, palette.lightness);
  const side = Math.min(width, height);
  const frame: Frame = {
    width,
    height,
    side,
    hash,
    color,
    ink: inkFor(color),
    text: label === undefined ? shortLabel(id, kind) : clipLabel(label),
    stroke: Math.max(1, whole(side * 0.02)),
    inset: Math.max(1, whole(side * 0.05)),
    fontSize: Math.max(8, whole(side * 0.07)),
    palette,
  };
  let body: string;
  switch (kind) {
    case "portrait":
    case "fullbody":
      body = drawFigure(frame);
      break;
    case "card":
      body = drawCard(frame);
      break;
    case "icon":
      body = drawIcon(frame);
      break;
    case "background":
      body = drawBackground(frame);
      break;
    case "prop":
      body = drawProp(frame);
      break;
  }
  return `<svg xmlns="http://www.w3.org/2000/svg" width="${width}" height="${height}" viewBox="0 0 ${width} ${height}" role="img" aria-label="${escapeXml(frame.text)}"><title>${escapeXml(id)}</title>${body}</svg>`;
}

/** The start of the data URL of a placeholder: an SVG document, encoded as UTF-8. */
const PLACEHOLDER_URL_PREFIX = "data:image/svg+xml;charset=utf-8,";

/**
 * The data URL that shows a placeholder's SVG text. The resolver uses it for every placeholder, the
 * generic one and a runtime's own, and {@link placeholderTextOf} reads the text back.
 * @param svg - the SVG document as text.
 */
export function placeholderUrl(svg: string): string {
  return `${PLACEHOLDER_URL_PREFIX}${encodeURIComponent(svg)}`;
}

/**
 * The SVG text inside a placeholder's data URL, as {@link placeholderUrl} makes it.
 * @throws ArtError with code "value" when the URL is not a placeholder's data URL.
 */
export function placeholderTextOf(url: string): string {
  if (!url.startsWith(PLACEHOLDER_URL_PREFIX)) {
    throw new ArtError("value", "a placeholder is shown as an SVG data URL");
  }
  return decodeURIComponent(url.slice(PLACEHOLDER_URL_PREFIX.length));
}
