// Writes the sounds of hello-vn: three short 16-bit PCM WAV files that the audio manifest names
// (public/audio/manifest.json). It needs no packages, only Node.js, so `node tools/make-sounds.mjs`
// from the example folder rewrites the files, and the same bytes come out every time.
//
// - ui/blip.wav: a text blip, a sweep of about 70 ms that plays once per letter.
// - ui/click.wav: a select click, a short burst of noise with a bright tone under it.
// - music/pad.wav: a soft pad of four seconds. Every partial completes a whole number of cycles in
//   four seconds, and so does the tremolo, so the file loops without a seam.

import { mkdirSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

/** Samples per second of every file. */
const RATE = 22050;

/** The folder that the audio manifest's paths are relative to. */
const OUT = join(dirname(fileURLToPath(import.meta.url)), "..", "public", "audio");

/**
 * Writes mono 16-bit PCM samples as a WAV file.
 * @param path - the file to write, relative to the audio folder.
 * @param samples - samples from -1 to 1.
 */
function writeWav(path, samples) {
  const dataBytes = samples.length * 2;
  const bytes = Buffer.alloc(44 + dataBytes);
  bytes.write("RIFF", 0, "ascii");
  bytes.writeUInt32LE(36 + dataBytes, 4);
  bytes.write("WAVE", 8, "ascii");
  bytes.write("fmt ", 12, "ascii");
  bytes.writeUInt32LE(16, 16); // the size of the fmt chunk
  bytes.writeUInt16LE(1, 20); // PCM
  bytes.writeUInt16LE(1, 22); // mono
  bytes.writeUInt32LE(RATE, 24);
  bytes.writeUInt32LE(RATE * 2, 28); // bytes per second
  bytes.writeUInt16LE(2, 32); // bytes per frame
  bytes.writeUInt16LE(16, 34); // bits per sample
  bytes.write("data", 36, "ascii");
  bytes.writeUInt32LE(dataBytes, 40);
  for (let index = 0; index < samples.length; index += 1) {
    const clamped = Math.max(-1, Math.min(1, samples[index]));
    bytes.writeInt16LE(Math.round(clamped * 32767), 44 + index * 2);
  }
  const target = join(OUT, path);
  mkdirSync(dirname(target), { recursive: true });
  writeFileSync(target, bytes);
  console.log(`wrote public/audio/${path} (${bytes.length} bytes, ${(samples.length / RATE).toFixed(2)} s)`);
}

/** A sound of `seconds` length, made sample by sample from its time in seconds. */
function render(seconds, sample) {
  const count = Math.round(seconds * RATE);
  const out = new Float32Array(count);
  for (let index = 0; index < count; index += 1) out[index] = sample(index / RATE, index);
  return out;
}

// The text blip: a sweep from 1040 Hz down to 880 Hz, with a fast attack and a quick decay.
const blipSeconds = 0.07;
const blipSweep = (t) => 1040 - (160 * t) / blipSeconds;
let blipPhase = 0;
const blip = render(blipSeconds, (t) => {
  blipPhase += (2 * Math.PI * blipSweep(t)) / RATE;
  const attack = Math.min(1, t / 0.002);
  return 0.45 * attack * Math.exp(-t * 45) * Math.sin(blipPhase);
});

// The select click: deterministic noise (a linear congruential generator, seeded) with a short tone.
let seed = 12345;
const noise = () => {
  seed = (seed * 1103515245 + 12345) & 0x7fffffff;
  return seed / 0x7fffffff - 0.5;
};
const click = render(0.045, (t) => {
  const envelope = Math.exp(-t * 120);
  return 0.55 * envelope * (0.6 * noise() + 0.4 * Math.sin(2 * Math.PI * 1800 * t));
});

// The pad: partials on multiples of 0.25 Hz, so each one fits a whole number of cycles in four
// seconds. The tremolo has a four-second period too, so the last sample flows into the first.
const padSeconds = 4;
const partials = [
  { hz: 110, level: 0.5 },
  { hz: 165, level: 0.3 },
  { hz: 220, level: 0.35 },
  { hz: 277.25, level: 0.2 },
];
const pad = render(padSeconds, (t) => {
  let sum = 0;
  for (const partial of partials) sum += partial.level * Math.sin(2 * Math.PI * partial.hz * t);
  const tremolo = 0.8 + 0.2 * Math.sin((2 * Math.PI * t) / padSeconds);
  return 0.22 * tremolo * sum;
});

writeWav("ui/blip.wav", blip);
writeWav("ui/click.wav", click);
writeWav("music/pad.wav", pad);
