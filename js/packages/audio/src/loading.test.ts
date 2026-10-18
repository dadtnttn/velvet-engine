// Tests for the cache of decoded files: a file is fetched once while it loads and once for good, the
// preload reports its progress and its failures, and unloading forgets a file.

import { describe, expect, it, vi } from "vitest";
import { AudioError } from "./error";
import { createRig, settle } from "./test-support";

const hum = { id: "sfx.hum", files: ["sfx/hum.opus"] };
const bell = { id: "sfx.bell", files: ["sfx/bell.opus"] };

describe("the decoded cache", () => {
  it("fetches a file once when two plays need it while it loads", async () => {
    const rig = createRig({ sounds: [hum] });
    const first = rig.audio.play("sfx.hum");
    const second = rig.audio.play("sfx.hum");
    await settle();
    expect(rig.network.requests).toEqual(["audio/sfx/hum.opus"]);
    expect(first.status).toBe("playing");
    expect(second.status).toBe("playing");
    expect(rig.context.sources).toHaveLength(2);
  });

  it("does not fetch a decoded file again", async () => {
    const rig = createRig({ sounds: [hum] });
    rig.audio.play("sfx.hum");
    await settle();
    rig.audio.play("sfx.hum");
    await settle();
    expect(rig.network.requests).toHaveLength(1);
    expect(rig.context.decodeCount).toBe(1);
  });
});

describe("preload", () => {
  it("decodes the named sounds and reports progress after each one", async () => {
    const rig = createRig({ sounds: [hum, bell] });
    const progress = vi.fn();
    const result = await rig.audio.preload(["sfx.hum", "sfx.bell"], progress);
    expect(result).toEqual({ failed: [] });
    expect(progress).toHaveBeenCalledTimes(2);
    expect(progress.mock.calls.map((call) => call[0]?.done)).toEqual([1, 2]);
    expect(progress.mock.calls[0]?.[0]).toMatchObject({ total: 2 });
    expect(rig.audio.isLoaded("sfx.hum")).toBe(true);
    expect(rig.audio.isLoaded("sfx.bell")).toBe(true);
  });

  it("lists the sounds that could not be loaded, and reports each one as an error", async () => {
    const rig = createRig({ sounds: [hum, bell], files: { "audio/sfx/hum.opus": { bytes: 1000 } } });
    const errors = vi.fn();
    rig.audio.on("error", errors);
    const result = await rig.audio.preload(["sfx.hum", "sfx.bell"]);
    expect(result).toEqual({ failed: ["sfx.bell"] });
    expect(errors).toHaveBeenCalledTimes(1);
    expect(errors.mock.calls[0]?.[0]).toMatchObject({ id: "sfx.bell" });
  });

  it("with no ids, decodes the sounds that the manifest marks for preloading", async () => {
    const rig = createRig({ sounds: [{ ...hum, preload: true }, bell] });
    await rig.audio.preload();
    expect(rig.network.requests).toEqual(["audio/sfx/hum.opus"]);
    expect(rig.audio.isLoaded("sfx.bell")).toBe(false);
  });

  it("refuses an id that the manifest does not have, before it decodes anything", async () => {
    const rig = createRig({ sounds: [hum] });
    await expect(rig.audio.preload(["sfx.hum", "sfx.nope"])).rejects.toBeInstanceOf(AudioError);
    expect(rig.network.requests).toHaveLength(0);
  });
});

describe("unload", () => {
  it("forgets a decoded file, so the next play fetches it again", async () => {
    const rig = createRig({ sounds: [hum] });
    rig.audio.play("sfx.hum");
    await settle();
    rig.audio.unload(["sfx.hum"]);
    expect(rig.audio.isLoaded("sfx.hum")).toBe(false);
    rig.audio.play("sfx.hum");
    await settle();
    expect(rig.network.requests).toHaveLength(2);
  });

  it("keeps a play that is already running", async () => {
    const rig = createRig({ sounds: [hum] });
    const handle = rig.audio.play("sfx.hum");
    await settle();
    rig.audio.unload();
    expect(handle.status).toBe("playing");
    expect(rig.audio.isLoaded("sfx.hum")).toBe(false);
  });

  it("does not keep a file whose load was still running when it was unloaded", async () => {
    const rig = createRig({ sounds: [hum] });
    const handle = rig.audio.play("sfx.hum");
    rig.audio.unload(["sfx.hum"]);
    await settle();
    expect(rig.audio.isLoaded("sfx.hum")).toBe(false);
    // The play still starts: unloading drops the cache, not the request that is waiting for it.
    expect(handle.status).toBe("playing");
  });

  it("refuses an id that the manifest does not have", () => {
    const rig = createRig({ sounds: [hum] });
    expect(() => rig.audio.unload(["sfx.nope"])).toThrow(AudioError);
  });
});
