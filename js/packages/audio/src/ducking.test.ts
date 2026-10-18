// Tests for ducking: while a voice plays, the music is turned down, and it comes back up after the
// last voice ends. The attack and the release are ramps of the music bus's duck.

import { describe, expect, it } from "vitest";
import { AudioError } from "./error";
import { decibelsToGain } from "./volume";
import { chainOf, createRig, settle, thrownBy } from "./test-support";

const track = { id: "music.day", bus: "music", files: ["music/day.opus"], loop: true };
const hello = { id: "voice.hello", bus: "voice", files: ["voice/hello.opus"] };

/** The source of the music track, after the track has started. */
async function startMusic(
  sounds: readonly unknown[],
  ducking?: false | undefined,
  files?: Readonly<Record<string, { bytes: number }>>,
) {
  const rig = createRig({ sounds, ducking, files });
  rig.audio.playMusic("music.day");
  await settle();
  const music = rig.context.sources[0];
  if (music === undefined) throw new Error("the music should have started");
  return { rig, music };
}

describe("ducking under a voice", () => {
  it("turns the music down over the attack when a voice starts", async () => {
    const { rig, music } = await startMusic([track, hello]);
    rig.audio.play("voice.hello");
    await settle();
    const duck = chainOf(music).duck.gain;
    expect(duck.lastRamp()).toEqual({ value: decibelsToGain(-12), time: 0.06 });
  });

  it("brings the music back up over the release when the last voice ends", async () => {
    const { rig, music } = await startMusic([track, hello]);
    rig.audio.play("voice.hello");
    await settle();
    rig.context.advance(1);
    const duck = chainOf(music).duck.gain;
    expect(duck.lastRamp()).toEqual({ value: 1, time: 1.4 });
  });

  it("waits for every voice before it releases", async () => {
    // The short line lasts one second and the long one two, so the first to end is not the last.
    const { rig, music } = await startMusic(
      [
        track,
        { id: "voice.short", bus: "voice", files: ["voice/short.opus"] },
        { id: "voice.long", bus: "voice", files: ["voice/long.opus"] },
      ],
      undefined,
      { "audio/music/day.opus": { bytes: 1000 }, "audio/voice/short.opus": { bytes: 1000 }, "audio/voice/long.opus": { bytes: 2000 } },
    );
    const duck = chainOf(music).duck.gain;
    rig.audio.play("voice.short");
    await settle();
    expect(duck.events.filter((event) => event.kind === "ramp")).toHaveLength(1);

    rig.context.advance(0.5);
    rig.audio.play("voice.long");
    await settle();
    expect(duck.events.filter((event) => event.kind === "ramp")).toHaveLength(1);

    rig.context.advance(0.5);
    expect(duck.lastRamp()?.value).toBe(decibelsToGain(-12));
    rig.context.advance(1.5);
    expect(duck.lastRamp()).toEqual({ value: 1, time: 2.9 });
  });

  it("is off when the ducking is set to false", async () => {
    const { rig, music } = await startMusic([track, hello], false);
    rig.audio.play("voice.hello");
    await settle();
    expect(chainOf(music).duck.gain.lastRamp()).toBeNull();
  });

  it("does not touch the music for a sound on another bus", async () => {
    const { rig, music } = await startMusic([track, { id: "sfx.door", files: ["sfx/door.opus"] }]);
    rig.audio.play("sfx.door");
    await settle();
    expect(chainOf(music).duck.gain.lastRamp()).toBeNull();
  });
});

describe("the ducking settings", () => {
  it("take their own trigger, target, depth and times", async () => {
    const ambience = { id: "ambience.wind", bus: "ambience", files: ["ambience/wind.opus"], loop: true };
    const door = { id: "sfx.door", files: ["sfx/door.opus"] };
    const rig = createRig({
      sounds: [ambience, door],
      buses: ["ambience"],
      ducking: { trigger: "sfx", target: "ambience", decibels: -6, attackMs: 10, releaseMs: 100 },
    });
    rig.audio.play("ambience.wind");
    await settle();
    const wind = rig.context.sources[0];
    if (wind === undefined) throw new Error("the ambience should have started");
    rig.audio.play("sfx.door");
    await settle();
    const duck = chainOf(wind).duck.gain;
    expect(duck.lastRamp()).toEqual({ value: decibelsToGain(-6), time: 0.01 });
  });

  it("are refused when a bus does not exist, or the depth is above zero", () => {
    expect(thrownBy(() => createRig({ sounds: [], ducking: { trigger: "nope" } }))).toBeInstanceOf(AudioError);
    expect(() => createRig({ sounds: [], ducking: { decibels: 3 } })).toThrow(RangeError);
    expect(() => createRig({ sounds: [], ducking: { trigger: "music", target: "music" } })).toThrow(RangeError);
  });
});
