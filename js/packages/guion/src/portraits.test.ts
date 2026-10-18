// Tests for Portraits: the mood with its synonym, the normal face as the fallback, the thinker's
// face for thoughts, and the error for a listing that is not valid.

import { describe, expect, it } from "vitest";
import { GuionError, Portraits } from "./index";

/** A listing in the format of docs/formats/GUION.md: Ana with a synonym, and Capitana. */
const LISTING = `{
  "ana": {
    "normal": { "imagen": "retratos/ana", "recorte": [0, 0, 10, 10] },
    "enfadada": { "imagen": "retratos/ana_enfadada", "recorte": [1, 1, 5, 5] },
    "enojada": "enfadada"
  },
  "capitana": { "normal": { "imagen": "retratos/capitana", "recorte": [0, 0, 8, 8] } }
}`;

describe("Portraits", () => {
  it("finds the face of a mood, following a synonym", async () => {
    const faces = await Portraits.create(LISTING);
    expect(faces.find("ana", "Enojada")).toEqual({
      image: "retratos/ana_enfadada",
      crop: [1, 1, 5, 5],
    });
  });

  it("shows the normal face for a mood that is not listed, and for no mood", async () => {
    const faces = await Portraits.create(LISTING);
    expect(faces.find("ana", "furiosa")).toEqual(faces.find("ana"));
    expect(faces.find("ana", null)?.image).toBe("retratos/ana");
  });

  it("shows the thinker's face for a thought, and null for a character with no face", async () => {
    const faces = await Portraits.create(LISTING);
    expect(faces.find("capitana_thought")?.crop).toEqual([0, 0, 8, 8]);
    expect(faces.find("teo")).toBeNull();
  });

  it("lists every face as a (who, mood) pair", async () => {
    const faces = await Portraits.create(LISTING);
    expect(faces.entries()).toHaveLength(4);
    expect(faces.entries()).toContainEqual(["capitana", "normal"]);
  });

  it("refuses a listing that is not valid, with the code bad-input", async () => {
    const error: unknown = await Portraits.create("[1]").then(
      () => undefined,
      (caught: unknown) => caught,
    );
    expect(error).toBeInstanceOf(GuionError);
    expect((error as GuionError).code).toBe("bad-input");
  });
});
