// @vitest-environment happy-dom
// Tests for the notices: the queue by priority, the expiry and the fade, the repeated keys, the buttons, the
// history and its drawer, the anchored column, and the rule that the column never covers the rectangle to avoid.

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { createNotices, type Notices, type NoticesOptions } from "./notices";
import { setOffsetSize } from "./test-support";

let host: HTMLElement;

beforeEach(() => {
  vi.useFakeTimers();
  host = document.createElement("div");
  document.body.append(host);
});

afterEach(() => {
  vi.useRealTimers();
  document.body.innerHTML = "";
});

/** Notices in the host, with no fade and a short default time. */
function make(options: Omit<NoticesOptions, "host"> = {}): Notices {
  return createNotices({ host, fadeMs: 0, durationMs: 1000, ...options });
}

describe("the queue", () => {
  it("shows the message as plain text in the column", () => {
    const notices = make();
    notices.notify({ message: "<b>Hola</b>" });
    expect(notices.el.querySelector(".vu-notice-text")?.textContent).toBe("<b>Hola</b>");
    expect(notices.el.querySelector("b")).toBeNull();
    expect(notices.el.parentElement).toBe(host);
  });

  it("shows at most maxVisible notices, and a higher priority takes the place of the lowest one", () => {
    const notices = make({ maxVisible: 2 });
    const a = notices.notify({ message: "a" });
    const b = notices.notify({ message: "b" });
    const c = notices.notify({ message: "c", priority: 5 });
    expect(notices.visible()).toEqual([c, a]);
    expect(notices.el.querySelectorAll(".vu-notice")).toHaveLength(2);
    expect(b).not.toBe(c);
  });

  it("a notice leaves after its time on screen, and the next one takes its place", () => {
    const notices = make({ maxVisible: 1, durationMs: 1000 });
    const a = notices.notify({ message: "a" });
    const b = notices.notify({ message: "b" });
    expect(notices.visible()).toEqual([a]);
    vi.advanceTimersByTime(999);
    expect(notices.visible()).toEqual([a]);
    vi.advanceTimersByTime(1);
    expect(notices.visible()).toEqual([b]);
  });

  it("a sticky notice stays until it is dismissed", () => {
    const notices = make();
    const id = notices.notify({ message: "Guarda", durationMs: null });
    vi.advanceTimersByTime(60_000);
    expect(notices.visible()).toEqual([id]);
    notices.dismiss(id);
    expect(notices.visible()).toEqual([]);
  });

  it("fades a dismissed notice before it is removed from the column", () => {
    const notices = make({ fadeMs: 200 });
    const id = notices.notify({ message: "Adios" });
    notices.dismiss(id);
    expect(notices.visible()).toEqual([]);
    expect(notices.el.querySelector('[data-state="leaving"]')).not.toBeNull();
    vi.advanceTimersByTime(200);
    expect(notices.el.querySelector(".vu-notice")).toBeNull();
  });

  it("dismiss of an unknown id does nothing, and clear removes every notice but keeps the history", () => {
    const notices = make();
    notices.notify({ message: "uno" });
    notices.notify({ message: "dos" });
    notices.dismiss("notice-unknown");
    expect(notices.visible()).toHaveLength(2);
    notices.clear();
    expect(notices.visible()).toEqual([]);
    expect(notices.el.querySelectorAll(".vu-notice")).toHaveLength(0);
    expect(notices.history()).toHaveLength(2);
  });

  it("refuses a negative time on screen, and a maxVisible below one", () => {
    const notices = make();
    expect(() => notices.notify({ message: "x", durationMs: -5 })).toThrow(RangeError);
    expect(() => make({ maxVisible: 0 })).toThrow(RangeError);
    expect(() => make({ durationMs: -1 })).toThrow(RangeError);
  });
});

describe("repeated keys", () => {
  it("counts a notice with a key that is still on screen, instead of showing it again", () => {
    const notices = make();
    const first = notices.notify({ message: "Hungry", key: "hunger", tone: "warn" });
    const second = notices.notify({ message: "Hungry", key: "hunger", tone: "warn" });
    expect(second).toBe(first);
    expect(notices.visible()).toEqual([first]);
    const badge = notices.el.querySelector<HTMLElement>(".vu-notice-count");
    expect(badge?.textContent).toBe("×2");
    expect(badge?.hidden).toBe(false);
    expect(notices.history()).toEqual([expect.objectContaining({ id: first, count: 2, tone: "warn" })]);
  });

  it("gives a repeated notice its full time on screen again", () => {
    const notices = make({ durationMs: 1000 });
    const id = notices.notify({ message: "x", key: "k" });
    vi.advanceTimersByTime(800);
    notices.notify({ message: "x", key: "k" });
    vi.advanceTimersByTime(800);
    expect(notices.visible()).toEqual([id]);
    vi.advanceTimersByTime(200);
    expect(notices.visible()).toEqual([]);
  });

  it("shows a new notice with the same key once the old one has gone", () => {
    const notices = make({ durationMs: 500 });
    const first = notices.notify({ message: "x", key: "k" });
    vi.advanceTimersByTime(500);
    const second = notices.notify({ message: "x", key: "k" });
    expect(second).not.toBe(first);
    expect(notices.visible()).toEqual([second]);
  });
});

