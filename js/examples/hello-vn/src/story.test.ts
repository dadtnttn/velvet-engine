// Tests of the story: the two routes to the two endings, the closed option, the stage directives, the record that
// a save keeps and the replay that restores it, the language switch in the middle of a scene, and the hooks.

import { beforeAll, describe, expect, it } from "vitest";
import { GuionError, type Shown, type ShownChoice } from "@velvet/guion";
import { Story, NO_VARS, type StorySnapshot } from "./story";
import { storyFixture, type StoryFixture } from "./test-support";

let fixture: StoryFixture;

beforeAll(async () => {
  fixture = await storyFixture();
});

/** A new story in a language, not started. */
function newStory(language: "es" | "en" = "es", hooks = {}): Story {
  return new Story(fixture.library, fixture.fileIds, language, hooks);
}

/** Goes on past lines until the box on screen is a choice. Fails when the story ends first. */
function untilChoice(story: Story): ShownChoice {
  for (let guard = 0; guard < 200; guard += 1) {
    const box = story.current();
    if (box.type === "choice") return box;
    if (box.type === "end") throw new Error("the story ended before a choice");
    story.next();
  }
  throw new Error("no choice within 200 boxes");
}

/** Goes on until the story ends. Each choice on the way takes the next index of `picks`. */
function toTheEnd(story: Story, picks: readonly number[]): void {
  let pick = 0;
  for (let guard = 0; guard < 200; guard += 1) {
    const box: Shown = story.current();
    if (box.type === "end") return;
    if (box.type === "choice") story.choose(picks[pick++] ?? 0);
    else story.next();
  }
  throw new Error("the story did not end within 200 boxes");
}

describe("the two routes to the two endings", () => {
  it("reaches the light through the tavern, where the sea glass is taken", () => {
    const story = newStory();
    story.begin("puerto");
    const start = untilChoice(story);
    expect(start.options.map((option) => option.label)).toHaveLength(2);
    story.choose(1); // to the tavern
    toTheEnd(story, [0]); // the lamp, with the glass
    expect(story.finished).toBe(true);
    expect(story.endingNumber).toBe(1);
    expect(story.vars.values["lente"]).toBe(1);
  });

  it("reaches the bell by the direct way, where the lamp option stays closed", () => {
    const story = newStory();
    story.begin("puerto");
    untilChoice(story);
    story.choose(0); // straight to the lighthouse
    const choice = untilChoice(story);
    expect(choice.options[0]?.available).toBe(false);
    expect(choice.options[1]?.available).toBe(true);
    const before = story.snapshot();
    expect(() => story.choose(0)).toThrow(GuionError);
    expect(story.snapshot()).toEqual(before);
    story.choose(1); // the bell
    expect(story.finished).toBe(true);
    expect(story.endingNumber).toBe(2);
    expect(story.vars.values["lente"]).toBeUndefined();
  });
});

describe("the stage", () => {
  it("sets the background and the music from the directives, as the boxes are passed", () => {
    const story = newStory();
    story.begin("puerto");
    expect(story.stage).toEqual({ background: "bg.puerto", music: "music.pad" });
    expect(story.scene).toBe("puerto");
    untilChoice(story);
    story.choose(1);
    expect(story.scene).toBe("taberna");
    expect(story.stage).toEqual({ background: "bg.taberna", music: "music.pad" });
    untilChoice(story);
    story.choose(0);
    expect(story.scene).toBe("faro");
    expect(story.stage).toEqual({ background: "bg.faro", music: "music.pad" });
  });
});

describe("the record of a story", () => {
  it("restores the same box, variables and stage from the record of the actions", () => {
    const first = newStory();
    first.begin("puerto");
    untilChoice(first);
    first.choose(1);
    untilChoice(first);
    const record: StorySnapshot = first.snapshot();
    expect(record.scene).toBe("puerto");
    expect(record.steps.length).toBeGreaterThan(0);

    const second = newStory();
    second.restore(record);
    expect(second.current()).toEqual(first.current());
    expect(second.vars).toEqual(first.vars);
    expect(second.stage).toEqual(first.stage);
    expect(second.scene).toBe(first.scene);
    expect(second.snapshot()).toEqual(record);
  });

  it("keeps the story as it was when a record does not fit the guion", () => {
    const story = newStory();
    story.begin("puerto");
    untilChoice(story);
    const before = story.snapshot();
    expect(() => story.restore({ scene: "puerto", vars: NO_VARS, steps: [{ kind: "choose", index: 9 }] })).toThrow(
      GuionError,
    );
    expect(story.snapshot()).toEqual(before);
  });

  it("refuses to give a record before a scene has started", () => {
    expect(() => newStory().snapshot()).toThrow("has not started");
  });
});

describe("the language", () => {
  it("switches in the middle of a scene, and keeps the box and the choices", () => {
    const story = newStory("es");
    story.begin("puerto");
    untilChoice(story);
    story.choose(0);
    const spanish = untilChoice(story);
    story.switchLanguage("en");
    expect(story.lang).toBe("en");
    const english = story.current();
    expect(english.type).toBe("choice");
    if (english.type !== "choice") throw new Error("the box is not a choice");
    expect(english.prompt).toBe("How do we warn the boats?");
    expect(english.options.map((option) => option.available)).toEqual(spanish.options.map((option) => option.available));
    story.choose(1);
    expect(story.endingNumber).toBe(2);
  });

  it("keeps the language for the next start when no scene has started", () => {
    const story = newStory("es");
    story.switchLanguage("en");
    story.begin("puerto");
    const box = story.current();
    expect(box.type === "line" && box.text.startsWith("The streetlamps")).toBe(true);
  });
});

describe("the hooks", () => {
  it("run for the player's own actions, and not for a restore", () => {
    const scenes: string[] = [];
    const choices: number[] = [];
    const story = newStory("es", {
      onScene: (scene: string) => scenes.push(scene),
      onChoice: (index: number) => choices.push(index),
    });
    story.begin("puerto");
    expect(scenes).toEqual(["puerto"]);
    untilChoice(story);
    story.choose(1);
    expect(choices).toEqual([1]);
    expect(scenes).toEqual(["puerto", "taberna"]);
    toTheEnd(story, [0]);
    expect(scenes).toEqual(["puerto", "taberna", "faro"]);
    expect(choices).toEqual([1, 0]);

    const record = story.snapshot();
    const replayed = newStory("es", {
      onScene: (scene: string) => scenes.push(scene),
      onChoice: (index: number) => choices.push(index),
    });
    replayed.restore(record);
    expect(scenes).toEqual(["puerto", "taberna", "faro"]);
    expect(choices).toEqual([1, 0]);
    expect(replayed.endingNumber).toBe(1);
  });
});

