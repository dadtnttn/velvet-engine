// Tests of the save: the record of a story goes through the save store and comes back to the same box, and a
// save document that is not one of this game's is refused.

import { beforeAll, describe, expect, it } from "vitest";
import { createMemoryBackend, createSaveStore } from "@velvet/core";
import { GAME_ID, SAVE_VERSION } from "./config";
import { buildSave, parseSave } from "./save-data";
import { Story } from "./story";
import { storyFixture, type StoryFixture } from "./test-support";

let fixture: StoryFixture;

beforeAll(async () => {
  fixture = await storyFixture();
});

/** A story played into the tavern and on to the lighthouse's choice: a record in the middle of the play. */
function playedStory(): Story {
  const story = new Story(fixture.library, fixture.fileIds, "es");
  story.begin("puerto");
  for (let guard = 0; guard < 50 && story.current().type !== "choice"; guard += 1) story.next();
  story.choose(1);
  for (let guard = 0; guard < 50 && story.current().type !== "choice"; guard += 1) story.next();
  return story;
}

describe("a save of the story", () => {
  it("goes through the save store and comes back to the same box, variables and stage", async () => {
    const store = createSaveStore({ game: GAME_ID, version: SAVE_VERSION, backend: createMemoryBackend() });
    const played = playedStory();
    await store.save("slot-1", buildSave(played.snapshot()));

    const loaded = await store.load("slot-1");
    expect(loaded).not.toBeNull();
    const save = parseSave(loaded?.data);
    expect(save).not.toBeNull();

    const restored = new Story(fixture.library, fixture.fileIds, "es");
    restored.restore(save?.story ?? played.snapshot());
    expect(restored.current()).toEqual(played.current());
    expect(restored.vars).toEqual(played.vars);
    expect(restored.stage).toEqual(played.stage);
    expect(restored.scene).toBe("faro");
  });

  it("lets the restored story go on to an ending", async () => {
    const played = playedStory();
    const save = parseSave(buildSave(played.snapshot()));
    const restored = new Story(fixture.library, fixture.fileIds, "es");
    restored.restore(save?.story ?? played.snapshot());
    for (let guard = 0; guard < 50 && restored.current().type !== "choice"; guard += 1) restored.next();
    restored.choose(0);
    expect(restored.endingNumber).toBe(1);
  });

  it("reads a save in the other language at the same box", () => {
    const played = playedStory();
    const save = parseSave(buildSave(played.snapshot()));
    const english = new Story(fixture.library, fixture.fileIds, "en");
    english.restore(save?.story ?? played.snapshot());
    const box = english.current();
    expect(box.type).toBe("choice");
    expect(box.type === "choice" && box.prompt).toBe("How do we warn the boats?");
  });
});

describe("reading a save", () => {
  const valid = {
    story: {
      scene: "puerto",
      vars: { values: { lente: 1 }, seen: ["faro.puerto"] },
      steps: [{ kind: "next" }, { kind: "choose", index: 1 }],
    },
  };

  it("accepts the document the game writes", () => {
    expect(parseSave(valid)).toEqual(valid);
  });

  it("refuses what is not a save of this game", () => {
    expect(parseSave(null)).toBeNull();
    expect(parseSave([])).toBeNull();
    expect(parseSave({})).toBeNull();
    expect(parseSave({ story: { ...valid.story, scene: "Puerto Mayor" } })).toBeNull();
    expect(parseSave({ story: { ...valid.story, steps: [{ kind: "jump" }] } })).toBeNull();
    expect(parseSave({ story: { ...valid.story, steps: [{ kind: "choose", index: -1 }] } })).toBeNull();
    expect(parseSave({ story: { ...valid.story, steps: [{ kind: "choose", index: 1.5 }] } })).toBeNull();
    expect(parseSave({ story: { ...valid.story, vars: { values: { lente: 0.5 }, seen: [] } } })).toBeNull();
    expect(parseSave({ story: { ...valid.story, vars: { values: {}, seen: [3] } } })).toBeNull();
  });
});
