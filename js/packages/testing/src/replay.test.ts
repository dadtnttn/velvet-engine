import { createMemoryBackend, createSaveStore, type SaveEnvelope } from "@velvet/core";
import { afterEach, describe, expect, it } from "vitest";
import { createDriver, type Driver } from "./driver";
import { TestingError } from "./errors";
import { parseReplayText, serializeReplay, type Replay } from "./replay-format";
import { checkReplay, saveEnvelopeOf, startReplay, type ReplayStart } from "./replay";
import { createRecorder } from "./recorder";
import { InputScript } from "./script";
import { startFakeGame } from "./test-support";

const drivers: Driver[] = [];

afterEach(() => {
  for (const driver of drivers.splice(0)) driver.dispose();
});

/** Options for recording the fake game's session. */
interface SessionOptions {
  readonly startCoins?: number;
  readonly save?: SaveEnvelope | null;
}

/**
 * Records a session of the fake game: keys, a skipped press, clicks, a drag, pad changes, a wheel tick and a
 * blur, over 90 frames, with a checkpoint every 10 frames.
 */
function recordSession(options: SessionOptions = {}): Replay {
  const driver = createDriver();
  drivers.push(driver);
  let game: { hash(): string; dispose(): void } | null = null;
  const recorder = createRecorder({
    game: "fake-game",
    gameVersion: "0.1.0",
    rate: 60,
    seeds: { coins: 7 },
    save: options.save ?? null,
    checkEvery: 10,
    hash: () => (game === null ? "unstarted" : game.hash()),
  });
  recorder.attach(driver);
  game = startFakeGame(driver, { seed: 7, startCoins: options.startCoins ?? 0 });

  const script = new InputScript();
  script.at(0).padConnect({ pad: 0, id: "Test pad" });
  script.at(3).press("Space", { holdFrames: 5 });
  script.at(12).click({ x: 300, y: 200 }, { holdFrames: 4 });
  script.at(20).drag({ x: 300, y: 200 }, { x: 600, y: 400 }, 10);
  script.at(31).run((driverNow) => driverNow.keyDown("Space", { skipped: true }));
  script.at(33).run((driverNow) => driverNow.keyUp("Space"));
  script.at(40).press("Space", { holdFrames: 2 });
  script.at(40).padButton("a", { holdFrames: 6 });
  script.at(50).padStick("left", 0.8, 0, { holdFrames: 10 });
  script.at(64).padDisconnect(0);
  script.at(70).wheel(120);
  script.at(75).blur();
  driver.play(script);
  driver.step(90);
  const replay = recorder.toReplay();
  game.dispose();
  return replay;
}

/** A replay start for the fake game. The seed and the jump bonus can be changed to test a divergence. */
function fakeStart(overrides: { seed?: number | string; jumpBonus?: (frame: number) => number } = {}): ReplayStart {
  return ({ driver, seeds, save }) => {
    const game = startFakeGame(driver, {
      seed: overrides.seed ?? seeds.coins ?? 0,
      jumpBonus: overrides.jumpBonus,
      startCoins: coinsOf(save),
    });
    return { hash: () => game.hash() };
  };
}

/** The coins that a save holds, or 0 for a new game. */
function coinsOf(save: SaveEnvelope | null): number {
  const data: unknown = save?.data;
  if (typeof data === "object" && data !== null && "coins" in data && typeof data.coins === "number") return data.coins;
  return 0;
}

