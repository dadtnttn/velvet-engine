// @vitest-environment happy-dom
// Tests for capturing the next input: keys and mouse buttons on the page (the capture phase, so the
// key does not reach the game), gamepads polled on their own frames, the cancel keys, the timeout, the
// abort signal, a second capture that replaces the first, and the handled events that are ignored.

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ActionMap, type Binding } from "./input";
import { markHandled } from "./input-controls";
import { FakePads, fakePad, manualScheduler, type ManualScheduler } from "./input-test-support";

type Action = "game" | "menu";

const pads = new FakePads();
let scheduler: ManualScheduler;

beforeEach(() => {
  pads.set([]);
  pads.install();
  scheduler = manualScheduler();
});

afterEach(() => {
  vi.unstubAllGlobals();
  vi.useRealTimers();
  document.body.replaceChildren();
});

/** A key press on the body, as the browser sends it: it bubbles up through the window. */
function keyDown(code: string, repeat = false): KeyboardEvent {
  return new KeyboardEvent("keydown", { code, key: code, bubbles: true, cancelable: true, repeat });
}

/** A mouse button press on the body, as the browser sends it. */
function pointerDown(button: number): PointerEvent {
  return new PointerEvent("pointerdown", { button, bubbles: true, cancelable: true });
}

/** True while the promise has not settled. It settles only in a later microtask, so it is read at once. */
function isPending(promise: Promise<unknown>): Promise<boolean> {
  return Promise.race([promise.then(() => false), Promise.resolve(true)]);
}

describe("capturing a key", () => {
  it("takes the next key press, and stops it from reaching the page and the game's map", async () => {
    const page = vi.fn();
    window.addEventListener("keydown", page);
    const game = new ActionMap<Action>([{ action: "game", control: { type: "key", key: "KeyQ" } }]);
    game.attach(window);
    const map = new ActionMap<Action>();

    const capture = map.captureNext({ devices: ["keyboard"], scheduler, timeoutMs: Infinity });
    document.body.dispatchEvent(keyDown("KeyQ"));

    await expect(capture).resolves.toEqual({ device: "keyboard", control: { type: "key", key: "KeyQ" } });
    expect(page).not.toHaveBeenCalled();
    expect(game.held("game")).toBe(false);
    window.removeEventListener("keydown", page);
    game.detach();
  });

  it("ignores an auto-repeated key, and waits for a new press", async () => {
    const map = new ActionMap<Action>();
    const capture = map.captureNext({ devices: ["keyboard"], scheduler, timeoutMs: Infinity });
    document.body.dispatchEvent(keyDown("KeyW", true));
    expect(await isPending(capture)).toBe(true);
    document.body.dispatchEvent(keyDown("KeyR"));
    await expect(capture).resolves.toEqual({ device: "keyboard", control: { type: "key", key: "KeyR" } });
  });

  it("cancels on Escape by default, and the Escape does not reach the page", async () => {
    const page = vi.fn();
    window.addEventListener("keydown", page);
    const map = new ActionMap<Action>();
    const capture = map.captureNext({ devices: ["keyboard"], scheduler, timeoutMs: Infinity });
    document.body.dispatchEvent(keyDown("Escape"));
    await expect(capture).resolves.toBeNull();
    expect(page).not.toHaveBeenCalled();
    window.removeEventListener("keydown", page);
  });

  it("cancels on the cancel keys that are given, and captures other keys, Escape included", async () => {
    const map = new ActionMap<Action>();
    const captured = map.captureNext({ devices: ["keyboard"], scheduler, cancelKeys: ["F1"], timeoutMs: Infinity });
    document.body.dispatchEvent(keyDown("Escape"));
    await expect(captured).resolves.toEqual({ device: "keyboard", control: { type: "key", key: "Escape" } });

    const cancelled = map.captureNext({ devices: ["keyboard"], scheduler, cancelKeys: ["F1"], timeoutMs: Infinity });
    document.body.dispatchEvent(keyDown("F1"));
    await expect(cancelled).resolves.toBeNull();
  });

  it("does not take a key that the UI has marked as handled", async () => {
    const map = new ActionMap<Action>();
    const capture = map.captureNext({ devices: ["keyboard"], scheduler, timeoutMs: Infinity });
    const handled = keyDown("KeyJ");
    markHandled(handled);
    document.body.dispatchEvent(handled);
    expect(await isPending(capture)).toBe(true);
    map.cancelCapture();
    await expect(capture).resolves.toBeNull();
  });
});

