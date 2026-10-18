// @vitest-environment happy-dom
// The whole game in a page, driven by @velvet/testing's driver: a playthrough to each ending, recorded as a replay
// and checked against the state hashes; the pause menu; the settings; a language switch in the middle of a scene;
// the save list, with a save and a load through the page; and the ending card.
//
// Every game and driver that a test makes is stopped by `afterEach`, even when the test fails, so that one failure
// cannot leave a screen listening to the keys for the next test.

import { afterAll, afterEach, beforeAll, describe, expect, it, vi } from "vitest";
import { createMemoryBackend } from "@velvet/core";
import {
  checkReplay,
  createDriver,
  createRecorder,
  parseReplayText,
  serializeReplay,
  type Driver,
} from "@velvet/testing";
import { Game } from "./game";
import { fetchFromPublicFolder, loadWasmForTests, memorySettings, must, waitUntil } from "./test-support";

beforeAll(async () => {
  await loadWasmForTests();
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

/**
 * Starts the game on a driver, with the loop of the driver, memory for the saves and the settings, and no sound.
 * The title screen is left for a new story, as a player does from the title. The game is stopped at the end of
 * the test.
 * @param driver - the driver that runs the frames.
 * @param host - the element that the game fills.
 */
async function startOn(driver: Driver, host: HTMLElement): Promise<Game> {
  const game = await Game.start(host, {
    createLoop: (options) => driver.createLoop(options),
    sound: false,
    devtools: false,
    saveBackend: createMemoryBackend(),
    settingsBackend: memorySettings(),
  });
  stops.push(() => game.dispose());
  game.startNew();
  return game;
}

/** A driver for a test, stopped at the end of the test. */
function driverForTest(): Driver {
  const driver = createDriver();
  stops.push(() => driver.dispose());
  return driver;
}

/** Presses a key for one frame, as the keyboard does: the key goes down, the frame runs, and the key goes up. */
function tap(driver: Driver, code: string): void {
  driver.keyDown(code);
  driver.step(1);
  driver.keyUp(code);
  driver.step(2);
}

/**
 * Goes on through the story with Enter until the predicate is true. Each choice on the way takes the next index of
 * `picks`. Fails when the predicate does not come true within the budget.
 * @param driver - the driver.
 * @param game - the game.
 * @param until - the condition that ends the walk.
 * @param picks - the options of the choices, in order.
 */
function walk(driver: Driver, game: Game, until: () => boolean, picks: readonly number[] = []): void {
  let pick = 0;
  for (let guard = 0; guard < 400 && !until(); guard += 1) {
    const box = game.box;
    if (box?.type === "choice") tap(driver, `Digit${(picks[pick++] ?? 0) + 1}`);
    else tap(driver, "Enter");
  }
  expect(until()).toBe(true);
}

/** The text of the ending card's title, as it is on the page. */
function endingTitle(): string {
  return must<HTMLElement>(document, ".vn-ending-title").textContent ?? "";
}

describe("the game in a page", () => {
  it("plays to the light ending through the tavern, and the replay reproduces the play", async () => {
    const driver = driverForTest();
    let recorded: Game | null = null;
    const recorder = createRecorder({
      game: "hello-vn",
      checkEvery: 30,
      hash: () => (recorded === null ? "" : recorded.hash()),
    });
    recorder.attach(driver);
    const stopRecorded = once(() => recorded?.dispose());
    stops.push(stopRecorded);
    const game = await startOn(driver, mountHost());
    recorded = game;
    walk(driver, game, () => game.screen === "ending", [1, 0]);
    expect(game.ending).toBe(1);
    expect(endingTitle()).toBe("La luz vuelve a la bahía");
    const finalHash = game.hash();
    const text = serializeReplay(recorder.toReplay());
    stopRecorded();

    const replays: Game[] = [];
    const stopReplays = once(() => replays.forEach((replay) => replay.dispose()));
    stops.push(stopReplays);
    const result = await checkReplay(parseReplayText(text), async ({ driver: replayDriver }) => {
      const started = await startOn(replayDriver, mountHost());
      replays.push(started);
      return { hash: () => started.hash() };
    });
    expect(result.ok).toBe(true);
    expect(result.ok && result.checkpoints).toBeGreaterThan(0);
    expect(replays[0]?.hash()).toBe(finalHash);
    stopReplays();
  });

  it("plays to the bell ending by the direct way, and the replay reproduces it", async () => {
    const driver = driverForTest();
    let recorded: Game | null = null;
    const recorder = createRecorder({
      game: "hello-vn",
      checkEvery: 30,
      hash: () => (recorded === null ? "" : recorded.hash()),
    });
    recorder.attach(driver);
    const stopRecorded = once(() => recorded?.dispose());
    stops.push(stopRecorded);
    const game = await startOn(driver, mountHost());
    recorded = game;
    walk(driver, game, () => game.screen === "ending", [0, 1]);
    expect(game.ending).toBe(2);
    expect(endingTitle()).toBe("Una campana en la niebla");
    const text = serializeReplay(recorder.toReplay());
    stopRecorded();

    const result = await checkReplay(parseReplayText(text), async ({ driver: replayDriver }) => {
      const started = await startOn(replayDriver, mountHost());
      return { hash: () => started.hash() };
    });
    expect(result.ok).toBe(true);
  });

  it("opens the pause menu with Escape and closes it again, and changes the text speed from it", async () => {
    const driver = driverForTest();
    const game = await startOn(driver, mountHost());
    tap(driver, "Escape");
    expect(game.screen).toBe("pause");
    must<HTMLElement>(document, '[data-id="settings"]').click();
    expect(game.screen).toBe("settings");
    expect(must<HTMLElement>(document, '[data-id="speed"]').textContent).toContain("40");
    must<HTMLElement>(document, '[data-id="speed"]').click();
    expect(must<HTMLElement>(document, '[data-id="speed"]').textContent).toContain("80");
    tap(driver, "Escape");
    expect(game.screen).toBe("pause");
    tap(driver, "Escape");
    expect(game.screen).toBe("game");
  });

  it("switches the language in the middle of a scene, and the ending card follows", async () => {
    const driver = driverForTest();
    const game = await startOn(driver, mountHost());
    // The first choice takes the tavern; the walk stops at the lighthouse's choice.
    walk(driver, game, () => game.box?.type === "choice" && game.box.prompt === "¿Cómo avisamos a los barcos?", [1]);
    game.setLanguage("en");
    expect(game.box?.type === "choice" && game.box.prompt).toBe("How do we warn the boats?");
    tap(driver, "Digit1");
    expect(game.screen).toBe("ending");
    expect(endingTitle()).toBe("The light returns to the bay");
  });

  it("does not close the ending card with Escape, and its button goes back to the title", async () => {
    const driver = driverForTest();
    const game = await startOn(driver, mountHost());
    walk(driver, game, () => game.screen === "ending", [0, 1]);
    tap(driver, "Escape");
    expect(game.screen).toBe("ending");
    must<HTMLElement>(document, ".vn-ending .vu-button").click();
    expect(game.screen).toBe("title");
  });

  it("saves from the pause menu into a slot, and loads the same box back from the list", async () => {
    const driver = driverForTest();
    const game = await startOn(driver, mountHost());
    walk(driver, game, () => game.box?.type === "choice");
    const prompt = game.box?.type === "choice" ? game.box.prompt : null;
    expect(prompt).toBe("¿Qué haces?");

    tap(driver, "Escape");
    must<HTMLElement>(document, '[data-id="save"]').click();
    expect(game.screen).toBe("saves");
    await waitUntil(() => document.querySelector('[data-slot="slot-1"]') !== null);
    must<HTMLElement>(document, '[data-slot="slot-1"]').click();
    await waitUntil(() => (document.querySelector(".vu-notices")?.textContent ?? "").includes("Partida guardada"));

    // Leave the menus, go on past the choice, then load the save: the choice comes back on screen.
    tap(driver, "Escape");
    tap(driver, "Escape");
    expect(game.screen).toBe("game");
    tap(driver, "Digit2"); // the tavern
    walk(driver, game, () => game.box?.type === "choice");
    expect(game.box?.type === "choice" && game.box.prompt).toBe("¿Cómo avisamos a los barcos?");

    tap(driver, "Escape");
    must<HTMLElement>(document, '[data-id="load"]').click();
    expect(game.screen).toBe("saves");
    await waitUntil(() => document.querySelector('[data-slot="slot-1"][data-state="filled"]') !== null);
    must<HTMLElement>(document, '[data-slot="slot-1"][data-state="filled"]').click();
    await waitUntil(() => game.screen === "game");
    expect(game.box?.type === "choice" && game.box.prompt).toBe(prompt);
  });
});
