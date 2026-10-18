// @vitest-environment happy-dom
// Tests for the slider: the steps and the range, the arrow keys, Home and End, the pointer on the track, the
// ARIA values, the text that `format` gives, the disabled state, silent changes and the clean-up.

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { createSlider, type Slider, type SliderOptions } from "./slider";
import { installFakeGeometry, placeElement } from "../test-support";

beforeEach(() => {
  installFakeGeometry();
});

afterEach(() => {
  vi.restoreAllMocks();
  document.body.innerHTML = "";
});

/** A slider in the document, with the options given. */
function make(options: SliderOptions): Slider {
  const slider = createSlider(options);
  document.body.append(slider.el);
  return slider;
}

/** Presses a key on the slider, the way a keyboard does while it has the focus. */
function press(target: HTMLElement, key: string): KeyboardEvent {
  const event = new KeyboardEvent("keydown", { key, bubbles: true, cancelable: true });
  target.dispatchEvent(event);
  return event;
}

/** A pointer event of the primary pointer. */
function pointer(type: string, clientX: number): PointerEvent {
  return new PointerEvent(type, { bubbles: true, cancelable: true, clientX, clientY: 0, button: 0, pointerId: 1, isPrimary: true });
}

/** The track of a slider, which is where the pointer maps to a value. */
function trackOf(slider: Slider): HTMLElement {
  const track = slider.el.querySelector<HTMLElement>(".vu-slider-track");
  if (track === null) throw new Error("the slider has no track");
  return track;
}

describe("slider structure and ARIA", () => {
  it("is a focusable slider with its range, its value and the text of the value", () => {
    const slider = make({ label: "Volumen", min: 0, max: 10, value: 4, format: (value) => `${value} %` });
    expect(slider.el.getAttribute("role")).toBe("slider");
    expect(slider.el.tabIndex).toBe(0);
    expect(slider.el.getAttribute("aria-label")).toBe("Volumen");
    expect(slider.el.getAttribute("aria-valuemin")).toBe("0");
    expect(slider.el.getAttribute("aria-valuemax")).toBe("10");
    expect(slider.el.getAttribute("aria-valuenow")).toBe("4");
    expect(slider.el.getAttribute("aria-valuetext")).toBe("4 %");
    expect(slider.el.querySelector(".vu-slider-value")?.textContent).toBe("4 %");
  });

  it("shows the number as it is when there is no format", () => {
    const slider = make({ min: 0, max: 1, step: 0.5 });
    expect(slider.el.getAttribute("aria-valuetext")).toBe("0");
    expect(slider.el.hasAttribute("aria-label")).toBe(false);
  });

  it("gives its largest value as aria-valuemax, which is the last step not above max", () => {
    const slider = make({ min: 0, max: 10, step: 3 });
    expect(slider.el.getAttribute("aria-valuemax")).toBe("9");
    slider.setValue(100);
    expect(slider.value).toBe(9);
  });
});

describe("slider values", () => {
  it("snaps the starting value to the steps and keeps it in the range", () => {
    expect(make({ min: 0, max: 10, step: 3, value: 7 }).value).toBe(6);
    expect(make({ min: 0, max: 10, value: 100 }).value).toBe(10);
    expect(make({ min: 0, max: 10, value: -5 }).value).toBe(0);
    expect(make({ min: 0, max: 10 }).value).toBe(0);
  });

  it("keeps decimal steps clean, so that a sum of steps shows its own value", () => {
    const slider = make({ min: 0, max: 1, step: 0.1, value: 0.2 });
    press(slider.el, "ArrowRight");
    expect(slider.value).toBe(0.3);
    expect(slider.el.getAttribute("aria-valuenow")).toBe("0.3");
  });

  it("moves by one step with the arrows: Right and Up raise it, Left and Down lower it", () => {
    const slider = make({ min: 0, max: 10, value: 5 });
    press(slider.el, "ArrowRight");
    expect(slider.value).toBe(6);
    press(slider.el, "ArrowUp");
    expect(slider.value).toBe(7);
    press(slider.el, "ArrowLeft");
    expect(slider.value).toBe(6);
    press(slider.el, "ArrowDown");
    expect(slider.value).toBe(5);
  });

  it("goes to the ends with Home and End, and stays at them", () => {
    const slider = make({ min: 2, max: 10, step: 2, value: 6 });
    press(slider.el, "End");
    expect(slider.value).toBe(10);
    press(slider.el, "ArrowRight");
    expect(slider.value).toBe(10);
    press(slider.el, "Home");
    expect(slider.value).toBe(2);
  });

  it("takes the keys it uses, and leaves the others to the page", () => {
    const slider = make({ min: 0, max: 10, value: 5 });
    expect(press(slider.el, "Tab").defaultPrevented).toBe(false);
    expect(press(slider.el, "a").defaultPrevented).toBe(false);
    expect(slider.value).toBe(5);
    expect(press(slider.el, "ArrowRight").defaultPrevented).toBe(true);
  });

  it("leaves a key alone when another control has already prevented it", () => {
    const slider = make({ min: 0, max: 10, value: 5 });
    const event = new KeyboardEvent("keydown", { key: "ArrowRight", bubbles: true, cancelable: true });
    event.preventDefault();
    slider.el.dispatchEvent(event);
    expect(slider.value).toBe(5);
  });

  it("sets the value with setValue, clamped and snapped, and refuses a value that is not a number", () => {
    const slider = make({ min: 0, max: 10, step: 2 });
    // 5.2 is nearer to the step 6 than to 4.
    slider.setValue(5.2);
    expect(slider.value).toBe(6);
    expect(slider.el.getAttribute("aria-valuenow")).toBe("6");
    slider.setValue(-3);
    expect(slider.value).toBe(0);
    expect(() => slider.setValue(Number.NaN)).toThrow(RangeError);
    expect(() => slider.setValue(Number.POSITIVE_INFINITY)).toThrow(RangeError);
    expect(slider.value).toBe(0);
  });
});

