// Tests for playing sounds: the bus graph, the volumes and the options of a play, the handles, and
// the events of a load that fails or succeeds.

import { describe, expect, it, vi } from "vitest";
import { AudioError } from "./error";
import { volumeToGain } from "./volume";
import { FakePanner, chainOf, createRig, settle, thrownBy } from "./test-support";

const door = { id: "sfx.door", files: ["sfx/door.opus", "sfx/door.wav"] };

describe("the audio context", () => {
  it("is made on the first sound that is needed, not when the engine is created", async () => {
    const created = vi.fn(() => null);
    const rig = createRig({ sounds: [door], createContext: created });
    expect(created).not.toHaveBeenCalled();
    rig.audio.play("sfx.door");
    expect(created).toHaveBeenCalledTimes(1);
    await settle();
  });
});

describe("the bus graph", () => {
  it("routes a play through its gain, the bus input and the duck to the master", async () => {
    const rig = createRig({ sounds: [door] });
    const handle = rig.audio.play("sfx.door");
    expect(handle.status).toBe("pending");
    await settle();
    expect(handle.status).toBe("playing");

    const [source] = rig.context.sources;
    expect(source).toBeDefined();
    if (source === undefined) return;
    const chain = chainOf(source);
    expect(chain.master.outputs).toContain(rig.context.destination);
    expect(chain.input.gain.value).toBeCloseTo(1, 12);
    expect(chain.duck.gain.value).toBe(1);
    expect(source.loop).toBe(false);
    expect(source.buffer?.duration).toBe(1);
  });

  it("makes no panner unless a pan is given", async () => {
    const rig = createRig({ sounds: [door] });
    rig.audio.play("sfx.door");
    await settle();
    expect(rig.context.panners).toHaveLength(0);
  });
});

describe("the volume of a play", () => {
  it("multiplies the sound's volume and the play's volume on the dB curve", async () => {
    const rig = createRig({ sounds: [{ id: "sfx.door", files: ["sfx/door.opus"], volume: 0.5 }] });
    rig.audio.play("sfx.door", { volume: 0.5 });
    await settle();
    const [source] = rig.context.sources;
    if (source === undefined) throw new Error("the play should have started");
    const gain = chainOf(source).gain.gain.lastSet();
    expect(gain?.value).toBeCloseTo(volumeToGain(0.5) * volumeToGain(0.5), 12);
    expect(gain?.time).toBe(0);
  });

  it("applies rate, detune and pan to the nodes", async () => {
    const rig = createRig({ sounds: [door] });
    rig.audio.play("sfx.door", { rate: 2, detune: -50, pan: -0.5 });
    await settle();
    const [source] = rig.context.sources;
    if (source === undefined) throw new Error("the play should have started");
    expect(source.playbackRate.value).toBe(2);
    expect(source.detune.value).toBe(-50);
    const panner = chainOf(source).gain.outputs[0];
    expect(panner).toBeInstanceOf(FakePanner);
    if (panner instanceof FakePanner) expect(panner.pan.value).toBe(-0.5);
  });

  it("starts after the delay, on the context's clock", async () => {
    const rig = createRig({ sounds: [door] });
    rig.audio.play("sfx.door", { delay: 250 });
    await settle();
    expect(rig.context.sources[0]?.startedAt).toBeCloseTo(0.25, 12);
  });

  it("lets the play decide whether it loops, whatever the sound says", async () => {
    const rig = createRig({ sounds: [door, { id: "sfx.hum", files: ["sfx/hum.opus"], loop: true }] });
    rig.audio.play("sfx.door", { loop: true });
    rig.audio.play("sfx.hum", { loop: false });
    await settle();
    expect(rig.context.sources.map((source) => source.loop)).toEqual([true, false]);
  });

  it("refuses options out of range", () => {
    const rig = createRig({ sounds: [door] });
    expect(() => rig.audio.play("sfx.door", { volume: 2 })).toThrow(RangeError);
    expect(() => rig.audio.play("sfx.door", { rate: 0 })).toThrow(RangeError);
    expect(() => rig.audio.play("sfx.door", { pan: 2 })).toThrow(RangeError);
    expect(() => rig.audio.play("sfx.door", { delay: -1 })).toThrow(RangeError);
    expect(() => rig.audio.play("sfx.door", { detune: Number.NaN })).toThrow(RangeError);
  });

  it("refuses an id that the manifest does not have", () => {
    const rig = createRig({ sounds: [door] });
    const error = thrownBy(() => rig.audio.play("sfx.nope"));
    expect(error).toBeInstanceOf(AudioError);
    expect(error instanceof AudioError ? error.code : null).toBe("unknown-id");
  });
});