describe("buttons", () => {
  it("runs onAction with the chosen button's id, and then the notice goes away", () => {
    const notices = make();
    const onAction = vi.fn();
    notices.notify({
      message: "¿Aceptar?",
      actions: [
        { id: "yes", label: "Sí" },
        { id: "no", label: "No" },
      ],
      onAction,
      durationMs: null,
    });
    const yes = notices.el.querySelector<HTMLButtonElement>('[data-action="yes"]');
    expect(yes?.textContent).toBe("Sí");
    yes?.click();
    expect(onAction).toHaveBeenCalledWith("yes");
    expect(onAction).toHaveBeenCalledTimes(1);
    expect(notices.visible()).toEqual([]);
  });
});

describe("the history and its drawer", () => {
  it("lists the notices newest first, and keeps only the limit", () => {
    const notices = make({ historyLimit: 2 });
    notices.notify({ message: "uno" });
    notices.notify({ message: "dos" });
    const three = notices.notify({ message: "tres" });
    expect(notices.history().map((item) => item.message)).toEqual(["tres", "dos"]);
    expect(notices.history()[0]?.id).toBe(three);
  });

  it("opens with its close button focused, lists the notices, and closes on Escape with the focus back", () => {
    const notices = make();
    const trigger = document.createElement("button");
    document.body.append(trigger);
    trigger.focus();
    notices.notify({ message: "Uno" });
    notices.toggleHistory();
    expect(notices.historyOpen).toBe(true);
    const drawer = host.querySelector<HTMLElement>(".vu-history");
    expect(drawer?.hidden).toBe(false);
    expect(drawer?.querySelectorAll(".vu-history-item")).toHaveLength(1);
    expect(document.activeElement?.classList.contains("vu-button")).toBe(true);
    drawer?.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true, cancelable: true }));
    expect(notices.historyOpen).toBe(false);
    expect(document.activeElement).toBe(trigger);
  });

  it("says that nothing has come yet when the history is empty", () => {
    const notices = make();
    notices.openHistory();
    expect(host.querySelector(".vu-history-empty")?.textContent).toBe("Nothing yet.");
    notices.closeHistory();
    expect(notices.historyOpen).toBe(false);
  });

  it("writes its words with the lookup", () => {
    const notices = make({ t: (key) => (key === "notice.history.title" ? "Registro" : key) });
    notices.openHistory();
    expect(host.querySelector(".vu-history-title")?.textContent).toBe("Registro");
    expect(host.querySelector(".vu-history-empty")?.textContent).toBe("Nothing yet.");
  });
});

describe("placing the column", () => {
  it("stacks from the chosen corner, inside the margin", () => {
    const notices = make({ anchor: "top-left", margin: 20 });
    notices.notify({ message: "x" });
    expect(notices.el.style.left).toBe("20px");
    expect(notices.el.style.top).toBe("20px");
    notices.update({ anchor: "top-right" });
    setOffsetSize(notices.el, 300, 0);
    notices.relayout();
    expect(notices.el.style.left).toBe("1600px");
  });

  it("moves the column above a rectangle that it would cover, when the side above has more room", () => {
    const notices = make({ anchor: "bottom-right", avoid: { left: 1200, top: 900, width: 600, height: 150 } });
    notices.notify({ message: "Aviso" });
    setOffsetSize(notices.el, 400, 0);
    const node = notices.el.querySelector<HTMLElement>(".vu-notice");
    if (node === null) throw new Error("no notice");
    setOffsetSize(node, 400, 80);
    notices.relayout();
    expect(notices.el.style.left).toBe("1488px");
    expect(notices.el.style.top).toBe("804px");
    const bottom = 804 + 80;
    expect(bottom).toBeLessThanOrEqual(900);
  });

  it("keeps the notices that do not fit beside a rectangle waiting, and shows them when it moves", () => {
    const notices = make({ anchor: "bottom-right", maxVisible: 3 });
    const first = notices.notify({ message: "uno" });
    const second = notices.notify({ message: "dos" });
    setOffsetSize(notices.el, 300, 0);
    for (const node of notices.el.querySelectorAll<HTMLElement>(".vu-notice")) setOffsetSize(node, 300, 80);
    notices.update({ avoid: { left: 0, top: 200, width: 1920, height: 800 } });
    expect(notices.visible()).toEqual([first]);
    expect(second).not.toBe(first);
    notices.update({ avoid: null });
    expect(notices.visible()).toEqual([first, second]);
  });

  it("takes the rectangle from a function, when the props give one", () => {
    let rect: { left: number; top: number; width: number; height: number } | null = null;
    const notices = make({ avoid: () => rect });
    notices.notify({ message: "x" });
    setOffsetSize(notices.el, 400, 0);
    rect = { left: 1000, top: 0, width: 1000, height: 1080 };
    notices.relayout();
    expect(notices.el.style.left).toBe("1488px");
    expect(notices.visible()).toHaveLength(0);
  });
});

describe("cleanup", () => {
  it("removes the column and the drawer on destroy, and then ignores new notices", () => {
    const notices = make();
    notices.notify({ message: "x" });
    notices.destroy();
    expect(host.querySelector(".vu-notices")).toBeNull();
    expect(host.querySelector(".vu-history")).toBeNull();
    expect(notices.notify({ message: "y" })).toBe("");
  });
});
