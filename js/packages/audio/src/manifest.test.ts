// Tests for the manifest reader beyond the fixtures: the defaults, the error object, the lenient
// reading of the envelope, the custom buses, and the grammar of the file paths.

import { describe, expect, it } from "vitest";
import { AudioError } from "./error";
import { parseAudioManifest, parseAudioManifestLenient } from "./manifest";

/** A document with the envelope and the given body, such as `"sounds": []`. */
function document(body: string): string {
  return `{ "format": "velvet-audio", "version": 1, ${body} }`;
}

/** Runs a call and returns the AudioError it throws. Fails the test for any other outcome. */
function audioErrorOf(call: () => unknown): AudioError {
  try {
    call();
  } catch (error) {
    if (error instanceof AudioError) return error;
    throw error;
  }
  throw new Error("expected the call to throw an AudioError");
}

describe("defaults of a sound", () => {
  it("fills in every default of a minimal sound", () => {
    const manifest = parseAudioManifest(document(`"sounds": [{ "id": "ui.click", "files": ["ui/click.opus"] }]`));
    expect(manifest.format).toBe("velvet-audio");
    expect(manifest.version).toBe(1);
    expect(manifest.buses).toEqual([]);
    expect(manifest.sounds).toEqual([
      {
        id: "ui.click",
        bus: "sfx",
        volume: 1,
        loop: false,
        loop_points: null,
        max_voices: null,
        cooldown_ms: 0,
        preload: false,
        variants: [{ files: ["ui/click.opus"] }],
        meta: {},
      },
    ]);
  });

  it("accepts a manifest with no sounds", () => {
    expect(parseAudioManifest(document(`"sounds": []`)).sounds).toEqual([]);
  });

  it("keeps the notes and ignores fields it does not know", () => {
    const manifest = parseAudioManifest(
      document(`"sounds": [{ "id": "ui.click", "files": ["ui/click.opus"], "colour": "red", "meta": { "prompt": "a tick" } }]`),
    );
    const [sound] = manifest.sounds;
    expect(sound?.meta).toEqual({ prompt: "a tick" });
    expect(sound).not.toHaveProperty("colour");
  });

  it("reads loop points for a looped sound", () => {
    const manifest = parseAudioManifest(
      document(`"sounds": [{ "id": "music.a", "bus": "music", "files": ["music/a.opus"], "loop": true, "loop_points": { "start": 1, "end": 4.5 } }]`),
    );
    expect(manifest.sounds[0]?.loop_points).toEqual({ start: 1, end: 4.5 });
  });
});

describe("errors", () => {
  it("reports every problem with its path, and gives the first problem's code", () => {
    const error = audioErrorOf(() =>
      parseAudioManifest(
        document(`"sounds": [{ "id": "Bad", "files": [] }, { "id": "ok.one", "files": ["a/b.flac"], "volume": 2 }]`),
      ),
    );
    expect(error.code).toBe("value");
    expect(error.issues.map((issue) => issue.path)).toEqual([
      "sounds[0].id",
      "sounds[0].files",
      "sounds[1].files[0]",
      "sounds[1].volume",
    ]);
    expect(error.message).toContain("4 problems");
  });

  it("refuses text that is not JSON with the json code", () => {
    expect(audioErrorOf(() => parseAudioManifest("{ nope")).code).toBe("json");
  });

  it("refuses a wrong format tag and a wrong version", () => {
    expect(audioErrorOf(() => parseAudioManifest(`{ "format": "velvet-art", "version": 1, "sounds": [] }`)).code).toBe("format");
    expect(audioErrorOf(() => parseAudioManifest(`{ "format": "velvet-audio", "version": 3, "sounds": [] }`)).code).toBe("version");
  });

  it("refuses a sound that names both files and variants", () => {
    const error = audioErrorOf(() =>
      parseAudioManifest(
        document(`"sounds": [{ "id": "ui.click", "files": ["ui/a.opus"], "variants": [{ "files": ["ui/b.opus"] }, { "files": ["ui/c.opus"] }] }]`),
      ),
    );
    expect(error.issues[0]?.path).toBe("sounds[0]");
  });
});

