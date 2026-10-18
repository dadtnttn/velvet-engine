// Writes the sounds of hello-2d: five short 16-bit PCM WAV files that the audio manifest names
// (public/audio/manifest.json). It needs no packages, only Node.js, so `node tools/make-sounds.mjs` from the
// example folder rewrites the files, and the same bytes come out every time.
//
// - sfx/jump.wav: a quick rising chirp.
// - sfx/coin.wav: two bright notes, the second higher.
// - sfx/hurt.wav: a falling tone with a little noise under it.
// - sfx/win.wav: a rising arpeggio that ends on a held chord.
// - ui/click.wav: a short click for the menus.

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

/** Mixes one sound into another, starting at a time in seconds. */
function mixInto(target, source, startSeconds) {
  const start = Math.round(startSeconds * RATE);
  for (let index = 0; index < source.length && start + index < target.length; index += 1) {
    target[start + index] += source[index];
  }
  return target;
}

/** Deterministic noise (a seeded linear congruential generator), from -0.5 to 0.5. */
function noiseSource(seed) {
  let state = seed;
  return () => {
    state = (state * 1103515245 + 12345) & 0x7fffffff;
    return state / 0x7fffffff - 0.5;
  };
}

// The jump: a chirp from 260 Hz up to 620 Hz, with a fast attack and a quick decay.
const jumpSeconds = 0.16;
let jumpPhase = 0;
const jump = render(jumpSeconds, (t) => {
  jumpPhase += (2 * Math.PI * (260 + (360 * t) / jumpSeconds)) / RATE;
  const attack = Math.min(1, t / 0.005);
  return 0.32 * attack * Math.exp(-t * 16) * Math.sign(Math.sin(jumpPhase)) * 0.8;
});

// The coin: 988 Hz, then 1319 Hz a moment later, each with a quick decay.
const coin = mixInto(
  render(0.22, (t) => 0.4 * Math.min(1, t / 0.003) * Math.exp(-t * 24) * Math.sin(2 * Math.PI * 988 * t)),
  render(0.2, (t) => 0.4 * Math.min(1, t / 0.003) * Math.exp(-t * 22) * Math.sin(2 * Math.PI * 1319 * t)),
  0.06,
);

// The hurt: a tone that falls from 200 Hz to 70 Hz, with noise under it.
const hurtSeconds = 0.36;
const noise = noiseSource(4242);
let hurtPhase = 0;
const hurt = render(hurtSeconds, (t) => {
  hurtPhase += (2 * Math.PI * (200 - (130 * t) / hurtSeconds)) / RATE;
  const envelope = Math.min(1, t / 0.004) * Math.exp(-t * 6);
  return 0.5 * envelope * (0.6 * Math.sin(hurtPhase) + 0.8 * noise());
});

// The win: C5, E5, G5, C6 in turn, then a held chord of the four notes.
const notes = [523.25, 659.25, 783.99, 1046.5];
const win = render(0.9, () => 0);
notes.forEach((hz, index) => {
  mixInto(
    win,
    render(0.22, (t) => 0.28 * Math.min(1, t / 0.004) * Math.exp(-t * 9) * Math.sin(2 * Math.PI * hz * t)),
    index * 0.12,
  );
});
for (const hz of notes) {
  mixInto(
    win,
    render(0.5, (t) => 0.12 * Math.min(1, t / 0.01) * Math.exp(-t * 4) * Math.sin(2 * Math.PI * hz * t)),
    0.5,
  );
}

// The click: a short burst of noise with a bright tone under it.
const clickNoise = noiseSource(12345);
const click = render(0.045, (t) => {
  const envelope = Math.exp(-t * 120);
  return 0.55 * envelope * (0.6 * clickNoise() + 0.4 * Math.sin(2 * Math.PI * 1800 * t));
});

writeWav("sfx/jump.wav", jump);
writeWav("sfx/coin.wav", coin);
writeWav("sfx/hurt.wav", hurt);
writeWav("sfx/win.wav", win);
writeWav("ui/click.wav", click);
