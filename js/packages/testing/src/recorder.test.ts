import { GameLoop } from "@velvet/core";
import { afterEach, describe, expect, it } from "vitest";
import { createDriver, type Driver } from "./driver";
import { TestingError } from "./errors";
import { createRecorder, type Recorder } from "./recorder";
import { InputScript } from "./script";
import { startFakeGame, type FakeGame } from "./test-support";

const drivers: Driver[] = [];
const games: FakeGame[] = [];

afterEach(() => {
  for (const game of games.splice(0)) game.dispose();
  for (const driver of drivers.splice(0)) driver.dispose();
});

/** A driver and a recorder that are attached to each other, for one test. */
function recording(options: { checkEvery?: number; withHash?: boolean } = {}): {
  driver: Driver;
  recorder: Recorder;
  game: FakeGame;
} {
  const driver = createDriver();
  drivers.push(driver);
  let game: FakeGame | null = null;
  const recorder = createRecorder({
    game: "fake-game",
    checkEvery: options.checkEvery ?? 10,
    seeds: { coins: 3 },
    hash: options.withHash === false ? undefined : () => (game === null ? "" : game.hash()),
  });
  recorder.attach(driver);
  const started = startFakeGame(driver, { seed: 3 });
  games.push(started);
  game = started;
  return { driver, recorder, game: started };
}

describe("recording a driver", () => {
  it("records keys, pointer moves, buttons and the wheel at the frame where they happened", () => {
    const { driver, recorder } = recording();
    driver.step(2);
    driver.keyDown("Space");
    driver.keyUp("Space");
    driver.pointerMove(10, 20);
    driver.pointerDown("left", { x: 10, y: 20 });
    driver.pointerUp("left", { x: 10, y: 20 });
    driver.wheel(120);
    driver.blur();
    const replay = recorder.toReplay();
    expect(replay.frames).toBe(2);
    expect(replay.inputs.map((input) => [input.frame, input.type])).toEqual([
      [2, "key"],
      [2, "key"],
      [2, "pointer"],
      [2, "button"],
      [2, "button"],
      [2, "wheel"],
      [2, "blur"],
    ]);
    expect(replay.inputs[3]).toEqual({ frame: 2, type: "button", button: "left", down: true, x: 10, y: 20 });
  });

  it("marks a press that the page had already used, and leaves the flag off the others", () => {
    const { driver, recorder } = recording();
    driver.keyDown("Space", { skipped: true });
    driver.keyDown("KeyA");
    const inputs = recorder.toReplay().inputs;
    expect(inputs[0]).toEqual({ frame: 0, type: "key", code: "Space", down: true, skipped: true });
    expect(inputs[1]).toEqual({ frame: 0, type: "key", code: "KeyA", down: true });
  });

  it("samples the gamepads once per frame, and logs a pad only when its state changes", () => {
    const { driver, recorder } = recording();
    driver.pads.connect(0, { id: "Pad A" });
    driver.step(1);
    driver.pads.setButton(0, "a", 1);
    driver.step(3);
    driver.pads.disconnect(0);
    driver.step(1);
    const pads = recorder.toReplay().inputs.filter((input) => input.type === "pad");
    // The connection is seen on frame 0, the button on frame 1, and the disconnection on frame 4.
    expect(pads.map((pad) => [pad.frame, pad.type === "pad" && pad.connected])).toEqual([
      [0, true],
      [1, true],
      [4, false],
    ]);
  });

  it("takes a checkpoint at each interval, with the game's hash at that frame", () => {
    const { driver, recorder, game } = recording({ checkEvery: 4 });
    driver.step(4);
    const atFour = game.hash();
    driver.step(4);
    const atEight = game.hash();
    driver.step(2);
    const replay = recorder.toReplay();
    expect(replay.checkpoints).toEqual([
      { frame: 4, hash: atFour },
      { frame: 8, hash: atEight },
    ]);
  });

  it("records no checkpoints when the recording has no hash", () => {
    const { driver, recorder } = recording({ withHash: false });
    driver.step(20);
    expect(recorder.toReplay().checkpoints).toEqual([]);
  });

  it("refuses a driver at another rate", () => {
    const { recorder } = recording();
    const other = createDriver({ rate: 30 });
    drivers.push(other);
    expect(() => recorder.attach(other)).toThrow(TestingError);
  });

  it("stops recording when the listener is removed", () => {
    const { driver, recorder } = recording();
    const stop = recorder.attach(driver);
    stop();
    driver.step(1);
    expect(recorder.frames).toBe(1);
  });

  it("writes the game id, the seeds and the save into the replay", () => {
    const driver = createDriver();
    drivers.push(driver);
    const recorder = createRecorder({
      game: "fake-game",
      gameVersion: "1.2.3",
      seeds: { board: "18446744073709551615", coins: 9 },
      save: { format: "velvet-save", version: 1, game: "fake-game", savedAt: "2026-10-18T00:00:00.000Z", data: { coins: 4 } },
    });
    recorder.attach(driver);
    driver.createLoop({ update: () => undefined }).start();
    driver.step(2);
    const replay = recorder.toReplay();
    expect(replay.gameVersion).toBe("1.2.3");
    expect(replay.seeds).toEqual({ board: "18446744073709551615", coins: 9 });
    expect(replay.save?.data).toEqual({ coins: 4 });
  });
});

describe("recording a live loop through wrap", () => {
  it("counts the frames of the wrapped update, and checks the state at each interval", () => {
    const driver = createDriver();
    drivers.push(driver);
    let updates = 0;
    const recorder = createRecorder({ game: "live", checkEvery: 2, hash: () => `updates:${updates}` });
    const loop = new GameLoop({
      ...recorder.wrap({ update: () => (updates += 1) }),
      scheduler: driver.scheduler,
    });
    loop.start();
    driver.step(5);
    const replay = recorder.toReplay();
    expect(replay.frames).toBe(5);
    expect(updates).toBe(5);
    expect(replay.checkpoints).toEqual([
      { frame: 2, hash: "updates:2" },
      { frame: 4, hash: "updates:4" },
    ]);
    loop.stop();
  });

  it("refuses a checkEvery that is not a whole number from 1 up", () => {
    expect(() => createRecorder({ game: "live", checkEvery: 0 })).toThrow(/checkEvery/);
  });

  it("refuses a game id that is empty when the replay is written", () => {
    expect(() => createRecorder({ game: "" }).toReplay()).toThrow(/bad-replay|game/);
  });
});

describe("a session with scripted input", () => {
  it("keeps the frames in order, with the input of each frame in its order", () => {
    const { driver, recorder } = recording();
    const script = new InputScript();
    script.at(1).keyDown("KeyA");
    script.at(1).keyDown("KeyB");
    script.at(2).keyUp("KeyA");
    driver.play(script);
    driver.step(3);
    const codes = recorder.toReplay().inputs.map((input) => (input.type === "key" ? input.code : input.type));
    expect(codes).toEqual(["KeyA", "KeyB", "KeyA"]);
  });
});
