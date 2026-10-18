// Checks the names that the package root exports, so that a missing export fails a test.

import { describe, expect, it } from "vitest";
import * as artprep from "./index";

describe("@velvet/artprep exports", () => {
  it("exports the readers, the composition, the jobs, the gallery and the error", () => {
    expect(typeof artprep.parseProfiles).toBe("function");
    expect(typeof artprep.parseStyle).toBe("function");
    expect(typeof artprep.parseIdentity).toBe("function");
    expect(typeof artprep.parseShots).toBe("function");
    expect(typeof artprep.parseResults).toBe("function");
    expect(typeof artprep.composePrompt).toBe("function");
    expect(typeof artprep.buildJobs).toBe("function");
    expect(typeof artprep.jobSize).toBe("function");
    expect(typeof artprep.outputName).toBe("function");
    expect(typeof artprep.galleryHtml).toBe("function");
    expect(typeof artprep.ArtPrepError).toBe("function");
  });

  it("exports the format constants", () => {
    expect(artprep.ARTPREP_FORMAT).toBe("velvet-artprep");
    expect(artprep.ARTPREP_VERSION).toBe(1);
    expect(artprep.ARTPREP_KINDS).toEqual(["profiles", "style", "identity", "shots", "job", "results"]);
    expect(artprep.DEFAULT_SEEDS).toEqual([101, 102, 103, 104, 105, 106]);
    expect(artprep.PROMPT_LAYERS).toEqual(["style", "identity", "variation"]);
    expect(artprep.VARIATION_FIELDS).toEqual([
      "subject",
      "framing",
      "pose",
      "expression",
      "outfit",
      "setting",
      "light",
      "palette",
    ]);
  });
});
