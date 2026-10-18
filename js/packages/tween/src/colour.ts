// Colours for tweens: parsing of #rgb, #rgba, #rrggbb, #rrggbbaa, rgb() and rgba(), and mixing in
// OKLab, a perceptual space. A colour tween that mixes in OKLab passes through the colours a person
// expects between the two ends, where a plain sRGB mix would go dull and grey. Channels here are
// sRGB values from 0 to 255, and alpha is from 0 to 1.
//
// OKLab is Bjorn Ottosson's space (2020). The matrices below are his published ones.

/** A colour in sRGB, with an opacity. */
export interface Colour {
  /** Red, from 0 to 255. */
  readonly red: number;
  /** Green, from 0 to 255. */
  readonly green: number;
  /** Blue, from 0 to 255. */
  readonly blue: number;
  /** Opacity, from 0 (clear) to 1 (solid). */
  readonly alpha: number;
  /** True when the text gave an alpha channel, so the colour is written back with one. */
  readonly hasAlpha: boolean;
}

/** A colour in OKLab: lightness, and the two opponent axes. */
interface Lab {
  readonly l: number;
  readonly a: number;
  readonly b: number;
}

const HEX_PATTERN = /^#([0-9a-f]{3,4}|[0-9a-f]{6}|[0-9a-f]{8})$/i;
const FUNCTION_PATTERN = /^rgba?\(([^)]*)\)$/i;
const NUMBER_PATTERN = /^[+-]?(\d+\.?\d*|\.\d+)%?$/;

/**
 * Reads a colour from text. Returns null when the text is not one of the supported forms:
 * `#rgb`, `#rgba`, `#rrggbb`, `#rrggbbaa`, `rgb(r, g, b)` and `rgba(r, g, b, a)`. The functional
 * forms take commas or spaces, a slash before the alpha, and percentages for each channel.
 * Named colours such as "red" are not supported.
 */
export function parseColour(text: string): Colour | null {
  const value = text.trim();
  const hex = HEX_PATTERN.exec(value);
  if (hex !== null) return parseHex(hex[1] ?? "");
  const functional = FUNCTION_PATTERN.exec(value);
  if (functional !== null) return parseFunctional(functional[1] ?? "");
  return null;
}

/** Mixes two colours in OKLab at progress `t`. Values outside [0, 1] extrapolate and are clamped to the displayable range. */
export function mixColours(from: Colour, to: Colour, t: number): Colour {
  const start = toLab(from);
  const end = toLab(to);
  const [red, green, blue] = fromLab(
    lerp(start.l, end.l, t),
    lerp(start.a, end.a, t),
    lerp(start.b, end.b, t),
  );
  return {
    red,
    green,
    blue,
    alpha: clampTo(lerp(from.alpha, to.alpha, t), 0, 1),
    hasAlpha: from.hasAlpha || to.hasAlpha,
  };
}

/**
 * Writes a colour as CSS text: `rgb(r, g, b)`, or `rgba(r, g, b, a)` when `withAlpha` is true.
 * Channels are rounded to whole numbers.
 */
export function formatColour(colour: Colour, withAlpha: boolean): string {
  const red = Math.round(clampTo(colour.red, 0, 255));
  const green = Math.round(clampTo(colour.green, 0, 255));
  const blue = Math.round(clampTo(colour.blue, 0, 255));
  if (!withAlpha) return `rgb(${red}, ${green}, ${blue})`;
  const alpha = Math.round(clampTo(colour.alpha, 0, 1) * 1000) / 1000;
  return `rgba(${red}, ${green}, ${blue}, ${alpha})`;
}

/** Reads the digits of a hex colour; the short forms repeat each digit, so #f80 is #ff8800. */
function parseHex(digits: string): Colour {
  const full = digits.length <= 4 ? [...digits].map((digit) => digit + digit).join("") : digits;
  const hasAlpha = full.length === 8;
  return {
    red: parseInt(full.slice(0, 2), 16),
    green: parseInt(full.slice(2, 4), 16),
    blue: parseInt(full.slice(4, 6), 16),
    alpha: hasAlpha ? parseInt(full.slice(6, 8), 16) / 255 : 1,
    hasAlpha,
  };
}

