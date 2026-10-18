// Volume curves. A volume is a slider position from 0 to 1. It maps to decibels on a curve of 60 dB
// per decade, so 0.1 is -60 dB, 0.5 is about -18 dB and 1 is 0 dB. Zero is silence. Gains multiply,
// so the levels of a sound, a play, a bus and the master add up in decibels. docs/formats/AUDIO.md
// gives the curve as a rule; the fixtures check its values.

import { checkRange } from "./checks";

/** Decibels per decade of the volume curve. */
const DECIBELS_PER_DECADE = 60;

/**
 * The level in decibels of a volume: 60 · log10(volume). Zero is silence, and comes out as -Infinity.
 * @throws RangeError when the volume is not a number from 0 to 1.
 */
export function volumeToDecibels(volume: number): number {
  checkRange(volume, 0, 1, "volume");
  return volume === 0 ? Number.NEGATIVE_INFINITY : DECIBELS_PER_DECADE * Math.log10(volume);
}

/**
 * The gain of a level in decibels: 10 to the power of decibels over 20. -Infinity gives silence.
 * @throws RangeError when the level is NaN or +Infinity.
 */
export function decibelsToGain(decibels: number): number {
  if (Number.isNaN(decibels) || decibels === Number.POSITIVE_INFINITY) {
    throw new RangeError(`decibels must be a number below infinity, got ${decibels}`);
  }
  return decibels === Number.NEGATIVE_INFINITY ? 0 : 10 ** (decibels / 20);
}

/**
 * The gain of a volume from 0 to 1, on the curve of {@link volumeToDecibels}. The gain is v cubed.
 * @throws RangeError when the volume is not a number from 0 to 1.
 */
export function volumeToGain(volume: number): number {
  return decibelsToGain(volumeToDecibels(volume));
}