describe("replaying a recorded session", () => {
  it("reproduces the hash at every checkpoint", async () => {
    const replay = recordSession();
    expect(replay.frames).toBe(90);
    expect(replay.checkpoints.map((checkpoint) => checkpoint.frame)).toEqual([10, 20, 30, 40, 50, 60, 70, 80, 90]);
    const result = await checkReplay(replay, fakeStart());
    expect(result).toEqual({ ok: true, frames: 90, checkpoints: 9 });
  });

  it("holds the skipped press, the pads and the blur in the recording, so the replay can reproduce them", () => {
    const replay = recordSession();
    expect(replay.inputs.some((input) => input.type === "key" && input.skipped === true)).toBe(true);
    expect(replay.inputs.some((input) => input.type === "pad" && !input.connected)).toBe(true);
    expect(replay.inputs.some((input) => input.type === "blur")).toBe(true);
  });

  it("replays a written and read-back document with the same result", async () => {
    const replay = parseReplayText(serializeReplay(recordSession()));
    expect((await checkReplay(replay, fakeStart())).ok).toBe(true);
  });

  it("runs from the seeds of the recording: a different seed diverges at the first checkpoint that sees it", async () => {
    const replay = recordSession();
    const result = await checkReplay(replay, fakeStart({ seed: 99 }));
    expect(result).toMatchObject({ ok: false, frame: 20, previousFrame: 10 });
  });

  it("reports a change to the game at the first checkpoint that differs, with both hashes", async () => {
    const replay = recordSession();
    const changed = await checkReplay(replay, fakeStart({ jumpBonus: (frame) => (frame >= 25 ? 2 : 1) }));
    expect(changed.ok).toBe(false);
    if (changed.ok) return;
    expect(changed.frame).toBe(50);
    expect(changed.previousFrame).toBe(40);
    expect(changed.frames).toBe(90);
    expect(changed.expected).toMatch(/^[0-9a-f]{16}$/);
    expect(changed.actual).toMatch(/^[0-9a-f]{16}$/);
    expect(changed.actual).not.toBe(changed.expected);
  });

  it("can be paced one frame at a time, and reports its result when it is done", async () => {
    const replay = recordSession();
    const run = await startReplay(replay, fakeStart());
    try {
      run.step(10);
      expect(run.frame).toBe(10);
      expect(run.done).toBe(false);
      expect(run.result).toBeNull();
      expect(run.finish()).toEqual({ ok: true, frames: 90, checkpoints: 9 });
      expect(run.done).toBe(true);
      expect(run.frame).toBe(90);
    } finally {
      run.dispose();
    }
  });

  it("starts from the save of the recording, which the game reads from the context", async () => {
    const store = createSaveStore({ game: "fake-game", version: 1, backend: createMemoryBackend() });
    await store.save("start", { coins: 4 });
    const loaded = await store.load("start");
    expect(loaded).not.toBeNull();
    const envelope = saveEnvelopeOf(store, loaded ?? { savedAt: "", data: null });
    const replay = recordSession({ startCoins: 4, save: envelope });
    expect(replay.save?.data).toEqual({ coins: 4 });
    expect((await checkReplay(replay, fakeStart())).ok).toBe(true);
    // Without the save the game starts from zero coins, and the replay must diverge.
    const withoutSave = await checkReplay(replay, ({ driver }) => {
      const game = startFakeGame(driver, { seed: 7 });
      return { hash: () => game.hash() };
    });
    expect(withoutSave.ok).toBe(false);
  });

  it("refuses a driver that has already run frames, since the recording counts from frame 0", async () => {
    const replay = recordSession();
    const driver = createDriver();
    drivers.push(driver);
    driver.createLoop({ update: () => undefined }).start();
    driver.step(3);
    let refused: unknown = null;
    try {
      await startReplay(replay, fakeStart(), { driver });
    } catch (error) {
      refused = error;
    }
    expect(refused).toBeInstanceOf(TestingError);
    expect((refused as TestingError).message).toMatch(/already run 3 frames/);
  });

  it("refuses a driver at another rate", async () => {
    const replay = recordSession();
    const driver = createDriver({ rate: 30 });
    drivers.push(driver);
    let refused: unknown = null;
    try {
      await startReplay(replay, fakeStart(), { driver });
    } catch (error) {
      refused = error;
    }
    expect(refused).toBeInstanceOf(TestingError);
    expect((refused as TestingError).code).toBe("bad-argument");
  });
});
