// Checks the composition rules one at a time, with small profiles and identities written inline, so that
// each rule has its own case: weights, negatives, edit guards, the layer order and the trigger words.

import { describe, expect, it } from "vitest";
import {
  composePrompt,
  parseIdentity,
  parseProfiles,
  type ArtPrepProfile,
  type CharacterIdentity,
  type StyleLayer,
} from "./index";

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

/** An identity made from the fields given, read by the identity reader. The name is Tess unless given. */
function identityOf(fields: Record<string, unknown>): CharacterIdentity {
  return parseIdentity(
    JSON.stringify({ format: "velvet-artprep", version: 1, kind: "identity", id: "tess", name: "Tess", ...fields }),
  );
}

const EMPTY_STYLE: StyleLayer = { phrases: [], negative: [] };

describe("composePrompt: weights", () => {
  it("writes a weight other than 1 with the paren syntax in a tag profile, and leaves 1 plain", () => {
    const profile = profileOf({ prompt: { style: "tags" } });
    const tess = identityOf({ fixed: [{ text: "bright eyes", weight: 1.25 }, "freckles"] });
    expect(composePrompt(EMPTY_STYLE, tess, {}, profile).prompt).toBe("Tess, (bright eyes:1.25), freckles");
  });

  it("drops weights when the profile writes none", () => {
    const profile = profileOf({ prompt: { style: "tags", weights: "none" } });
    const tess = identityOf({ fixed: [{ text: "bright eyes", weight: 1.25 }] });
    expect(composePrompt(EMPTY_STYLE, tess, {}, profile).prompt).toBe("Tess, bright eyes");
  });

  it("writes a prose weight as plain text, since prose has no weight syntax", () => {
    const profile = profileOf({ prompt: { style: "prose" } });
    const tess = identityOf({ fixed: [{ text: "bright eyes", weight: 1.25 }] });
    expect(composePrompt(EMPTY_STYLE, tess, {}, profile).prompt).toBe("Tess, bright eyes.");
  });
});

describe("composePrompt: negatives and what the identities lack", () => {
  it("writes a lacked feature as 'no X' in the text when the profile takes no negative prompt", () => {
    const profile = profileOf({ negative: false });
    const tess = identityOf({ lacks: ["horns"] });
    const composed = composePrompt(EMPTY_STYLE, tess, {}, profile);
    expect(composed.prompt).toBe("Tess, no horns.");
    expect(composed.negative).toBeNull();
  });

  it("writes a lacked feature in the negative prompt, and not in the text, when the profile takes one", () => {
    const profile = profileOf({ negative: true, prompt: { style: "tags" } });
    const tess = identityOf({ lacks: ["horns"] });
    const composed = composePrompt({ phrases: [], negative: ["photo"] }, tess, {}, profile);
    expect(composed.prompt).toBe("Tess");
    expect(composed.negative).toBe("photo, horns");
  });

  it("leaves a lacked feature out of the negative when another character in the picture has it", () => {
    const profile = profileOf({ negative: true, max_identities: 2, prompt: { style: "tags" } });
    const ada = identityOf({ id: "ada", name: "Ada", lacks: ["horns", "tail"] });
    const bex = identityOf({ id: "bex", name: "Bex", has: ["two short curved horns"] });
    const composed = composePrompt({ phrases: [], negative: ["photo"] }, [ada, bex], {}, profile);
    expect(composed.negative).toBe("photo, tail");
  });

  it("returns no negative when the profile takes none, even if the style has negatives", () => {
    const profile = profileOf({ negative: false, prompt: { style: "tags" } });
    expect(composePrompt({ phrases: [], negative: ["photo"] }, identityOf({}), {}, profile).negative).toBeNull();
  });
});

describe("composePrompt: layers, joiners and triggers", () => {
  it("writes prose layers as sentences, each with a capital letter, and leads with the trigger words", () => {
    const profile = profileOf({ trigger_words: ["@KEY"] });
    const style: StyleLayer = { phrases: [{ text: "soft light", weight: 1 }], negative: [] };
    expect(composePrompt(style, identityOf({}), { framing: { text: "half length", weight: 1 } }, profile).prompt).toBe(
      "@KEY Soft light. Tess. Half length.",
    );
  });

  it("follows the layer order of the profile", () => {
    const profile = profileOf({ prompt: { style: "tags", order: ["variation", "identity", "style"] } });
    const style: StyleLayer = { phrases: [{ text: "flat colours", weight: 1 }], negative: [] };
    const variation = { setting: { text: "a wooden table", weight: 1 } };
    expect(composePrompt(style, identityOf({}), variation, profile).prompt).toBe("a wooden table, Tess, flat colours");
  });

  it("writes quality tags and trigger words first, in tag profiles", () => {
    const profile = profileOf({
      trigger_words: ["@KEY"],
      prompt: { style: "tags", quality_tags: ["masterpiece"] },
    });
    const style: StyleLayer = { phrases: [{ text: "soft light", weight: 1 }], negative: [] };
    expect(composePrompt(style, identityOf({}), {}, profile).prompt).toBe("masterpiece, @KEY, soft light, Tess");
  });

  it("gives an empty prompt when no layer has anything to say", () => {
    const profile = profileOf({ prompt: { style: "tags", order: ["style", "identity", "variation"] } });
    const composed = composePrompt(EMPTY_STYLE, [], {}, profile);
    expect(composed.prompt).toBe("");
    expect(composed.negative).toBeNull();
  });
});

describe("composePrompt: edit guards", () => {
  it("adds the guard of a field right after the field, in an edit profile", () => {
    const profile = profileOf({
      mode: "edit",
      max_references: 1,
      edit_guards: { pose: "nothing left where the moved part was" },
    });
    const variation = { pose: { text: "the arm raised", weight: 1 } };
    expect(composePrompt(EMPTY_STYLE, identityOf({}), variation, profile).prompt).toBe(
      "Tess. The arm raised, nothing left where the moved part was.",
    );
  });

  it("adds no guard to a field that has none", () => {
    const profile = profileOf({
      mode: "edit",
      max_references: 1,
      edit_guards: { pose: "nothing left where the moved part was" },
    });
    const variation = { outfit: { text: "a green scarf", weight: 1 } };
    expect(composePrompt(EMPTY_STYLE, identityOf({}), variation, profile).prompt).toBe("Tess. A green scarf.");
  });
});
