// @vitest-environment happy-dom
// Tests for the dirty marks and their write helpers: each mark draws once per flush, one flush is asked for
// however many marks come in, marks made during a flush wait for the next one, and the helpers write nothing
// that is already there.

import { afterEach, describe, expect, it, vi } from "vitest";
import { createDirtyMarks, setActive, setShown, setText } from "./dirty";

afterEach(() => {
  document.body.innerHTML = "";
});

describe("createDirtyMarks", () => {
  it("draws a marked element once, and draws nothing when nothing is marked", () => {
    const marks = createDirtyMarks({ schedule: null });
    const render = vi.fn();
    marks.register("gold", render);
    expect(marks.flush()).toBe(1);
    expect(marks.flush()).toBe(0);
    marks.mark("gold");
    marks.mark("gold");
    expect(marks.flush()).toBe(1);
    expect(render).toHaveBeenCalledTimes(2);
  });

  it("draws the marked elements in the order they were registered", () => {
    const marks = createDirtyMarks({ schedule: null });
    const order: string[] = [];
    marks.register("a", () => order.push("a"));
    marks.register("b", () => order.push("b"));
    marks.register("c", () => order.push("c"));
    marks.flush();
    order.length = 0;
    marks.mark("c");
    marks.mark("a");
    expect(marks.flush()).toBe(2);
    expect(order).toEqual(["a", "c"]);
  });

  it("asks the scheduler for one flush however many marks come in", () => {
    const schedule = vi.fn<(flush: () => void) => void>();
    const marks = createDirtyMarks({ schedule });
    const render = vi.fn();
    marks.register("a", render);
    marks.register("b", render);
    marks.mark("a");
    marks.mark("b");
    expect(schedule).toHaveBeenCalledTimes(1);
    const scheduled = schedule.mock.calls[0]?.[0];
    scheduled?.();
    expect(render).toHaveBeenCalledTimes(2);
  });

  it("asks again for a flush after the scheduled one has run", () => {
    const schedule = vi.fn<(flush: () => void) => void>();
    const marks = createDirtyMarks({ schedule });
    marks.register("a", () => undefined);
    schedule.mock.calls[0]?.[0]();
    marks.mark("a");
    expect(schedule).toHaveBeenCalledTimes(2);
  });

  it("keeps a mark for a name that is not registered yet, and draws it when it is", () => {
    const marks = createDirtyMarks({ schedule: null });
    marks.mark("later");
    expect(marks.isMarked("later")).toBe(true);
    const render = vi.fn();
    marks.register("later", render);
    marks.flush();
    expect(render).toHaveBeenCalledTimes(1);
  });

  it("marks every element with markAll, and forgets one that is unregistered", () => {
    const marks = createDirtyMarks({ schedule: null });
    const first = vi.fn();
    const second = vi.fn();
    marks.register("first", first);
    marks.register("second", second);
    marks.flush();
    marks.markAll();
    expect(marks.flush()).toBe(2);
    marks.mark("first");
    marks.unregister("first");
    marks.flush();
    expect(first).toHaveBeenCalledTimes(2);
    expect(second).toHaveBeenCalledTimes(2);
  });

  it("a mark made inside a render draws at the next flush, not this one", () => {
    const marks = createDirtyMarks({ schedule: null });
    let rounds = 0;
    marks.register("chain", () => {
      rounds += 1;
      if (rounds === 1) marks.mark("chain");
    });
    expect(marks.flush()).toBe(1);
    expect(rounds).toBe(1);
    expect(marks.flush()).toBe(1);
    expect(rounds).toBe(2);
  });

  it("draws on the next animation frame with the default scheduler", async () => {
    const marks = createDirtyMarks();
    const render = vi.fn();
    marks.register("frame", render);
    expect(render).not.toHaveBeenCalled();
    await new Promise((resolve) => setTimeout(resolve, 60));
    expect(render).toHaveBeenCalledTimes(1);
    marks.destroy();
  });

  it("does nothing once it is destroyed", () => {
    const schedule = vi.fn<(flush: () => void) => void>();
    const marks = createDirtyMarks({ schedule });
    const render = vi.fn();
    marks.register("a", render);
    marks.destroy();
    marks.mark("a");
    expect(marks.flush()).toBe(0);
    expect(render).not.toHaveBeenCalled();
  });
});

describe("the write helpers", () => {
  it("setText writes only when the text is different", () => {
    const node = document.createElement("span");
    node.textContent = "70";
    const before = node.firstChild;
    expect(setText(node, "70")).toBe(false);
    expect(node.firstChild).toBe(before);
    expect(setText(node, "71")).toBe(true);
    expect(node.textContent).toBe("71");
  });

  it("setShown toggles the hidden attribute, and writes nothing when the state is already right", () => {
    const node = document.createElement("div");
    expect(setShown(node, false)).toBe(true);
    expect(node.hidden).toBe(true);
    expect(setShown(node, false)).toBe(false);
    expect(setShown(node, true)).toBe(true);
    expect(node.hidden).toBe(false);
  });

  it("setActive sets the is-active class and aria-pressed on a button", () => {
    const button = document.createElement("button");
    expect(setActive(button, true)).toBe(true);
    expect(button.classList.contains("is-active")).toBe(true);
    expect(button.getAttribute("aria-pressed")).toBe("true");
    expect(setActive(button, true)).toBe(false);
    expect(setActive(button, false)).toBe(true);
    expect(button.getAttribute("aria-pressed")).toBe("false");
  });

  it("setActive leaves aria-pressed off a plain element", () => {
    const node = document.createElement("div");
    setActive(node, true);
    expect(node.classList.contains("is-active")).toBe(true);
    expect(node.hasAttribute("aria-pressed")).toBe(false);
  });
});
