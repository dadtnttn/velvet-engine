// @vitest-environment happy-dom
// Tests for the toast queue. Fake timers stand in for the clock, so every step of the timing is
// checked exactly: how long a notice stays, how long its fade takes, and when the next one shows.

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { createToast } from "./toast";

beforeEach(() => {
  vi.useFakeTimers();
});

afterEach(() => {
  vi.useRealTimers();
  vi.unstubAllGlobals();
  document.body.innerHTML = "";
});

/** The text of the notices on screen, in order. */
function shown(root: HTMLElement): string[] {
  return Array.from(root.querySelectorAll(".vu-toast")).map((node) => node.textContent ?? "");
}

/** True when every notice on screen is fading. */
function allLeaving(root: HTMLElement): boolean {
  const notices = Array.from(root.querySelectorAll(".vu-toast"));
  return notices.length > 0 && notices.every((node) => node.classList.contains("is-leaving"));
}

describe("toast queue", () => {
  it("shows at most maxVisible notices and queues the rest", () => {
    const toast = createToast({ maxVisible: 2, duration: 1000, fadeMs: 200 });
    document.body.append(toast.el);
    toast.show("A");
    toast.show("B");
    toast.show("C");
    expect(shown(toast.el)).toEqual(["A", "B"]);
  });

  it("fades a notice after its duration, and admits the next one when the fade ends", () => {
    const toast = createToast({ maxVisible: 2, duration: 1000, fadeMs: 200 });
    document.body.append(toast.el);
    toast.show("A");
    toast.show("B");
    toast.show("C");

    vi.advanceTimersByTime(999);
    expect(allLeaving(toast.el)).toBe(false);

    vi.advanceTimersByTime(1);
    expect(allLeaving(toast.el)).toBe(true);
    expect(shown(toast.el)).toEqual(["A", "B"]);

    vi.advanceTimersByTime(199);
    expect(shown(toast.el)).toEqual(["A", "B"]);

    vi.advanceTimersByTime(1);
    expect(shown(toast.el)).toEqual(["C"]);

    // C had its own full duration from the moment it appeared.
    vi.advanceTimersByTime(1000);
    expect(allLeaving(toast.el)).toBe(true);
    vi.advanceTimersByTime(200);
    expect(shown(toast.el)).toEqual([]);
  });

  it("removes a notice at once when the fade time is zero", () => {
    const toast = createToast({ maxVisible: 1, duration: 500, fadeMs: 0 });
    document.body.append(toast.el);
    toast.show("A");
    toast.show("B");
    vi.advanceTimersByTime(500);
    expect(shown(toast.el)).toEqual(["B"]);
  });

  it("removes a notice without a fade when the player prefers reduced motion", () => {
    vi.stubGlobal("matchMedia", () => ({ matches: true }));
    const toast = createToast({ maxVisible: 1, duration: 500, fadeMs: 300 });
    document.body.append(toast.el);
    toast.show("A");
    toast.show("B");
    vi.advanceTimersByTime(500);
    expect(shown(toast.el)).toEqual(["B"]);
    expect(allLeaving(toast.el)).toBe(false);
  });

  it("gives each notice its tone", () => {
    const toast = createToast();
    document.body.append(toast.el);
    toast.show("Cuidado", "warn");
    toast.show("Error", "danger");
    const tones = Array.from(toast.el.querySelectorAll(".vu-toast")).map((node) => node.getAttribute("data-tone"));
    expect(tones).toEqual(["warn", "danger"]);
  });

  it("applies new settings to the notices that show from now on", () => {
    const toast = createToast({ maxVisible: 1 });
    document.body.append(toast.el);
    toast.update({ maxVisible: 3 });
    toast.show("A");
    toast.show("B");
    toast.show("C");
    expect(shown(toast.el)).toEqual(["A", "B", "C"]);
  });

  it("starts queued notices when the limit goes up", () => {
    const toast = createToast({ maxVisible: 1 });
    document.body.append(toast.el);
    toast.show("A");
    toast.show("B");
    expect(shown(toast.el)).toEqual(["A"]);
    toast.update({ maxVisible: 2 });
    expect(shown(toast.el)).toEqual(["A", "B"]);
  });

  it("rejects settings that cannot work", () => {
    expect(() => createToast({ maxVisible: 0 })).toThrow(RangeError);
    expect(() => createToast({ maxVisible: 1.5 })).toThrow(RangeError);
    expect(() => createToast({ duration: -1 })).toThrow(RangeError);
    const toast = createToast({ maxVisible: 2 });
    expect(() => toast.update({ fadeMs: Number.NaN })).toThrow(RangeError);
  });

  it("clears its timers and drops the queue on destroy", () => {
    const toast = createToast({ maxVisible: 1, duration: 1000 });
    document.body.append(toast.el);
    toast.show("A");
    toast.show("B");
    toast.destroy();
    expect(vi.getTimerCount()).toBe(0);
    expect(toast.el.isConnected).toBe(false);
    toast.show("C");
    vi.advanceTimersByTime(5000);
    expect(vi.getTimerCount()).toBe(0);
  });
});
