// @vitest-environment happy-dom
// The whole game in a page, driven by @velvet/testing's driver: the title and the level, the pause menu, the language,
// and a scripted playthrough. The playthrough is a deterministic bot that holds right and jumps over what is in its
// way; its input is recorded as a replay, and the replay must reproduce the state hashes and the run.
//
// Every game that a test makes is stopped by `afterEach`, even when the test fails, so that one failure cannot leave a
// screen listening to the keys for the next test.

import { afterAll, afterEach, beforeAll, describe, expect, it, vi } from "vitest";
import { createLogger, type Logger } from "@velvet/core";
import {
  checkReplay,
  createDriver,
  createRecorder,
  parseReplayText,
  serializeReplay,
  type Driver,
} from "@velvet/testing";
import { Game } from "./game";
import { HERO, STEPS_PER_SECOND } from "./config";
import { fakeCanvas, fakeImage, fetchFromPublicFolder, memoryBackend, must, wantsJump } from "./test-support";

beforeAll(() => {
  fetchFromPublicFolder();
});

/** The stops that the tests registered; `afterEach` runs them all. */
const stops: Array<() => void> = [];

afterEach(() => {
  for (const stop of stops.splice(0).reverse()) stop();
  document.body.innerHTML = "";
});

afterAll(() => {
  vi.unstubAllGlobals();
});

/** A stop that runs once, so that a test can stop a game early and the cleanup can still run. */
function once(stop: () => void): () => void {
  let done = false;
  return () => {
    if (done) return;
    done = true;
    stop();
  };
}

/** A host element for one game, added to the page. */
function mountHost(): HTMLElement {
  const host = document.createElement("div");
  host.className = "game-host";
  document.body.append(host);
  return host;
}

/** A logger that writes nowhere, so that the test output stays readable. */
function quietLogger(): Logger {
  return createLogger({ name: "hello-2d", level: "error", sinks: [] });
}

/** A driver for a test, stopped at the end of the test. */
function driverForTest(): Driver {
  const driver = createDriver();
  stops.push(() => driver.dispose());
  return driver;
}

/**
 * Starts the game on a driver: the loop of the driver, memory for the settings and the achievements, a stand-in canvas
 * and pictures, and no sound. The game is stopped at the end of the test. It stays on the title screen.
 * @param driver - the driver that runs the frames.
 * @param host - the element that the game fills.
 */
async function startOn(driver: Driver, host: HTMLElement): Promise<Game> {
  const game = await Game.start(host, {
    createLoop: (options) => driver.createLoop(options),
    sound: false,
    devtools: false,
    logger: quietLogger(),
    backends: { settings: memoryBackend(), achievements: memoryBackend() },
    loadImage: fakeImage,
    createCanvas: fakeCanvas,
  });
  stops.push(() => game.dispose());
  return game;
}

/** Starts the game and begins the level, as the title's Play does. */
async function startOnLevel(driver: Driver, host: HTMLElement): Promise<Game> {
  const game = await startOn(driver, host);
  game.startNew();
  return game;
}

/** Presses a key for one frame, as the keyboard does: the key goes down, the frame runs, and the key goes up. */
function tap(driver: Driver, code: string): void {
  driver.keyDown(code);
  driver.step(1);
  driver.keyUp(code);
  driver.step(2);
}

/**
 * Plays the level to the flag. The hero holds right all the time; before each jump the bot looks at the level and
 * jumps when a gap, a wall or spikes are just ahead. Each jump holds the jump key for a few frames, so the hero jumps
 * high enough to clear what is there.
 * @param driver - the driver.
 * @param game - the game being played.
 */
function playToFlag(driver: Driver, game: Game): void {
  const holdFrames = 14;
  let heldFor = 0;
  driver.keyDown("ArrowRight");
  for (let frame = 0; frame < 3000 && game.run.phase !== "finished"; frame += 1) {
    if (heldFor === 0 && wantsJump(game.run.level, game.run.hero)) {
      driver.keyDown("Space");
      heldFor = holdFrames;
    }
    driver.step(1);
    if (heldFor > 0) {
      heldFor -= 1;
      if (heldFor === 0) driver.keyUp("Space");
    }
  }
  if (heldFor > 0) driver.keyUp("Space");
  driver.keyUp("ArrowRight");
  expect(game.run.phase).toBe("finished");
}