describe("slider changes and silent sets", () => {
  it("calls onChange when a key changes the value, with the new value", () => {
    const onChange = vi.fn();
    const slider = make({ min: 0, max: 10, value: 5, onChange });
    press(slider.el, "ArrowRight");
    expect(onChange).toHaveBeenCalledWith(6);
  });

  it("does not call onChange for a silent setValue, or for a value that does not change", () => {
    const onChange = vi.fn();
    const slider = make({ min: 0, max: 10, value: 5, onChange });
    slider.setValue(8, { silent: true });
    expect(slider.value).toBe(8);
    expect(slider.el.getAttribute("aria-valuenow")).toBe("8");
    slider.setValue(8);
    expect(onChange).not.toHaveBeenCalled();
  });

  it("calls the onChange that the game sets after the slider is made", () => {
    const slider = make({ min: 0, max: 10, value: 5 });
    const seen: number[] = [];
    slider.onChange = (value) => seen.push(value);
    press(slider.el, "End");
    expect(seen).toEqual([10]);
  });
});

describe("slider pointer", () => {
  it("sets the value under the pointer on a press, and follows a drag until the release", () => {
    const onChange = vi.fn();
    const slider = make({ min: 0, max: 10, value: 0, onChange });
    placeElement(trackOf(slider), { left: 100, top: 0, width: 200, height: 14 });
    slider.el.dispatchEvent(pointer("pointerdown", 200));
    expect(slider.value).toBe(5);
    expect(document.activeElement).toBe(slider.el);
    slider.el.dispatchEvent(pointer("pointermove", 300));
    expect(slider.value).toBe(10);
    slider.el.dispatchEvent(pointer("pointerup", 300));
    slider.el.dispatchEvent(pointer("pointermove", 100));
    expect(slider.value).toBe(10);
    expect(onChange).toHaveBeenLastCalledWith(10);
  });

  it("clamps a press that is left of the track to the minimum, and right of it to the last step", () => {
    const slider = make({ min: 0, max: 10, value: 5 });
    placeElement(trackOf(slider), { left: 100, top: 0, width: 200, height: 14 });
    slider.el.dispatchEvent(pointer("pointerdown", 20));
    expect(slider.value).toBe(0);
    slider.el.dispatchEvent(pointer("pointerdown", 900));
    expect(slider.value).toBe(10);
  });

  it("ignores a press that is not the primary button, and a press when it is disabled", () => {
    const slider = make({ min: 0, max: 10, value: 5, disabled: true });
    placeElement(trackOf(slider), { left: 0, top: 0, width: 100, height: 14 });
    slider.el.dispatchEvent(pointer("pointerdown", 90));
    expect(slider.value).toBe(5);
    slider.setDisabled(false);
    const secondary = new PointerEvent("pointerdown", { bubbles: true, cancelable: true, clientX: 90, button: 2, pointerId: 2, isPrimary: true });
    slider.el.dispatchEvent(secondary);
    expect(slider.value).toBe(5);
  });
});

describe("slider disabled state and clean-up", () => {
  it("is out of the Tab order when disabled, and its keys do nothing until it is enabled again", () => {
    const slider = make({ min: 0, max: 10, value: 5 });
    slider.setDisabled(true);
    expect(slider.el.tabIndex).toBe(-1);
    expect(slider.el.getAttribute("aria-disabled")).toBe("true");
    press(slider.el, "ArrowRight");
    expect(slider.value).toBe(5);
    slider.setDisabled(false);
    expect(slider.el.tabIndex).toBe(0);
    expect(slider.el.hasAttribute("aria-disabled")).toBe(false);
    press(slider.el, "ArrowRight");
    expect(slider.value).toBe(6);
  });

  it("gives up the focus when it is disabled while it has it", () => {
    const slider = make({ min: 0, max: 10, value: 5 });
    slider.el.focus();
    slider.setDisabled(true);
    expect(slider.el.contains(document.activeElement)).toBe(false);
  });

  it("refuses a range that is not valid, with a RangeError", () => {
    expect(() => createSlider({ min: 5, max: 5 })).toThrow(RangeError);
    expect(() => createSlider({ min: 5, max: 1 })).toThrow(RangeError);
    expect(() => createSlider({ min: 0, max: 10, step: 0 })).toThrow(RangeError);
    expect(() => createSlider({ min: 0, max: 10, step: 11 })).toThrow(RangeError);
    expect(() => createSlider({ min: 0, max: Number.NaN })).toThrow(RangeError);
    expect(() => createSlider({ min: 0, max: 10, value: Number.NaN })).toThrow(RangeError);
  });

  it("removes the element, and its keys and pointer no longer reach it", () => {
    const onChange = vi.fn();
    const slider = make({ min: 0, max: 10, value: 5, onChange });
    const element = slider.el;
    element.focus();
    slider.destroy();
    expect(element.isConnected).toBe(false);
    expect(element.contains(document.activeElement)).toBe(false);
    press(element, "ArrowRight");
    expect(onChange).not.toHaveBeenCalled();
  });
});