describe("the handle", () => {
  it("ends with 'finished' when the sound plays to its end", async () => {
    const rig = createRig({ sounds: [door] });
    const handle = rig.audio.play("sfx.door");
    await settle();
    rig.context.advance(1.5);
    expect(handle.status).toBe("ended");
    await expect(handle.ended).resolves.toBe("finished");
  });

  it("stop with a fade ramps the gain to silence and stops the source when the fade is over", async () => {
    const rig = createRig({ sounds: [door] });
    const handle = rig.audio.play("sfx.door");
    await settle();
    handle.stop(200);
    expect(handle.status).toBe("ended");
    await expect(handle.ended).resolves.toBe("stopped");
    const chain = chainOf(rig.context.sources[0] ?? thrownAway());
    expect(chain.gain.gain.lastRamp()).toEqual({ value: 0, time: 0.2 });
    expect(chain.source.stopAt).toBeCloseTo(0.2, 12);
  });

  it("stop on a pending play ends it, and it never starts", async () => {
    const rig = createRig({ sounds: [door] });
    const handle = rig.audio.play("sfx.door");
    handle.stop();
    await settle();
    expect(handle.status).toBe("ended");
    await expect(handle.ended).resolves.toBe("stopped");
    expect(rig.context.sources).toHaveLength(0);
  });

  it("fade ramps the play's volume and keeps the new value", async () => {
    const rig = createRig({ sounds: [door] });
    const handle = rig.audio.play("sfx.door");
    await settle();
    handle.fade(0.5, 100);
    expect(handle.volume).toBe(0.5);
    const chain = chainOf(rig.context.sources[0] ?? thrownAway());
    expect(chain.gain.gain.lastRamp()?.value).toBeCloseTo(volumeToGain(0.5), 12);
    expect(chain.gain.gain.lastRamp()?.time).toBeCloseTo(0.1, 12);
  });

  it("does nothing when stopped or faded after it has ended", async () => {
    const rig = createRig({ sounds: [door] });
    const handle = rig.audio.play("sfx.door");
    await settle();
    rig.context.advance(2);
    await handle.ended;
    expect(() => handle.stop(100)).not.toThrow();
    expect(() => handle.fade(0.1, 10)).not.toThrow();
    expect(handle.status).toBe("ended");
  });

  it("refuses a fade with a volume out of range or a negative time", async () => {
    const rig = createRig({ sounds: [door] });
    const handle = rig.audio.play("sfx.door");
    await settle();
    expect(() => handle.fade(1.5, 10)).toThrow(RangeError);
    expect(() => handle.fade(0.5, -10)).toThrow(RangeError);
    expect(() => handle.stop(-1)).toThrow(RangeError);
  });
});

describe("loading in the play path", () => {
  it("reports a loaded variant through the event system", async () => {
    const rig = createRig({ sounds: [door] });
    const listener = vi.fn();
    rig.audio.on("loaded", listener);
    rig.audio.play("sfx.door");
    await settle();
    expect(listener).toHaveBeenCalledTimes(1);
    expect(listener).toHaveBeenCalledWith({ id: "sfx.door", variant: 0 });
  });

  it("skips a candidate that is missing and uses the next one", async () => {
    const rig = createRig({ sounds: [door], files: { "audio/sfx/door.wav": { bytes: 2000 } } });
    rig.audio.play("sfx.door");
    await settle();
    expect(rig.network.requests).toEqual(["audio/sfx/door.opus", "audio/sfx/door.wav"]);
    expect(rig.context.sources[0]?.buffer?.duration).toBe(2);
  });

  it("skips a candidate that cannot be decoded and uses the next one", async () => {
    const rig = createRig({
      sounds: [door],
      files: { "audio/sfx/door.opus": { bytes: 1000, decodeFails: true }, "audio/sfx/door.wav": { bytes: 3000 } },
    });
    rig.audio.play("sfx.door");
    await settle();
    expect(rig.context.sources[0]?.buffer?.duration).toBe(3);
  });

  it("does not fetch a format that the runtime cannot play", async () => {
    const rig = createRig({ sounds: [door], canPlay: (format) => format !== "opus" });
    rig.audio.play("sfx.door");
    await settle();
    expect(rig.network.requests).toEqual(["audio/sfx/door.wav"]);
  });

  it("reports an error and ends the play as failed when no candidate loads", async () => {
    const rig = createRig({ sounds: [door], files: {} });
    const listener = vi.fn();
    rig.audio.on("error", listener);
    const handle = rig.audio.play("sfx.door");
    await settle();
    expect(handle.status).toBe("ended");
    await expect(handle.ended).resolves.toBe("failed");
    expect(listener).toHaveBeenCalledTimes(1);
    const [payload] = listener.mock.calls[0] ?? [];
    expect(payload).toMatchObject({ id: "sfx.door" });
    expect(String((payload as { message?: unknown }).message)).toContain("could not be loaded");
    expect(rig.context.sources).toHaveLength(0);
  });

  it("fails the play at once when the runtime has no audio", async () => {
    const rig = createRig({ sounds: [door], createContext: () => null });
    const listener = vi.fn();
    rig.audio.on("error", listener);
    const handle = rig.audio.play("sfx.door");
    expect(handle.status).toBe("ended");
    await expect(handle.ended).resolves.toBe("failed");
    expect(listener).toHaveBeenCalledWith({ id: "sfx.door", message: "audio is not available in this runtime" });
  });

  it("says why when the context factory throws", async () => {
    const rig = createRig({
      sounds: [door],
      createContext: () => {
        throw new Error("no sound device");
      },
    });
    const listener = vi.fn();
    rig.audio.on("error", listener);
    rig.audio.play("sfx.door");
    expect(listener.mock.calls[0]?.[0]).toMatchObject({ message: expect.stringContaining("no sound device") });
  });

  it("reports whether a sound is loaded, and is false for an unknown id", async () => {
    const rig = createRig({ sounds: [door] });
    expect(rig.audio.isLoaded("sfx.door")).toBe(false);
    rig.audio.play("sfx.door");
    await settle();
    expect(rig.audio.isLoaded("sfx.door")).toBe(true);
    expect(rig.audio.isLoaded("sfx.nope")).toBe(false);
  });
});

/** Stands in for a missing source in a test that has already checked the source exists. */
function thrownAway(): never {
  throw new Error("the play should have started a source");
}
