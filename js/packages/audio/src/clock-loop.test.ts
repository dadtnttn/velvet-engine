// Tests for the engine's own following of the game clock and the game loop: a GameClock's pause events
// gate the music at once, a plain clock is read through the loop's frame hook, a clock that is paused when
// the engine starts mutes it, the context policy suspends and resumes the context, update() stays an
// optional call, and close releases the subscriptions and saves the mix. The clock is a real GameClock.

import { GameClock, GameLoop } from "@velvet/core";
import { describe, expect, it, vi } from "vitest";
import type { AudioClock, AudioLoop, AudioSettingsStore } from "./api";
import { createAudio } from "./engine";
import { FakeAudioContext, FakeHost, ManualFrames, chainOf, createFakeNetwork, createMemorySettings, filesFor, manifestOf, settle } from "./test-support";

const track = { id: "music.day", bus: "music", files: ["music/day.opus"], loop: true };
const door = { id: "sfx.door", files: ["sfx/door.opus"] };
const manifest = manifestOf([track, door]);

interface BuildOptions {
  readonly clock?: AudioClock;
  readonly loop?: AudioLoop;
  readonly pauseWith?: "game-buses" | "context";
  readonly running?: boolean;
  readonly settings?: AudioSettingsStore | null;
}

/** An engine on a fake context, with a fake network and a fake page. Settings are kept for the session by default. */
function build(options: BuildOptions = {}) {
  const context = new FakeAudioContext();
  context.state = options.running === false ? "suspended" : "running";
  const network = createFakeNetwork(filesFor(manifest));
  const host = new FakeHost();
  const audio = createAudio({
    manifest,
    baseUrl: "audio/",
    fetch: network.fetchFile,
    createContext: () => context,
    canPlay: () => true,
    host,
    clock: options.clock,
    loop: options.loop,
    pauseWith: options.pauseWith,
    settings: options.settings === undefined ? null : options.settings,
  });
  return { audio, context, host };
}

/** The first source the context started, which must exist once the test has played a sound. */
function firstSource(context: FakeAudioContext) {
  const source = context.sources[0];
  if (source === undefined) throw new Error("a source should have started");
  return source;
}

describe("a GameClock's pause events", () => {
  it("gates the game buses as soon as the clock pauses and opens them on resume, with no update call", async () => {
    const clock = new GameClock();
    const { audio, context } = build({ clock });
    audio.playMusic("music.day");
    await settle();
    const music = chainOf(firstSource(context)).input.gain;

    clock.pause();
    expect(music.lastRamp()).toEqual({ value: 0, time: 0.02 });
    clock.resume();
    expect(music.lastRamp()).toEqual({ value: 1, time: 0.02 });
  });

  it("suspends the context on pause and starts it on resume with the context policy, with no update call", async () => {
    const clock = new GameClock();
    const { audio, context } = build({ clock, pauseWith: "context" });
    audio.playMusic("music.day");
    await settle();
    clock.pause();
    expect(context.state).toBe("suspended");
    clock.resume();
    expect(context.state).toBe("running");
  });

  it("mutes the game buses from the start when the clock is already paused as the engine is made", async () => {
    const clock = new GameClock();
    clock.pause();
    const { audio, context } = build({ clock });
    audio.playMusic("music.day");
    await settle();
    const music = chainOf(firstSource(context)).input.gain;
    expect(music.value).toBe(0);
  });

  it("leaves update as an optional call: calling it after the events changes nothing", async () => {
    const clock = new GameClock();
    const { audio, context } = build({ clock });
    audio.playMusic("music.day");
    await settle();
    clock.pause();
    const music = chainOf(firstSource(context)).input.gain;
    const before = music.events.length;
    audio.update();
    audio.update();
    expect(music.events.length).toBe(before);
  });
});

describe("a plain clock and the loop's frame hook", () => {
  it("is read through the loop's beforeFrame hook, so the game needs no call to update", async () => {
    const plain = { paused: false };
    const frames = new ManualFrames();
    const loop = new GameLoop({ scheduler: frames, update: () => undefined });
    const { audio, context } = build({ clock: plain, loop });
    loop.start();
    frames.runFrame(0);
    audio.playMusic("music.day");
    await settle();
    const music = chainOf(firstSource(context)).input.gain;

    plain.paused = true;
    frames.runFrame(100);
    expect(music.lastRamp()).toEqual({ value: 0, time: 0.02 });
  });

  it("starts the plays that waited for the browser, once the context runs, with no update call", async () => {
    const frames = new ManualFrames();
    const loop = new GameLoop({ scheduler: frames, update: () => undefined });
    const { audio, context } = build({ loop, running: false });
    const handle = audio.play("sfx.door");
    await settle();
    expect(handle.status).toBe("pending");

    context.state = "running";
    loop.start();
    frames.runFrame(0);
    await settle();
    expect(handle.status).toBe("playing");
  });

  it("stops checking the audio every frame after close, and releases the clock's events", async () => {
    const plain = { paused: false };
    const frames = new ManualFrames();
    const loop = new GameLoop({ scheduler: frames, update: () => undefined });
    const { audio } = build({ clock: plain, loop });
    const update = vi.spyOn(audio, "update");
    loop.start();
    frames.runFrame(0);
    expect(update).toHaveBeenCalled();

    await audio.close();
    update.mockClear();
    frames.runFrame(100);
    expect(update).not.toHaveBeenCalled();
  });
});

describe("close and the saved mix", () => {
  it("saves a volume that was changed in the last moment before close", async () => {
    const memory = createMemorySettings();
    const { audio } = build({ settings: memory.store });
    audio.setVolume("sfx", 0.3);
    await audio.close();
    expect(memory.values.get("audio")).toMatchObject({ values: { volumes: { sfx: 0.3 } } });
  });
});
