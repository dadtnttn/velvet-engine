// Tests for the music channel: the track that plays, the crossfade between two tracks, stopping, the
// change events, loop points and the start offset.

import { describe, expect, it, vi } from "vitest";
import { AudioError } from "./error";
import { chainOf, createRig, settle, thrownBy } from "./test-support";

const dayTrack = { id: "music.day", bus: "music", files: ["music/day.opus"], loop: true };
const nightTrack = { id: "music.night", bus: "music", files: ["music/night.opus"], loop: true };

describe("playMusic", () => {
  it("refuses a sound that is not on the music bus", () => {
    const rig = createRig({ sounds: [{ id: "sfx.door", files: ["sfx/door.opus"] }] });
    const error = thrownBy(() => rig.audio.playMusic("sfx.door"));
    expect(error).toBeInstanceOf(AudioError);
    expect(error instanceof AudioError ? error.code : null).toBe("wrong-bus");
  });

  it("starts the track, makes it the current music and reports the change", async () => {
    const rig = createRig({ sounds: [dayTrack] });
    const changes = vi.fn();
    rig.audio.on("music-changed", changes);
    const handle = rig.audio.playMusic("music.day");
    expect(rig.audio.currentMusic).toBe("music.day");
    expect(changes).toHaveBeenCalledWith({ id: "music.day", previous: null });
    await settle();
    expect(handle.status).toBe("playing");
    expect(rig.context.sources[0]?.loop).toBe(true);
  });

  it("does nothing when the track that is asked for already plays", async () => {
    const rig = createRig({ sounds: [dayTrack] });
    const changes = vi.fn();
    rig.audio.on("music-changed", changes);
    const first = rig.audio.playMusic("music.day");
    await settle();
    const again = rig.audio.playMusic("music.day");
    expect(again).toBe(first);
    expect(changes).toHaveBeenCalledTimes(1);
  });

  it("crossfades: the old track fades out and the new one fades in over the same time", async () => {
    const rig = createRig({ sounds: [dayTrack, nightTrack] });
    rig.audio.playMusic("music.day");
    await settle();
    const changes = vi.fn();
    rig.audio.on("music-changed", changes);
    const next = rig.audio.playMusic("music.night", { crossfadeMs: 1000 });
    await settle();

    const [day, night] = rig.context.sources;
    if (day === undefined || night === undefined) throw new Error("both tracks should have started");
    const outgoing = chainOf(day);
    expect(outgoing.gain.gain.lastRamp()).toEqual({ value: 0, time: 1 });
    expect(outgoing.source.stopAt).toBeCloseTo(1, 12);

    const incoming = chainOf(night);
    expect(incoming.gain.gain.events.slice(0, 2)).toEqual([
      { kind: "set", value: 0, time: 0 },
      { kind: "ramp", value: 1, time: 1 },
    ]);
    expect(next.status).toBe("playing");
    expect(changes).toHaveBeenCalledWith({ id: "music.night", previous: "music.day" });
    expect(rig.audio.currentMusic).toBe("music.night");
  });

  it("starts from the offset in seconds", async () => {
    const rig = createRig({ sounds: [dayTrack], files: { "audio/music/day.opus": { bytes: 4000 } } });
    rig.audio.playMusic("music.day", { startAt: 1.5 });
    await settle();
    expect(rig.context.sources[0]?.offset).toBe(1.5);
  });

  it("refuses a negative crossfade or start", () => {
    const rig = createRig({ sounds: [dayTrack] });
    expect(() => rig.audio.playMusic("music.day", { crossfadeMs: -1 })).toThrow(RangeError);
    expect(() => rig.audio.playMusic("music.day", { startAt: -1 })).toThrow(RangeError);
  });
});

describe("stopping the music", () => {
  it("fades the track out, clears the current music and reports the change", async () => {
    const rig = createRig({ sounds: [dayTrack] });
    const handle = rig.audio.playMusic("music.day");
    await settle();
    const changes = vi.fn();
    rig.audio.on("music-changed", changes);
    rig.audio.stopMusic(500);
    expect(rig.audio.currentMusic).toBeNull();
    expect(changes).toHaveBeenCalledWith({ id: null, previous: "music.day" });
    expect(chainOf(rig.context.sources[0] ?? failMissing()).gain.gain.lastRamp()).toEqual({ value: 0, time: 0.5 });
    await expect(handle.ended).resolves.toBe("stopped");
  });

  it("does nothing when no music plays", () => {
    const rig = createRig({ sounds: [dayTrack] });
    const changes = vi.fn();
    rig.audio.on("music-changed", changes);
    rig.audio.stopMusic();
    expect(changes).not.toHaveBeenCalled();
  });

  it("clears the current music when a track ends on its own", async () => {
    const rig = createRig({
      sounds: [{ id: "music.intro", bus: "music", files: ["music/intro.opus"] }],
      files: { "audio/music/intro.opus": { bytes: 2000 } },
    });
    const handle = rig.audio.playMusic("music.intro");
    await settle();
    const changes = vi.fn();
    rig.audio.on("music-changed", changes);
    rig.context.advance(3);
    expect(rig.audio.currentMusic).toBeNull();
    expect(changes).toHaveBeenCalledWith({ id: null, previous: "music.intro" });
    await expect(handle.ended).resolves.toBe("finished");
  });

  it("stopAll stops the music and every sound with it", async () => {
    const rig = createRig({ sounds: [dayTrack, { id: "sfx.door", files: ["sfx/door.opus"] }] });
    const music = rig.audio.playMusic("music.day");
    const door = rig.audio.play("sfx.door");
    await settle();
    rig.audio.stopAll();
    expect(rig.audio.currentMusic).toBeNull();
    await expect(music.ended).resolves.toBe("stopped");
    await expect(door.ended).resolves.toBe("stopped");
  });
});

describe("loop points", () => {
  it("are applied to the source, within the length of the file", async () => {
    const rig = createRig({
      sounds: [{ ...dayTrack, loop_points: { start: 1.5, end: 9 } }],
      files: { "audio/music/day.opus": { bytes: 2000 } },
    });
    rig.audio.playMusic("music.day");
    await settle();
    const [source] = rig.context.sources;
    expect(source?.loopStart).toBe(1.5);
    expect(source?.loopEnd).toBe(2);
  });

  it("are ignored when they fall outside the file, so the whole file repeats", async () => {
    const rig = createRig({
      sounds: [{ ...dayTrack, loop_points: { start: 3, end: 4 } }],
      files: { "audio/music/day.opus": { bytes: 2000 } },
    });
    rig.audio.playMusic("music.day");
    await settle();
    const [source] = rig.context.sources;
    expect(source?.loopStart).toBe(0);
    expect(source?.loopEnd).toBe(0);
  });
});

/** Stands in for a source that a test expects to exist. */
function failMissing(): never {
  throw new Error("expected a started source");
}
