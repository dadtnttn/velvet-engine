// Colours for placeholders: an HSL colour written as `#rrggbb`, and the rule that picks the ink
// (the colour of text and lines) which reads on a given colour.

/** Keeps a number between 0 and 1. */
function clamp01(value: number): number {
  return Math.min(1, Math.max(0, value));
}

/** One channel as two lowercase hex digits. */
function channelHex(value: number): string {
  return Math.round(clamp01(value) * 255)
    .toString(16)
    .padStart(2, "0");
}

/**
 * Converts HSL to a hex colour, with the same formula that CSS `hsl()` uses. Each channel is
 * rounded to the nearest of 0 to 255.
 * @param hue - degrees. Any number: it is wrapped into 0 to 360.
 * @param saturation - from 0 to 1.
 * @param lightness - from 0 to 1.
 * @returns the colour as `#rrggbb` in lowercase.
 */
export function hslToHex(hue: number, saturation: number, lightness: number): string {
  const degrees = ((hue % 360) + 360) % 360;
  const s = clamp01(saturation);
  const l = clamp01(lightness);
  const chroma = (1 - Math.abs(2 * l - 1)) * s;
  const sector = degrees / 60;
  const secondary = chroma * (1 - Math.abs((sector % 2) - 1));
  let red = 0;
  let green = 0;
  let blue = 0;
  if (sector < 1) {
    [red, green, blue] = [chroma, secondary, 0];
  } else if (sector < 2) {
    [red, green, blue] = [secondary, chroma, 0];
  } else if (sector < 3) {
    [red, green, blue] = [0, chroma, secondary];
  } else if (sector < 4) {
    [red, green, blue] = [0, secondary, chroma];
  } else if (sector < 5) {
    [red, green, blue] = [secondary, 0, chroma];
  } else {
    [red, green, blue] = [chroma, 0, secondary];
  }
  const match = l - chroma / 2;
  return `#${channelHex(red + match)}${channelHex(green + match)}${channelHex(blue + match)}`;
}

/**
 * The relative luma of a `#rrggbb` colour, from 0 (black) to 1 (white). It uses the gamma-encoded
 * channels, weighted 0.2126, 0.7152 and 0.0722.
 * @param hex - the colour as `#rrggbb`.
 */
export function luma(hex: string): number {
  const red = Number.parseInt(hex.slice(1, 3), 16) / 255;
  const green = Number.parseInt(hex.slice(3, 5), 16) / 255;
  const blue = Number.parseInt(hex.slice(5, 7), 16) / 255;
  return 0.2126 * red + 0.7152 * green + 0.0722 * blue;
}

/**
 * The ink for text and lines on a colour: near-black on a light colour (luma above 0.5), white on
 * a dark one.
 * @param hex - the background colour as `#rrggbb`.
 */
export function inkFor(hex: string): string {
  return luma(hex) > 0.5 ? "#1a1a1a" : "#ffffff";
}
