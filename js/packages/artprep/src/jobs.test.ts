// Checks the job builder: the default seeds and output names, the size rule, the settings a job copies,
// the notes that the art manifest gives, and the reporting of several problems at once.

import { parseArtManifest } from "@velvet/assets";
import { describe, expect, it } from "vitest";
import {
  ArtPrepError,
  buildJobs,
  jobSize,
  outputName,
  parseIdentity,
  parseProfiles,
  parseShots,
  type ArtPrepProfile,
  type Shot,
  type StyleLayer,
} from "./index";

/** The art manifest of these tests: a portrait with a variant and notes for generation, and a prop. */
const manifest = parseArtManifest(
  JSON.stringify({
    format: "velvet-art",
    version: 1,
    entries: [
      {
        id: "character.tess.portrait",
        kind: "portrait",
        size: [480, 640],
        variants: ["happy"],
        meta: {
          prompt: "a friendly guide",
          style: ["soft lines"],
          negative: "extra fingers, smudged text",
          notes: "for people only, never read by the code",
        },
      },
      { id: "prop.lamp", kind: "prop", size: [128, 160] },
    ],
  }),
);

/** A profile made from the fields given, read by the profile reader as a file would be. */
function profileOf(fields: Record<string, unknown>): ArtPrepProfile {
  const document = {
    format: "velvet-artprep",
    version: 1,
    kind: "profiles",
    profiles: [
      {
        id: "test",
        mode: "generate",
        generation: { model: "model", sampler: "sampler", steps: 8, cfg: 1 },
        size: { long_side: 1024, multiple_of: 64 },
        negative: false,
        prompt: { style: "prose" },
        ...fields,
      },
    ],
  };
  const [profile] = parseProfiles(JSON.stringify(document));
  if (profile === undefined) throw new Error("the profile did not read");
  return profile;
}

/**
 * One shot, read by the shot reader. Unless the fields give a variation, the shot has a framing, so that
 * its prompt is never empty.
 */
function shotOf(fields: Record<string, unknown>): Shot {
  const written = { id: "shot", variation: { framing: "three-quarter view" }, ...fields };
  const [shot] = parseShots(JSON.stringify({ format: "velvet-artprep", version: 1, kind: "shots", shots: [written] }));
  if (shot === undefined) throw new Error("the shot did not read");
  return shot;
}

describe("buildJobs: seeds and output names", () => {
  it("makes six candidates per shot, named after the picture, when no seeds are given", () => {
    const [job] = buildJobs(manifest.entries, [], [shotOf({ art: "prop.lamp" })], profileOf({}));
    expect(job?.seeds).toEqual([101, 102, 103, 104, 105, 106]);
    expect(job?.outputs.map((output) => output.name)).toEqual([
      "prop.lamp_101",
      "prop.lamp_102",
      "prop.lamp_103",
      "prop.lamp_104",
      "prop.lamp_105",
      "prop.lamp_106",
    ]);
  });

  it("puts the variant between the art id and the seed", () => {
    const [job] = buildJobs(
      manifest.entries,
      [],
      [shotOf({ art: "character.tess.portrait", variant: "happy" })],
      profileOf({ max_identities: 1 }),
      [7],
    );
    expect(job?.outputs).toEqual([{ seed: 7, name: "character.tess.portrait.happy_7" }]);
  });

  it("names outputs with a variant or without one", () => {
    expect(outputName("prop.lamp", null, 3)).toBe("prop.lamp_3");
    expect(outputName("character.tess.portrait", "happy", 3)).toBe("character.tess.portrait.happy_3");
  });
});

describe("jobSize", () => {
  const rule = { longSide: 1024, multipleOf: 64 };

  it("gives the long side the rule's length and keeps the proportion of the box", () => {
    expect(jobSize([480, 640], rule)).toEqual([768, 1024]);
    expect(jobSize([640, 1080], rule)).toEqual([576, 1024]);
    expect(jobSize([1920, 1080], rule)).toEqual([1024, 576]);
    expect(jobSize([64, 64], rule)).toEqual([1024, 1024]);
  });

  it("never gives the short side less than one multiple", () => {
    expect(jobSize([10, 1000], { longSide: 512, multipleOf: 64 })).toEqual([64, 512]);
  });

  it("rounds the short side down to the multiple", () => {
    expect(jobSize([128, 160], { longSide: 768, multipleOf: 64 })).toEqual([576, 768]);
  });
});