describe("capturing a mouse button", () => {
  it("takes the button by name, and ignores buttons that have no name", async () => {
    const map = new ActionMap<Action>();
    const capture = map.captureNext({ devices: ["mouse"], scheduler, timeoutMs: Infinity });
    document.body.dispatchEvent(pointerDown(3));
    expect(await isPending(capture)).toBe(true);
    document.body.dispatchEvent(pointerDown(2));
    await expect(capture).resolves.toEqual({ device: "mouse", control: { type: "mouse", button: "right" } });
  });

  it("does not take keys when only the mouse may answer", async () => {
    const map = new ActionMap<Action>();
    const capture = map.captureNext({ devices: ["mouse"], scheduler, timeoutMs: Infinity });
    document.body.dispatchEvent(keyDown("KeyE"));
    expect(await isPending(capture)).toBe(true);
    map.cancelCapture();
    await expect(capture).resolves.toBeNull();
  });
});

describe("capturing a gamepad input", () => {
  it("takes a button that goes from released to pressed", async () => {
    pads.set([fakePad(0)]);
    const map = new ActionMap<Action>();
    const capture = map.captureNext({ devices: ["gamepad"], scheduler, timeoutMs: Infinity });
    pads.set([fakePad(0, { buttons: { 1: 1 } })]);
    scheduler.run();
    await expect(capture).resolves.toEqual({ device: "gamepad", control: { type: "pad-button", button: "b" } });
  });

  it("does not take a button that was held when the capture started, until it is released and pressed again", async () => {
    pads.set([fakePad(0, { buttons: { 0: 1 } })]);
    const map = new ActionMap<Action>();
    const capture = map.captureNext({ devices: ["gamepad"], scheduler, timeoutMs: Infinity });
    scheduler.run();
    expect(await isPending(capture)).toBe(true);

    pads.set([fakePad(0)]);
    scheduler.run();
    expect(await isPending(capture)).toBe(true);

    pads.set([fakePad(0, { buttons: { 0: 1 } })]);
    scheduler.run();
    await expect(capture).resolves.toEqual({ device: "gamepad", control: { type: "pad-button", button: "a" } });
  });

  it("takes a trigger once it is pulled past half way", async () => {
    pads.set([fakePad(0)]);
    const map = new ActionMap<Action>();
    const capture = map.captureNext({ devices: ["gamepad"], scheduler, timeoutMs: Infinity });
    pads.set([fakePad(0, { buttons: { 6: 0.3 } })]);
    scheduler.run();
    expect(await isPending(capture)).toBe(true);
    pads.set([fakePad(0, { buttons: { 6: 0.9 } })]);
    scheduler.run();
    await expect(capture).resolves.toEqual({ device: "gamepad", control: { type: "pad-button", button: "leftTrigger" } });
  });

  it("takes a stick pushed from rest, with the sign of the push", async () => {
    pads.set([fakePad(0)]);
    const map = new ActionMap<Action>();
    const capture = map.captureNext({ devices: ["gamepad"], scheduler, timeoutMs: Infinity });
    pads.set([fakePad(0, { axes: [0, -0.9, 0, 0] })]);
    scheduler.run();
    await expect(capture).resolves.toEqual({
      device: "gamepad",
      control: { type: "pad-axis", axis: "leftY" },
      sign: -1,
    });
  });

  it("does not take a stick that was already pushed when the capture started", async () => {
    pads.set([fakePad(0, { axes: [0, 0, 0.9, 0] })]);
    const map = new ActionMap<Action>();
    const capture = map.captureNext({ devices: ["gamepad"], scheduler, timeoutMs: Infinity });
    scheduler.run();
    expect(await isPending(capture)).toBe(true);
    pads.set([fakePad(0)]);
    scheduler.run();
    pads.set([fakePad(0, { axes: [0, 0, 0.9, 0] })]);
    scheduler.run();
    await expect(capture).resolves.toEqual({
      device: "gamepad",
      control: { type: "pad-axis", axis: "rightX" },
      sign: 1,
    });
  });

  it("asks for a frame at a time, and stops asking when it ends", async () => {
    const map = new ActionMap<Action>();
    const capture = map.captureNext({ devices: ["gamepad"], scheduler, timeoutMs: Infinity });
    expect(scheduler.pending).toBe(1);
    scheduler.run();
    expect(scheduler.pending).toBe(1);
    map.cancelCapture();
    expect(scheduler.pending).toBe(0);
    await expect(capture).resolves.toBeNull();
  });
});