describe("lenient reading", () => {
  it("gives the same manifest and no issues for a valid document", () => {
    const text = document(`"sounds": [{ "id": "ui.click", "files": ["ui/click.opus"] }]`);
    const lenient = parseAudioManifestLenient(text);
    expect(lenient.issues).toEqual([]);
    expect(lenient.manifest).toEqual(parseAudioManifest(text));
  });

  it("still refuses the envelope", () => {
    expect(audioErrorOf(() => parseAudioManifestLenient("[")).code).toBe("json");
    expect(audioErrorOf(() => parseAudioManifestLenient(`{ "format": "x", "version": 1, "sounds": [] }`)).code).toBe("format");
    expect(audioErrorOf(() => parseAudioManifestLenient(document(`"sounds": {}`))).code).toBe("structure");
    expect(audioErrorOf(() => parseAudioManifestLenient(document(`"buses": "a", "sounds": []`))).code).toBe("structure");
  });

  it("keeps the good sounds and reports the bad ones", () => {
    const result = parseAudioManifestLenient(
      document(`"sounds": [{ "id": "ui.click", "files": ["ui/click.opus"] }, { "id": "ui.bad", "files": ["ui/x.flac"] }]`),
    );
    expect(result.manifest.sounds.map((sound) => sound.id)).toEqual(["ui.click"]);
    expect(result.issues.map((issue) => issue.path)).toEqual(["sounds[1].files[0]"]);
  });
});

describe("custom buses", () => {
  it("lets a sound use a declared bus", () => {
    const manifest = parseAudioManifest(
      document(`"buses": ["ambience"], "sounds": [{ "id": "ambience.wind", "bus": "ambience", "files": ["ambience/wind.opus"] }]`),
    );
    expect(manifest.buses).toEqual(["ambience"]);
    expect(manifest.sounds[0]?.bus).toBe("ambience");
  });

  it("refuses a sound on a bus that is not declared", () => {
    const error = audioErrorOf(() =>
      parseAudioManifest(document(`"sounds": [{ "id": "ambience.wind", "bus": "ambience", "files": ["ambience/wind.opus"] }]`)),
    );
    expect(error.issues[0]?.path).toBe("sounds[0].bus");
  });

  it("refuses the reserved name master, a built-in name, a malformed name and a repeated name", () => {
    const codes = (buses: string) =>
      audioErrorOf(() => parseAudioManifest(document(`"buses": [${buses}], "sounds": []`))).issues.map((issue) => issue.code);
    expect(codes(`"master"`)).toEqual(["value"]);
    expect(codes(`"music"`)).toEqual(["duplicate"]);
    expect(codes(`"Ambience"`)).toEqual(["value"]);
    expect(codes(`"fx", "fx"`)).toEqual(["duplicate"]);
  });
});

describe("file paths", () => {
  it("accepts folders and a dotted name", () => {
    const manifest = parseAudioManifest(document(`"sounds": [{ "id": "sfx.door", "files": ["sfx/door.open.opus"] }]`));
    expect(manifest.sounds[0]?.variants[0]?.files).toEqual(["sfx/door.open.opus"]);
  });

  it("refuses uppercase letters, a leading slash, a backslash and a missing extension", () => {
    // The backslash is doubled so that the JSON text holds a valid escape for it.
    for (const path of ["SFX/door.opus", "/sfx/door.opus", "sfx\\\\door.opus", "sfx/door"]) {
      const error = audioErrorOf(() => parseAudioManifest(document(`"sounds": [{ "id": "sfx.door", "files": ["${path}"] }]`)));
      expect(error.issues[0]?.code, path).toBe("value");
    }
  });
});