describe("the menus", () => {
  it("starts on the title, and the level starts on the ground", async () => {
    const driver = driverForTest();
    const game = await startOn(driver, mountHost());
    expect(game.screen).toBe("title");
    game.startNew();
    expect(game.screen).toBe("game");
    driver.step(120);
    expect(game.run.hero.grounded).toBe(true);
    expect(game.run.hero.x).toBe(HERO.width / 2 + 60);
  });

  it("pauses with Escape: the level stops until the pause is closed", async () => {
    const driver = driverForTest();
    const game = await startOnLevel(driver, mountHost());
    driver.step(120);
    tap(driver, "Escape");
    expect(game.screen).toBe("pause");
    const before = game.run.hero.x;
    driver.keyDown("ArrowRight");
    driver.step(60);
    expect(game.run.hero.x).toBe(before);
    driver.keyUp("ArrowRight");
    tap(driver, "Escape");
    expect(game.screen).toBe("game");
  });

  it("moves the hero while the key is held, at the run speed", async () => {
    const driver = driverForTest();
    const game = await startOnLevel(driver, mountHost());
    driver.step(120);
    const before = game.run.hero.x;
    driver.keyDown("ArrowRight");
    driver.step(STEPS_PER_SECOND);
    driver.keyUp("ArrowRight");
    expect(game.run.hero.x - before).toBeGreaterThan(150);
    expect(game.run.hero.x - before).toBeLessThan(230);
  });

  it("writes the words of the menus again in the language that is set", async () => {
    const driver = driverForTest();
    const game = await startOn(driver, mountHost());
    expect(must<HTMLElement>(document, ".h2d-title-sub").textContent).toContain("Velvet JS");
    expect(document.documentElement.lang).toBe("es");
    game.setLanguage("en");
    expect(game.language).toBe("en");
    expect(document.documentElement.lang).toBe("en");
    expect(must<HTMLElement>(document, ".h2d-title-sub").textContent).toBe("A short platformer · Velvet JS");
    game.setLanguage("es");
    expect(must<HTMLElement>(document, ".h2d-title-sub").textContent).toBe("Un plataformas corto · Velvet JS");
  });
});

describe("the saved progress", () => {
  it("keeps the achievements that an earlier run saved", async () => {
    const driver = driverForTest();
    const achievements = memoryBackend();
    const startWith = async (): Promise<Game> => {
      const game = await Game.start(mountHost(), {
        createLoop: (options) => driver.createLoop(options),
        sound: false,
        devtools: false,
        logger: quietLogger(),
        backends: { settings: memoryBackend(), achievements },
        loadImage: fakeImage,
        createCanvas: fakeCanvas,
      });
      stops.push(() => game.dispose());
      return game;
    };

    const first = await startWith();
    first.achievements.increment("coins");
    expect(first.achievements.isUnlocked("first_coin")).toBe(true);
    expect(await first.achievements.save()).toBe(true);

    const second = await startWith();
    expect(second.achievements.isUnlocked("first_coin")).toBe(true);
  });
});

describe("the scripted playthrough", () => {
  it("reaches the flag with a bot, the run is clean, and the achievements of the run unlock", async () => {
    const driver = driverForTest();
    const game = await startOnLevel(driver, mountHost());
    playToFlag(driver, game);
    expect(game.screen).toBe("result");
    expect(game.run.hurts).toBe(0);
    expect(game.run.coins).toBeGreaterThan(0);
    expect(game.achievements.isUnlocked("first_coin")).toBe(true);
    expect(game.achievements.isUnlocked("clean_finish")).toBe(true);
    expect(game.achievements.isUnlocked("speedrun")).toBe(true);
    expect(game.achievements.isUnlocked("completionist")).toBe(false);
    expect(document.body.textContent).toContain("Primera moneda");
  });

  it("is recorded, and the replay reproduces every checkpoint and the final state", async () => {
    const driver = driverForTest();
    let recorded: Game | null = null;
    const recorder = createRecorder({
      game: "hello-2d",
      checkEvery: 30,
      hash: () => (recorded === null ? "" : recorded.hash()),
    });
    recorder.attach(driver);
    const stopRecorded = once(() => recorded?.dispose());
    stops.push(stopRecorded);

    const game = await startOnLevel(driver, mountHost());
    recorded = game;
    playToFlag(driver, game);
    const finalHash = game.hash();
    const text = serializeReplay(recorder.toReplay());
    stopRecorded();

    const replays: Game[] = [];
    const stopReplays = once(() => replays.forEach((replay) => replay.dispose()));
    stops.push(stopReplays);
    const result = await checkReplay(parseReplayText(text), async ({ driver: replayDriver }) => {
      const started = await startOnLevel(replayDriver, mountHost());
      replays.push(started);
      return { hash: () => started.hash() };
    });
    expect(result.ok).toBe(true);
    expect(result.ok && result.checkpoints).toBeGreaterThan(0);
    expect(replays[0]?.hash()).toBe(finalHash);
    expect(replays[0]?.run.phase).toBe("finished");
    stopReplays();
  });
});
