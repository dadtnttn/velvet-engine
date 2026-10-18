// @vitest-environment happy-dom
// Tests for the stat bar: it is a meter with the label and the range, its fill is a scaleX transform of the
// value, values outside the range are drawn at the ends, the text shows only when it is given, and bad props
// are refused.

import { afterEach, describe, expect, it } from "vitest";
import { createBar } from "./bar";

afterEach(() => {
  document.body.innerHTML = "";
});

/** The fill element of a bar. */
function fillOf(el: HTMLElement): HTMLElement {
  const fill = el.querySelector<HTMLElement>(".vu-bar-fill");
  if (fill === null) throw new Error("the bar has no fill");
  return fill;
}

describe("createBar", () => {
  it("is a meter named by its label, with the range and the value", () => {
    const bar = createBar({ label: "Hunger", value: 70, max: 100 });
    expect(bar.el.getAttribute("role")).toBe("meter");
    expect(bar.el.getAttribute("aria-label")).toBe("Hunger");
    expect(bar.el.getAttribute("aria-valuemin")).toBe("0");
    expect(bar.el.getAttribute("aria-valuemax")).toBe("100");
    expect(bar.el.getAttribute("aria-valuenow")).toBe("70");
  });

  it("draws the fill as a scaleX transform of the ratio, so no layout changes", () => {
    const bar = createBar({ label: "Loyalty", value: 35, max: 50 });
    document.body.append(bar.el);
    expect(fillOf(bar.el).style.transform).toBe("scaleX(0.7)");
    expect(fillOf(bar.el).style.width).toBe("");
  });

  it("draws a value below zero empty and a value above the maximum full", () => {
    const bar = createBar({ label: "Gauge", value: -5, max: 10 });
    expect(fillOf(bar.el).style.transform).toBe("scaleX(0)");
    expect(bar.el.getAttribute("aria-valuenow")).toBe("0");
    bar.update({ value: 99 });
    expect(fillOf(bar.el).style.transform).toBe("scaleX(1)");
    expect(bar.el.getAttribute("aria-valuenow")).toBe("10");
  });

  it("uses 100 as the maximum when none is given", () => {
    const bar = createBar({ label: "Health", value: 25 });
    expect(bar.el.getAttribute("aria-valuemax")).toBe("100");
    expect(fillOf(bar.el).style.transform).toBe("scaleX(0.25)");
  });

  it("shows its text over the fill, and hides the text element when the text is null or empty", () => {
    const bar = createBar({ label: "Energy", value: 3, max: 4, text: "3 / 4" });
    const text = bar.el.querySelector<HTMLElement>(".vu-bar-text");
    expect(text?.textContent).toBe("3 / 4");
    expect(text?.hidden).toBe(false);
    bar.update({ text: null });
    expect(text?.hidden).toBe(true);
    expect(text?.textContent).toBe("");
  });

  it("sets its tone as data-tone, with accent as the default", () => {
    const bar = createBar({ label: "Mood", value: 1 });
    expect(bar.el.getAttribute("data-tone")).toBe("accent");
    bar.update({ tone: "danger" });
    expect(bar.el.getAttribute("data-tone")).toBe("danger");
  });

  it("update keeps the props it is not given", () => {
    const bar = createBar({ label: "Trust", value: 10, max: 20, tone: "gold" });
    bar.update({ value: 15 });
    expect(bar.el.getAttribute("aria-label")).toBe("Trust");
    expect(bar.el.getAttribute("aria-valuemax")).toBe("20");
    expect(bar.el.getAttribute("data-tone")).toBe("gold");
    expect(fillOf(bar.el).style.transform).toBe("scaleX(0.75)");
  });

  it("refuses a value that is not finite, and a maximum that is not finite", () => {
    expect(() => createBar({ label: "Bad", value: Number.NaN })).toThrow(RangeError);
    expect(() => createBar({ label: "Bad", value: 1, max: Number.NaN })).toThrow(RangeError);
    expect(() => createBar({ label: "Bad", value: 1, max: Number.POSITIVE_INFINITY })).toThrow(RangeError);
    const bar = createBar({ label: "Good", value: 1, max: 4 });
    expect(() => bar.update({ max: Number.NEGATIVE_INFINITY })).toThrow(RangeError);
    expect(() => bar.update({ value: Number.NaN })).toThrow(RangeError);
    expect(bar.el.getAttribute("aria-valuemax")).toBe("4");
    expect(bar.el.getAttribute("aria-valuenow")).toBe("1");
  });

  it("draws an empty bar when the maximum is zero or less, and shows its text as given", () => {
    const bar = createBar({ label: "Workers", value: 3, max: 0, text: "0 / 0" });
    document.body.append(bar.el);
    expect(fillOf(bar.el).style.transform).toBe("scaleX(0)");
    expect(bar.el.getAttribute("aria-valuemax")).toBe("0");
    expect(bar.el.getAttribute("aria-valuenow")).toBe("0");
    expect(bar.el.querySelector(".vu-bar-text")?.textContent).toBe("0 / 0");
    bar.update({ max: -4, value: 1 });
    expect(fillOf(bar.el).style.transform).toBe("scaleX(0)");
    expect(bar.el.getAttribute("aria-valuemax")).toBe("0");
    bar.update({ max: 2, value: 1 });
    expect(fillOf(bar.el).style.transform).toBe("scaleX(0.5)");
  });

  it("removes its element on destroy", () => {
    const bar = createBar({ label: "Gone", value: 1 });
    document.body.append(bar.el);
    bar.destroy();
    expect(bar.el.isConnected).toBe(false);
  });
});
