// @vitest-environment happy-dom
// The in-page path: a live loop wrapped by a recorder, trusted events observed on the window, and a replay
// on a fresh driver that puts the same events back. happy-dom has no real input, so the trusted events are
// made here with `isTrusted` set, the way the browser sets it for a real key or pointer.

import { afterEach, describe, expect, it } from "vitest";
import { createDriver, type Driver } from "./driver";
import { createRecorder, type Recorder } from "./recorder";
import { checkReplay } from "./replay";
import { createStageMap } from "./stage";
import { startFakeGame, type FakeGame } from "./test-support";

const drivers: Driver[] = [];
const games: FakeGame[] = [];
const stops: Array<() => void> = [];

afterEach(() => {
  for (const stop of stops.splice(0)) stop();
  for (const game of games.splice(0)) game.dispose();
  for (const driver of drivers.splice(0)) driver.dispose();
});

/** Marks an event as one that the browser made from real input. */
function trusted<Event extends globalThis.Event>(event: Event): Event {
  Object.defineProperty(event, "isTrusted", { value: true });
  return event;
}

/** A key event as the keyboard sends it to the window. */
function keyOnWindow(type: "keydown" | "keyup", code: string): void {
  window.dispatchEvent(trusted(new KeyboardEvent(type, { code, bubbles: true, cancelable: true })));
}

/** A pointer event at a client position, as the mouse sends it to the window. */
function pointerOnWindow(type: "pointerdown" | "pointerup" | "pointermove", x: number, y: number): void {
  window.dispatchEvent(trusted(new PointerEvent(type, { clientX: x, clientY: y, button: 0, bubbles: true, cancelable: true })));
}

describe("recording a page", () => {
  it("records trusted input in logical positions, and leaves out what the page did not get from the player", () => {
    // The stage is 960 by 540 client pixels at x = 100, so a logical pixel is half a client pixel.
    const stage = createStageMap({ area: () => ({ left: 100, top: 0, width: 960, height: 540 }) });
    const recorder = createRecorder({ game: "page-game", stage });
    stops.push(recorder.observe(window));
    keyOnWindow("keydown", "KeyW");
    keyOnWindow("keyup", "KeyW");
    pointerOnWindow("pointermove", 580, 270);
    pointerOnWindow("pointerdown", 580, 270);
    window.dispatchEvent(new KeyboardEvent("keydown", { code: "KeyQ", bubbles: true }));
    const inputs = recorder.toReplay().inputs;
    expect(inputs.map((input) => input.type)).toEqual(["key", "key", "pointer", "button"]);
    expect(inputs[2]).toMatchObject({ type: "pointer", x: 960, y: 540 });
    expect(inputs[3]).toMatchObject({ type: "button", button: "left", down: true, x: 960, y: 540 });
  });

  it("marks a press on a text field as skipped, since the field keeps its own keys", () => {
    const recorder = createRecorder({ game: "page-game" });
    stops.push(recorder.observe(window));
    const field = document.createElement("input");
    document.body.append(field);
    field.dispatchEvent(trusted(new KeyboardEvent("keydown", { code: "KeyA", bubbles: true, cancelable: true })));
    const [first] = recorder.toReplay().inputs;
    expect(first).toEqual({ frame: 0, type: "key", code: "KeyA", down: true, skipped: true });
    field.remove();
  });
});

describe("the page's frame takeover", () => {
  it("gives back the page's own requestAnimationFrame when the takeover is restored", () => {
    const original = window.requestAnimationFrame;
    const driver = createDriver();
    drivers.push(driver);
    const restore = driver.takeOverFrames();
    expect(window.requestAnimationFrame).not.toBe(original);
    restore();
    expect(window.requestAnimationFrame).toBe(original);
  });
});

describe("replaying a page session", () => {
  it("replays what a player did on the page, and reproduces the state at every checkpoint", async () => {
    const stage = createStageMap({ area: () => ({ left: 100, top: 0, width: 960, height: 540 }) });
    let live: FakeGame | null = null;
    const recorder: Recorder = createRecorder({
      game: "page-game",
      checkEvery: 5,
      seeds: { coins: 3 },
      stage,
      hash: () => (live === null ? "" : live.hash()),
    });
    stops.push(recorder.observe(window));

    const liveDriver = createDriver();
    drivers.push(liveDriver);
    // The live loop asks for frames through the driver, as a page's loop asks for animation frames; the
    // recorder wraps its update, so every frame and every pad state is recorded.
    live = startFakeGame(liveDriver, { seed: 3, wrap: (options) => recorder.wrap(options) });
    games.push(live);

    keyOnWindow("keydown", "Space");
    liveDriver.step(3);
    keyOnWindow("keyup", "Space");
    pointerOnWindow("pointerdown", 580, 270);
    liveDriver.step(4);
    pointerOnWindow("pointermove", 700, 300);
    pointerOnWindow("pointerup", 700, 300);
    liveDriver.step(6);
    keyOnWindow("keydown", "Space");
    liveDriver.step(10);

    const replay = recorder.toReplay();
    // 3 + 4 + 6 + 10 steps: the live loop's first step is its first update.
    expect(replay.frames).toBe(23);
    expect(replay.checkpoints.map((checkpoint) => checkpoint.frame)).toEqual([5, 10, 15, 20]);
    live.dispose();
    games.splice(0);
    stops.splice(0).forEach((stop) => stop());

    // The replay runs on a fresh driver whose target is the same window. It converts the logical positions
    // with the page's stage, so the game's map sees the same client positions as it did when recorded.
    const result = await checkReplay(
      replay,
      ({ driver, seeds }) => {
        const game = startFakeGame(driver, { seed: seeds.coins });
        games.push(game);
        return { hash: () => game.hash() };
      },
      { stage },
    );
    expect(result).toEqual({ ok: true, frames: 23, checkpoints: 4 });
  });
});