describe("buildJobs: what a job copies and what it reads", () => {
  it("copies the shift and the scheduler of the profile into the job", () => {
    const profile = profileOf({
      generation: {
        model: "m",
        loras: [{ name: "style", weight: 0.5 }],
        sampler: "s",
        scheduler: "simple",
        steps: 4,
        cfg: 1,
        shift: 3,
      },
    });
    const [job] = buildJobs(manifest.entries, [], [shotOf({ art: "prop.lamp" })], profile);
    expect(job?.generation).toEqual({
      model: "m",
      loras: [{ name: "style", weight: 0.5 }],
      sampler: "s",
      scheduler: "simple",
      steps: 4,
      cfg: 1,
      shift: 3,
    });
  });

  it("adds the style and the negative notes of the art entry, and uses its prompt as the subject", () => {
    const style: StyleLayer = { phrases: [{ text: "flat colours", weight: 1 }], negative: ["photo"] };
    const profile = profileOf({ negative: true, prompt: { style: "tags" } });
    const [job] = buildJobs(
      manifest.entries,
      [],
      [shotOf({ art: "character.tess.portrait", variant: "happy", variation: { framing: "half length" } })],
      profile,
      undefined,
      style,
    );
    expect(job?.prompt).toBe("flat colours, soft lines, a friendly guide, half length");
    expect(job?.negative).toBe("photo, extra fingers, smudged text");
  });

  it("never reads the notes key of the art entry, which is for people", () => {
    const profile = profileOf({ prompt: { style: "tags" } });
    const [job] = buildJobs(
      manifest.entries,
      [],
      [shotOf({ art: "character.tess.portrait", variant: "happy" })],
      profile,
    );
    expect(job?.prompt).not.toContain("for people");
  });

  it("lets the shot's own subject replace the subject of the art entry", () => {
    const profile = profileOf({ prompt: { style: "tags" } });
    const [job] = buildJobs(
      manifest.entries,
      [],
      [shotOf({ art: "character.tess.portrait", variant: "happy", variation: { subject: "a fox scout" } })],
      profile,
    );
    expect(job?.prompt).toBe("soft lines, a fox scout");
  });

  it("reports the references of the job by art id, without a file path", () => {
    const profile = profileOf({ max_references: 1 });
    const [job] = buildJobs(
      manifest.entries,
      [],
      [shotOf({ art: "prop.lamp", references: ["character.tess.portrait"] })],
      profile,
    );
    expect(job?.references).toEqual([{ id: "character.tess.portrait", variant: null }]);
  });
});

describe("buildJobs: refusals", () => {
  it("lists every problem of a batch, not only the first", () => {
    let thrown: unknown;
    try {
      buildJobs(
        manifest.entries,
        [],
        [shotOf({ id: "one", art: "nope.one" }), shotOf({ id: "two", art: "nope.two" })],
        profileOf({}),
      );
    } catch (error) {
      thrown = error;
    }
    expect(thrown).toBeInstanceOf(ArtPrepError);
    if (thrown instanceof ArtPrepError) {
      expect(thrown.issues.map((issue) => issue.code)).toEqual(["unknown-id", "unknown-id"]);
      expect(thrown.issues.map((issue) => issue.path)).toEqual(["shots[0].art", "shots[1].art"]);
    }
  });

  it("refuses two identity layers that share one character id", () => {
    const ada = parseIdentity(
      JSON.stringify({ format: "velvet-artprep", version: 1, kind: "identity", id: "ada", name: "Ada" }),
    );
    expect(() => buildJobs(manifest.entries, [ada, ada], [shotOf({ art: "prop.lamp" })], profileOf({}))).toThrow(
      ArtPrepError,
    );
  });

  it("refuses a seed that is not a whole number from 0 to 2^32 - 1", () => {
    expect(() => buildJobs(manifest.entries, [], [shotOf({ art: "prop.lamp" })], profileOf({}), [-1])).toThrow(
      ArtPrepError,
    );
  });
});
