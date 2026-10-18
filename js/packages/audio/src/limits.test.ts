// Tests for the limits on a sound: the cooldown between plays, the voice limit that steals the oldest
// copy, and the random variants that never repeat the last one.

import { describe, expect, it } from "vitest";
import { chainOf, createRig, settle } from "./test-support";

describe("cooldowns", () => {
  const clack = { id: "sfx.clack", files: ["sfx/clack.opus"], cooldown_ms: 250 };

  it("skips a play that comes before the cooldown is over", async () => {
    const rig = createRig({ sounds: [clack] });
    rig.audio.play("sfx.clack");
    rig.time.ms = 100;
    const early = rig.audio.play("sfx.clack");
    expect(early.status).toBe("ended");
    await expect(early.ended).resolves.toBe("skipped");
    await settle();
    expect(rig.context.sources).toHaveLength(1);
  });

  it("plays again once the cooldown has passed", async () => {
    const rig = createRig({ sounds: [clack] });
    rig.audio.play("sfx.clack");
    rig.time.ms = 250;
    const handle = rig.audio.play("sfx.clack");
    await settle();
    expect(handle.status).toBe("playing");
    expect(rig.context.sources).toHaveLength(2);
  });

  it("is counted from the last play that was not skipped", async () => {
    const rig = createRig({ sounds: [clack] });
    rig.audio.play("sfx.clack");
    rig.time.ms = 200;
    rig.audio.play("sfx.clack");
    rig.time.ms = 260;
    const handle = rig.audio.play("sfx.clack");
    expect(handle.status).not.toBe("ended");
    await settle();
    expect(handle.status).toBe("playing");
  });

  it("does not apply to a sound without a cooldown", async () => {
    const rig = createRig({ sounds: [{ id: "sfx.tap", files: ["sfx/tap.opus"] }] });
    rig.audio.play("sfx.tap");
    rig.audio.play("sfx.tap");
    await settle();
    expect(rig.context.sources).toHaveLength(2);
  });
});

describe("voice limits", () => {
  const step = { id: "sfx.step", files: ["sfx/step.opus"], max_voices: 2 };

  it("steals the oldest copy when a new one would pass the limit", async () => {
    const rig = createRig({ sounds: [step] });
    const first = rig.audio.play("sfx.step");
    await settle();
    rig.audio.play("sfx.step");
    await settle();
    const third = rig.audio.play("sfx.step");
    await settle();

    expect(first.status).toBe("ended");
    await expect(first.ended).resolves.toBe("stolen");
    expect(third.status).toBe("playing");
    const stolen = chainOf(rig.context.sources[0] ?? failMissing());
    expect(stolen.source.stopAt).toBeCloseTo(0.008, 12);
    expect(stolen.gain.gain.lastRamp()?.value).toBe(0);
  });

  it("does not count a copy that has finished", async () => {
    const rig = createRig({ sounds: [step] });
    const first = rig.audio.play("sfx.step");
    await settle();
    rig.context.advance(2);
    await expect(first.ended).resolves.toBe("finished");
    const second = rig.audio.play("sfx.step");
    const third = rig.audio.play("sfx.step");
    await settle();
    expect(second.status).toBe("playing");
    expect(third.status).toBe("playing");
  });

  it("counts a copy from the time it starts, not the time it is asked for", async () => {
    const rig = createRig({ sounds: [step], running: false });
    const waiting = [rig.audio.play("sfx.step"), rig.audio.play("sfx.step"), rig.audio.play("sfx.step")];
    rig.context.state = "running";
    rig.audio.update();
    await settle();
    expect(waiting.map((handle) => handle.status)).toEqual(["ended", "playing", "playing"]);
    await expect(waiting[0]?.ended).resolves.toBe("stolen");
  });
});

describe("variants", () => {
  // Each variant is a file of a different length, so the length of the source tells which one played.
  const stone = {
    id: "sfx.stone",
    variants: [
      { files: ["sfx/stone_1.opus"] },
      { files: ["sfx/stone_2.opus"] },
      { files: ["sfx/stone_3.opus"] },
    ],
  };
  const files = {
    "audio/sfx/stone_1.opus": { bytes: 1000 },
    "audio/sfx/stone_2.opus": { bytes: 2000 },
    "audio/sfx/stone_3.opus": { bytes: 3000 },
  };

  it("never plays the variant that played last", async () => {
    const draws = [0, 0, 0, 0.99, 0.5, 0.2];
    const rig = createRig({ sounds: [stone], files, random: () => draws.shift() ?? 0 });
    for (let play = 0; play < 6; play += 1) {
      rig.audio.play("sfx.stone");
      await settle();
    }
    // Variant indexes 0, 1, 0, 2, 1, 0: the length of each source gives its variant plus one.
    expect(rig.context.sources.map((source) => source.buffer?.duration)).toEqual([1, 2, 1, 3, 2, 1]);
  });

  it("chooses among the others when the sound has more than one variant", async () => {
    const rig = createRig({ sounds: [stone], files, random: () => 0.5 });
    for (let play = 0; play < 4; play += 1) {
      rig.audio.play("sfx.stone");
      await settle();
    }
    const lengths = rig.context.sources.map((source) => source.buffer?.duration);
    for (let index = 1; index < lengths.length; index += 1) {
      expect(lengths[index]).not.toBe(lengths[index - 1]);
    }
  });

  it("plays a sound with a single file the same way every time", async () => {
    const rig = createRig({ sounds: [{ id: "sfx.tap", files: ["sfx/tap.opus"] }], random: () => 0.9 });
    for (let play = 0; play < 3; play += 1) {
      rig.audio.play("sfx.tap");
      await settle();
    }
    expect(rig.network.requests).toEqual(["audio/sfx/tap.opus"]);
  });
});

/** Stands in for a source that a test expects to exist. */
function failMissing(): never {
  throw new Error("expected a started source");
}
