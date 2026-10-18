// @vitest-environment happy-dom
// Tests for the gamepad menu keys: the d-pad and the left stick press the arrows, A presses Enter and
// clicks a focused button, B presses Escape, a held arrow repeats, and stopping releases what is held.
// The keys reach the focused element through the page's own bubbling, and each one is marked handled.

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { isHandled } from "./input-controls";
import { startPadMenuKeys, type PadKeyMapping } from "./pad-keys";
import { FakePads, fakePad, manualScheduler, type ManualScheduler } from "./input-test-support";

const pads = new FakePads();
let scheduler: ManualScheduler;

/** The key events that the page saw, in order: the type, the key, whether it repeated, and whether it was marked. */
interface Seen {
  readonly type: string;
  readonly key: string;
  readonly repeat: boolean;
  readonly handled: boolean;
}

let seen: Seen[] = [];

/** Records every key event on the page. */
function record(event: Event): void {
  const key = event as KeyboardEvent;
  seen.push({ type: key.type, key: key.key, repeat: key.repeat, handled: isHandled(event) });
}

beforeEach(() => {
  pads.set([]);
  pads.install();
  scheduler = manualScheduler();
  seen = [];
  document.addEventListener("keydown", record);
  document.addEventListener("keyup", record);
  const box = document.createElement("div");
  box.tabIndex = 0;
  document.body.append(box);
  box.focus();
});

afterEach(() => {
  document.removeEventListener("keydown", record);
  document.removeEventListener("keyup", record);
  vi.unstubAllGlobals();
  document.body.replaceChildren();
});

describe("forwarding the pad", () => {
  it("presses an arrow when the d-pad goes down, and releases it when the d-pad comes back up", () => {
    const stop = startPadMenuKeys({ scheduler });
    pads.set([fakePad(0, { buttons: { 13: 1 } })]);
    scheduler.run(0);
    pads.set([fakePad(0)]);
    scheduler.run(50);
    stop();
    expect(seen).toEqual([
      { type: "keydown", key: "ArrowDown", repeat: false, handled: true },
      { type: "keyup", key: "ArrowDown", repeat: false, handled: true },
    ]);
  });

  it("presses an arrow when the left stick is pushed that way", () => {
    const stop = startPadMenuKeys({ scheduler });
    pads.set([fakePad(0, { axes: [0, -0.9, 0, 0] })]);
    scheduler.run(0);
    stop();
    expect(seen[0]).toMatchObject({ type: "keydown", key: "ArrowUp" });
  });

  it("presses Enter with A, and clicks a focused button, which a synthetic Enter does not do", () => {
    const button = document.createElement("button");
    document.body.append(button);
    button.focus();
    const clicks = vi.fn();
    button.addEventListener("click", clicks);
    const stop = startPadMenuKeys({ scheduler });
    pads.set([fakePad(0, { buttons: { 0: 1 } })]);
    scheduler.run(0);
    stop();
    expect(seen[0]).toMatchObject({ type: "keydown", key: "Enter" });
    expect(clicks).toHaveBeenCalledTimes(1);
  });

  it("presses Escape with B", () => {
    const stop = startPadMenuKeys({ scheduler });
    pads.set([fakePad(0, { buttons: { 1: 1 } })]);
    scheduler.run(0);
    stop();
    expect(seen[0]).toMatchObject({ type: "keydown", key: "Escape" });
  });

  it("repeats a held arrow after the delay, at the repeat interval, and not for other keys", () => {
    const stop = startPadMenuKeys({ scheduler });
    pads.set([fakePad(0, { buttons: { 12: 1 } })]);
    scheduler.run(0);
    for (const time of [100, 200, 300, 400, 450, 500]) scheduler.run(time);
    stop();
    const repeats = seen.filter((event) => event.repeat).length;
    expect(repeats).toBe(2);
    expect(seen.filter((event) => event.type === "keydown" && event.key === "ArrowUp")).toHaveLength(3);
  });

  it("does not repeat Enter while it is held", () => {
    const stop = startPadMenuKeys({ scheduler });
    pads.set([fakePad(0, { buttons: { 0: 1 } })]);
    scheduler.run(0);
    for (const time of [400, 500, 600]) scheduler.run(time);
    stop();
    expect(seen.filter((event) => event.repeat)).toEqual([]);
  });

  it("releases the keys that are still held when it stops, and asks for no more frames", () => {
    const stop = startPadMenuKeys({ scheduler });
    pads.set([fakePad(0, { buttons: { 13: 1 } })]);
    scheduler.run(0);
    expect(scheduler.pending).toBe(1);
    stop();
    expect(scheduler.pending).toBe(0);
    expect(seen.at(-1)).toMatchObject({ type: "keyup", key: "ArrowDown" });
  });

  it("uses the mappings that are passed in instead of the defaults", () => {
    const mappings: PadKeyMapping[] = [{ control: { type: "pad-button", button: "x" }, key: "KeyK" }];
    const stop = startPadMenuKeys({ scheduler, mappings });
    pads.set([fakePad(0, { buttons: { 12: 1, 2: 1 } })]);
    scheduler.run(0);
    stop();
    expect(seen).toEqual([
      { type: "keydown", key: "KeyK", repeat: false, handled: true },
      { type: "keyup", key: "KeyK", repeat: false, handled: true },
    ]);
  });

  it("sends the keys to the target the caller names", () => {
    const other = document.createElement("div");
    document.body.append(other);
    const received: string[] = [];
    other.addEventListener("keydown", (event) => received.push(event.key));
    const stop = startPadMenuKeys({ scheduler, target: () => other });
    pads.set([fakePad(0, { buttons: { 15: 1 } })]);
    scheduler.run(0);
    stop();
    expect(received).toEqual(["ArrowRight"]);
  });
});