describe("ending a capture", () => {
  it("gives up after the timeout, and resolves with null", async () => {
    vi.useFakeTimers();
    const map = new ActionMap<Action>();
    const capture = map.captureNext({ devices: ["keyboard"], scheduler, timeoutMs: 1000 });
    vi.advanceTimersByTime(999);
    expect(await isPending(capture)).toBe(true);
    vi.advanceTimersByTime(1);
    await expect(capture).resolves.toBeNull();
  });

  it("ends when the signal is aborted, and at once when it was aborted already", async () => {
    const map = new ActionMap<Action>();
    const controller = new AbortController();
    const capture = map.captureNext({ devices: ["keyboard"], scheduler, timeoutMs: Infinity, signal: controller.signal });
    controller.abort();
    await expect(capture).resolves.toBeNull();

    const aborted = new AbortController();
    aborted.abort();
    await expect(map.captureNext({ scheduler, signal: aborted.signal })).resolves.toBeNull();
  });

  it("is replaced by a second capture, which then takes the next input", async () => {
    const map = new ActionMap<Action>();
    const first = map.captureNext({ devices: ["keyboard"], scheduler, timeoutMs: Infinity });
    const second = map.captureNext({ devices: ["keyboard"], scheduler, timeoutMs: Infinity });
    await expect(first).resolves.toBeNull();
    document.body.dispatchEvent(keyDown("KeyE"));
    await expect(second).resolves.toEqual({ device: "keyboard", control: { type: "key", key: "KeyE" } });
  });

  it("is cancelled by cancelCapture, and cancelCapture does nothing when none runs", async () => {
    const map = new ActionMap<Action>();
    map.cancelCapture();
    const capture = map.captureNext({ devices: ["keyboard"], scheduler, timeoutMs: Infinity });
    map.cancelCapture();
    await expect(capture).resolves.toBeNull();
  });

  it("rejects a timeout that is not above zero", () => {
    const map = new ActionMap<Action>();
    expect(() => map.captureNext({ timeoutMs: 0, scheduler })).toThrow(RangeError);
    expect(() => map.captureNext({ timeoutMs: Number.NaN, scheduler })).toThrow(RangeError);
  });
});

describe("the map a capture belongs to", () => {
  it("keeps the bindings of the map as they were", async () => {
    const bindings: Binding<Action>[] = [{ action: "game", control: { type: "key", key: "KeyQ" } }];
    const map = new ActionMap<Action>(bindings);
    const capture = map.captureNext({ devices: ["keyboard"], scheduler, timeoutMs: Infinity });
    document.body.dispatchEvent(keyDown("KeyX"));
    await capture;
    expect(map.slotsOf("game")[0]?.control).toEqual({ type: "key", key: "KeyQ" });
  });
});