/** Reads the inside of an rgb() or rgba() call. Null when a channel or the alpha is not a number. */
function parseFunctional(inner: string): Colour | null {
  const parts = inner.split(/[\s,/]+/).filter((part) => part !== "");
  if (parts.length !== 3 && parts.length !== 4) return null;
  const channels: number[] = [];
  for (const part of parts.slice(0, 3)) {
    const channel = channelValue(part);
    if (channel === null) return null;
    channels.push(channel);
  }
  let alpha = 1;
  const alphaPart = parts[3];
  if (alphaPart !== undefined) {
    const value = alphaValue(alphaPart);
    if (value === null) return null;
    alpha = value;
  }
  return {
    red: channels[0] ?? 0,
    green: channels[1] ?? 0,
    blue: channels[2] ?? 0,
    alpha,
    hasAlpha: parts.length === 4,
  };
}

/** Reads a number token and whether it was a percentage. Null when it is not a number. */
function readNumber(token: string): { value: number; percent: boolean } | null {
  if (!NUMBER_PATTERN.test(token)) return null;
  const percent = token.endsWith("%");
  const value = Number(percent ? token.slice(0, -1) : token);
  return Number.isFinite(value) ? { value, percent } : null;
}

/** A colour channel: a number from 0 to 255, or a percentage of 255. */
function channelValue(token: string): number | null {
  const parsed = readNumber(token);
  if (parsed === null) return null;
  return clampTo(parsed.percent ? (parsed.value * 255) / 100 : parsed.value, 0, 255);
}

/** An alpha value: a number from 0 to 1, or a percentage. */
function alphaValue(token: string): number | null {
  const parsed = readNumber(token);
  if (parsed === null) return null;
  return clampTo(parsed.percent ? parsed.value / 100 : parsed.value, 0, 1);
}

/** Keeps `value` inside [low, high]. */
function clampTo(value: number, low: number, high: number): number {
  return Math.min(high, Math.max(low, value));
}

/** Linear mix, exact at both ends. */
function lerp(from: number, to: number, t: number): number {
  if (t === 0) return from;
  if (t === 1) return to;
  return from + (to - from) * t;
}

/** One sRGB channel from 0 to 255, to linear light from 0 to 1. */
function toLinear(channel: number): number {
  const value = channel / 255;
  return value <= 0.04045 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4;
}

/** Linear light from 0 to 1, back to an sRGB channel from 0 to 255, clamped to the displayable range. */
function fromLinear(linear: number): number {
  const value = clampTo(linear, 0, 1);
  const encoded = value <= 0.0031308 ? 12.92 * value : 1.055 * value ** (1 / 2.4) - 0.055;
  return clampTo(encoded, 0, 1) * 255;
}

/** Converts a colour to OKLab. */
function toLab(colour: Colour): Lab {
  const red = toLinear(colour.red);
  const green = toLinear(colour.green);
  const blue = toLinear(colour.blue);
  const l = Math.cbrt(0.4122214708 * red + 0.5363325363 * green + 0.0514459929 * blue);
  const m = Math.cbrt(0.2119034982 * red + 0.6806995451 * green + 0.1073969566 * blue);
  const s = Math.cbrt(0.0883024619 * red + 0.2817188376 * green + 0.6299787005 * blue);
  return {
    l: 0.2104542553 * l + 0.793617785 * m - 0.0040720468 * s,
    a: 1.9779984951 * l - 2.428592205 * m + 0.4505937099 * s,
    b: 0.0259040371 * l + 0.7827717662 * m - 0.808675766 * s,
  };
}

/** Converts OKLab back to sRGB channels from 0 to 255. */
function fromLab(l: number, a: number, b: number): [number, number, number] {
  const lRoot = l + 0.3963377774 * a + 0.2158037573 * b;
  const mRoot = l - 0.1055613458 * a - 0.0638541728 * b;
  const sRoot = l - 0.0894841775 * a - 1.291485548 * b;
  const cubedL = lRoot ** 3;
  const cubedM = mRoot ** 3;
  const cubedS = sRoot ** 3;
  return [
    fromLinear(4.0767416621 * cubedL - 3.3077115913 * cubedM + 0.2309699292 * cubedS),
    fromLinear(-1.2684380046 * cubedL + 2.6097574011 * cubedM - 0.3413193965 * cubedS),
    fromLinear(-0.0041960863 * cubedL - 0.7034186147 * cubedM + 1.707614701 * cubedS),
  ];
}
