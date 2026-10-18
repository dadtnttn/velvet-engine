// Tests for the browser's rule that audio starts only after a user gesture. Plays made before the
// first gesture wait; the first press, key or touch unlocks the context and starts them; a refused
// unlock is tried again at the next gesture. Closing the engine stops everything and frees the page.

import { describe, expect, it } from "vitest";
import { createRig, settle } from "./test-support";

const door = { id: "sfx.door", files: ["sfx/door.opus"] };

describe("the first gesture", () => {
  it("holds the plays that are made before it, and starts them on the gesture", async () => {
    const rig = createRig({ sounds: [door], running: false });
    const handle = rig.audio.play("sfx.door");
    await settle();
    expect(handle.status).toBe("pending");
    expect(rig.context.sources).toHaveLength(0);
    expect(rig.audio.unlocked).toBe(false);
    expect(rig.context.resumeCalls).toBe(0);

    rig.host.gesture();
    await settle();
    expect(rig.audio.unlocked).toBe(true);
    expect(rig.context.resumeCalls).toBe(1);
    expect(handle.status).toBe("playing");
  });

  it("stops listening for gestures once the audio runs", async () => {
    const rig = createRig({ sounds: [door], running: false });
    rig.audio.play("sfx.door");
    expect(rig.host.listenerCount).toBe(2);
    rig.host.gesture();
    await settle();
    expect(rig.host.listenerCount).toBe(1);
    rig.host.gesture();
    await settle();
    expect(rig.context.resumeCalls).toBe(1);
  });

  it("needs no gesture when the runtime lets the context run from the start", async () => {
    const rig = createRig({ sounds: [door] });
    const handle = rig.audio.play("sfx.door");
    expect(rig.audio.unlocked).toBe(true);
    await settle();
    expect(handle.status).toBe("playing");
    expect(rig.context.resumeCalls).toBe(0);
  });
});

describe("unlock", () => {
  it("resolves true once the audio runs", async () => {
    const rig = createRig({ sounds: [door], running: false });
    await expect(rig.audio.unlock()).resolves.toBe(true);
    expect(rig.audio.unlocked).toBe(true);
  });

  it("resolves false when the browser refuses, and tries again at the next gesture", async () => {
    const rig = createRig({ sounds: [door], running: false });
    rig.context.resumeAllowed = false;
    const handle = rig.audio.play("sfx.door");
    await expect(rig.audio.unlock()).resolves.toBe(false);
    expect(rig.audio.unlocked).toBe(false);
    expect(rig.host.listenerCount).toBe(2);

    rig.context.resumeAllowed = true;
    rig.host.gesture();
    await settle();
    expect(rig.audio.unlocked).toBe(true);
    expect(handle.status).toBe("playing");
  });
});

describe("close", () => {
  it("stops every play, removes the page's listeners and closes the context", async () => {
    const rig = createRig({ sounds: [door] });
    const handle = rig.audio.play("sfx.door");
    await settle();
    await rig.audio.close();
    await expect(handle.ended).resolves.toBe("stopped");
    expect(rig.host.listenerCount).toBe(0);
    expect(rig.context.state).toBe("closed");
  });

  it("refuses new plays by ending them as skipped", async () => {
    const rig = createRig({ sounds: [door] });
    await rig.audio.close();
    const handle = rig.audio.play("sfx.door");
    expect(handle.status).toBe("ended");
    await expect(handle.ended).resolves.toBe("skipped");
  });
});
