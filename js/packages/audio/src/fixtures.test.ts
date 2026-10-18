// Runs the velvet-audio conformance suite, docs/formats/fixtures/audio/cases.json. The suite is shared
// with other readers, so each case states its results in the format's own terms: the codes and paths
// of a refusal, the ids and codes of a lenient read, the normal form of a sound, and the gain of a
// volume.

import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { AudioError, type AudioErrorCode } from "./error";
import { parseAudioManifest, parseAudioManifestLenient } from "./manifest";
import type { AudioManifest } from "./types";
import { volumeToDecibels, volumeToGain } from "./volume";

const FIXTURES = new URL("../../../../docs/formats/fixtures/audio/", import.meta.url);

/** A manifest that must be refused. The path is checked when given. */
interface InvalidCase {
  readonly file: string;
  readonly code: AudioErrorCode;
  readonly path?: string;
}

/** A manifest that a lenient reader repairs: the ids it keeps, and the codes of its issues in order. */
interface LenientCase {
  readonly file: string;
  readonly kept: readonly string[];
  readonly codes: readonly AudioErrorCode[];
}

/** A sound whose normal form has the listed fields. Other fields are not checked. */
interface SoundCase {
  readonly id: string;
  readonly fields: Readonly<Record<string, unknown>>;
}

/** One point of the volume curve. */
interface CurveCase {
  readonly volume: number;
  readonly gain: number;
  readonly decibels?: number;
}

/** The whole cases file. */
interface Cases {
  readonly manifest: string;
  readonly buses: readonly string[];
  readonly invalid: readonly InvalidCase[];
  readonly lenient: readonly LenientCase[];
  readonly sounds: readonly SoundCase[];
  readonly curve: readonly CurveCase[];
}

/** Reads a file of the fixtures folder. */
function read(name: string): string {
  return readFileSync(new URL(name, FIXTURES), "utf8");
}

/** Picks the named fields of a sound, as a plain record, so that they compare by value. */
function pickFields(sound: AudioManifest["sounds"][number], names: readonly string[]): Record<string, unknown> {
  const all: Record<string, unknown> = { ...sound };
  return Object.fromEntries(names.map((name) => [name, all[name]]));
}

const cases = JSON.parse(read("cases.json")) as Cases;
const manifest = parseAudioManifest(read(cases.manifest));

describe("velvet-audio conformance: the valid manifest", () => {
  it("declares the custom buses of the fixture", () => {
    expect(manifest.buses).toEqual(cases.buses);
  });

  it("keeps the sounds in the order of the file", () => {
    expect(manifest.sounds.map((sound) => sound.id)).toEqual(cases.sounds.map((sound) => sound.id));
  });

  for (const testCase of cases.sounds) {
    it(`${testCase.id} has the fields of the fixture`, () => {
      const sound = manifest.sounds.find((item) => item.id === testCase.id);
      expect(sound).toBeDefined();
      if (sound === undefined) return;
      const names = Object.keys(testCase.fields);
      expect(pickFields(sound, names)).toEqual(testCase.fields);
    });
  }
});

describe("velvet-audio conformance: manifests that are refused", () => {
  for (const testCase of cases.invalid) {
    it(`${testCase.file} is refused with "${testCase.code}"`, () => {
      let caught: unknown = null;
      try {
        parseAudioManifest(read(testCase.file));
      } catch (error) {
        caught = error;
      }
      expect(caught).toBeInstanceOf(AudioError);
      if (!(caught instanceof AudioError)) return;
      expect(caught.code).toBe(testCase.code);
      expect(caught.issues[0]?.code).toBe(testCase.code);
      if (testCase.path !== undefined) {
        expect(caught.issues.map((issue) => issue.path)).toContain(testCase.path);
      }
    });
  }
});

describe("velvet-audio conformance: lenient reading", () => {
  for (const testCase of cases.lenient) {
    it(`${testCase.file} keeps ${testCase.kept.join(", ")}`, () => {
      const result = parseAudioManifestLenient(read(testCase.file));
      expect(result.manifest.sounds.map((sound) => sound.id)).toEqual(testCase.kept);
      expect(result.issues.map((issue) => issue.code)).toEqual(testCase.codes);
    });
  }
});

describe("velvet-audio conformance: the volume curve", () => {
  for (const point of cases.curve) {
    it(`volume ${point.volume} has gain ${point.gain}`, () => {
      expect(volumeToGain(point.volume)).toBeCloseTo(point.gain, 9);
      if (point.decibels !== undefined) {
        expect(volumeToDecibels(point.volume)).toBeCloseTo(point.decibels, 3);
      }
    });
  }
});
