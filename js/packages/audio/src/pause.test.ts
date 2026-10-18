// Tests for following the game clock's pause and the page's visibility. A paused clock mutes the game
// buses (or suspends the whole context, by option). A hidden page suspends the context. Plays that
// cannot start yet wait in a queue, which keeps only the newest plays and drops the ones that are stale.

import { describe, expect, it } from "vitest";
import { chainOf, createRig, settle } from "./test-support";

const track = { id: "music.day", bus: "music", files: ["music/day.opus"], loop: true };
const click = { id: "ui.click", bus: "ui", files: ["ui/click.opus"] };
const door = { id: "sfx.door", files: ["sfx/door.opus"] };

describe("the game clock's pause", () => {
  it("mutes the game buses over the ramp and leaves the ui bus sounding", async () => {
    // Each file has its own length, so the source of each sound can be found by its duration.
    const rig = createRig({
      sounds: [track, click, door],
      files: { "audio/music/day.opus": { bytes: 2000 }, "audio/ui/click.opus": { bytes: 1000 }, "audio/sfx/door.opus": { bytes: 3000 } },
    });
    rig.audio.playMusic("music.day");
    rig.audio.play("ui.click");
    rig.audio.play("sfx.door");
    await settle();
    const music = rig.context.sources.find((source) => source.buffer?.duration === 2);
    const ui = rig.context.sources.find((source) => source.buffer?.duration === 1);
    const sfx = rig.context.sources.find((source) => source.buffer?.duration === 3);
    if (music === undefined || ui === undefined || sfx === undefined) throw new Error("three sources should play");

    rig.clock.paused = true;
    rig.audio.update();
    expect(chainOf(music).input.gain.lastRamp()).toEqual({ value: 0, time: 0.02 });
    expect(chainOf(sfx).input.gain.lastRamp()).toEqual({ value: 0, time: 0.02 });
    expect(chainOf(ui).input.gain.lastRamp()).toBeNull();
  });

  it("brings the game buses back on resume", async () => {
    const rig = createRig({ sounds: [track] });
    rig.audio.playMusic("music.day");
    await settle();
    rig.clock.paused = true;
    rig.audio.update();
    rig.clock.paused = false;
    rig.audio.update();
    const [music] = rig.context.sources;
    if (music === undefined) throw new Error("the track should play");
    expect(chainOf(music).input.gain.lastRamp()).toEqual({ value: 1, time: 0.02 });
  });

  it("changes nothing on an update when the pause has not changed", async () => {
    const rig = createRig({ sounds: [track] });
    rig.audio.playMusic("music.day");
    await settle();
    rig.clock.paused = true;
    rig.audio.update();
    const [music] = rig.context.sources;
    if (music === undefined) throw new Error("the track should play");
    const before = chainOf(music).input.gain.events.length;
    rig.audio.update();
    rig.audio.update();
    expect(chainOf(music).input.gain.events.length).toBe(before);
  });

  it("suspends the whole context with the context policy, and starts it again on resume", async () => {
    const rig = createRig({ sounds: [track, click], pauseWith: "context" });
    rig.audio.playMusic("music.day");
    await settle();
    rig.clock.paused = true;
    rig.audio.update();
    expect(rig.context.state).toBe("suspended");
    rig.clock.paused = false;
    rig.audio.update();
    expect(rig.context.state).toBe("running");
  });

  it("holds the plays of a paused context until it runs again", async () => {
    const rig = createRig({ sounds: [click], pauseWith: "context" });
    rig.audio.play("ui.click");
    await settle();
    rig.clock.paused = true;
    rig.audio.update();
    const waiting = rig.audio.play("ui.click");
    await settle();
    expect(waiting.status).toBe("pending");
    rig.clock.paused = false;
    rig.audio.update();
    await settle();
    expect(waiting.status).toBe("playing");
  });
});

describe("the page's visibility", () => {
  it("suspends the context while the page is hidden, and resumes it when it is shown", async () => {
    const rig = createRig({ sounds: [track] });
    rig.audio.playMusic("music.day");
    await settle();
    rig.host.setHidden(true);
    expect(rig.context.state).toBe("suspended");
    rig.host.setHidden(false);
    expect(rig.context.state).toBe("running");
  });

  it("leaves the context running when suspendWhenHidden is false", async () => {
    const rig = createRig({ sounds: [track], suspendWhenHidden: false });
    rig.audio.playMusic("music.day");
    await settle();
    rig.host.setHidden(true);
    expect(rig.context.state).toBe("running");
  });
});

describe("the queue of plays that wait", () => {
  it("keeps only the newest plays when the queue is full", async () => {
    const rig = createRig({ sounds: [door], running: false, queueLimit: 2 });
    const first = rig.audio.play("sfx.door");
    const second = rig.audio.play("sfx.door");
    const third = rig.audio.play("sfx.door");
    expect(first.status).toBe("ended");
    await expect(first.ended).resolves.toBe("skipped");
    expect(second.status).toBe("pending");
    expect(third.status).toBe("pending");
  });

  it("skips a play that waited longer than the stale time", async () => {
    const rig = createRig({ sounds: [door], running: false, staleAfterMs: 1000 });
    const old = rig.audio.play("sfx.door");
    rig.time.ms = 1500;
    const fresh = rig.audio.play("sfx.door");
    rig.context.state = "running";
    rig.audio.update();
    await settle();
    await expect(old.ended).resolves.toBe("skipped");
    expect(fresh.status).toBe("playing");
  });

  it("keeps only the newest track that waits for the audio", async () => {
    const day = { id: "music.day", bus: "music", files: ["music/day.opus"], loop: true };
    const night = { id: "music.night", bus: "music", files: ["music/night.opus"], loop: true };
    const rig = createRig({ sounds: [day, night], running: false });
    const first = rig.audio.playMusic("music.day");
    const second = rig.audio.playMusic("music.night");
    await expect(first.ended).resolves.toBe("stopped");
    rig.context.state = "running";
    rig.audio.update();
    await settle();
    expect(second.status).toBe("playing");
    expect(rig.context.sources).toHaveLength(1);
    expect(rig.audio.currentMusic).toBe("music.night");
  });
});
