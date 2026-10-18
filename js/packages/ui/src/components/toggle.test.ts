// @vitest-environment happy-dom
// Tests for the toggle: the switch's role and state, Enter and Space, the click, a press that turns it once
// (the pad sends Enter, and a div does not click on it), silent sets, the disabled state and the clean-up.

import { afterEach, describe, expect, it, vi } from "vitest";
import { createToggle, type Toggle, type ToggleOptions } from "./toggle";

afterEach(() => {
  document.body.innerHTML = "";
});

/** A toggle in the document, with the options given. */
function make(options: ToggleOptions = {}): Toggle {
  const toggle = createToggle(options);
  document.body.append(toggle.el);
  return toggle;
}

/** Presses a key on the toggle, the way a keyboard does while it has the focus. */
function press(target: HTMLElement, key: string): KeyboardEvent {
  const event = new KeyboardEvent("keydown", { key, bubbles: true, cancelable: true });
  target.dispatchEvent(event);
  return event;
}

describe("toggle structure", () => {
  it("is a focusable switch whose state is aria-checked, and that has a name", () => {
    const toggle = make({ label: "Subtítulos" });
    expect(toggle.el.getAttribute("role")).toBe("switch");
    expect(toggle.el.tabIndex).toBe(0);
    expect(toggle.el.getAttribute("aria-checked")).toBe("false");
    expect(toggle.el.getAttribute("aria-label")).toBe("Subtítulos");
    expect(toggle.value).toBe(false);
  });

  it("starts on when it is given the value true", () => {
    const toggle = make({ value: true });
    expect(toggle.el.getAttribute("aria-checked")).toBe("true");
  });

  it("draws a knob that is hidden from the screen reader", () => {
    const toggle = make();
    expect(toggle.el.querySelector(".vu-toggle-knob")?.getAttribute("aria-hidden")).toBe("true");
  });
});

describe("toggle keys and clicks", () => {
  it("turns over with Enter and with Space, and calls onChange with the new value", () => {
    const onChange = vi.fn();
    const toggle = make({ onChange });
    press(toggle.el, "Enter");
    expect(toggle.value).toBe(true);
    expect(toggle.el.getAttribute("aria-checked")).toBe("true");
    press(toggle.el, " ");
    expect(toggle.value).toBe(false);
    expect(onChange.mock.calls).toEqual([[true], [false]]);
  });

  it("takes Enter and Space, and leaves the other keys to the page", () => {
    const toggle = make();
    expect(press(toggle.el, "a").defaultPrevented).toBe(false);
    expect(press(toggle.el, "Tab").defaultPrevented).toBe(false);
    expect(toggle.value).toBe(false);
    expect(press(toggle.el, "Enter").defaultPrevented).toBe(true);
  });

  it("turns over once for one Enter press, which is what a pad's A sends through startPadMenuKeys", () => {
    const onChange = vi.fn();
    const toggle = make({ onChange });
    // The pad sends a keydown, and then clicks the focused element only when it is a button. This switch is a div,
    // so the one keydown is the only thing that turns it.
    press(toggle.el, "Enter");
    expect(toggle.value).toBe(true);
    expect(onChange).toHaveBeenCalledTimes(1);
  });

  it("turns over on a click", () => {
    const toggle = make();
    toggle.el.click();
    expect(toggle.value).toBe(true);
    toggle.el.click();
    expect(toggle.value).toBe(false);
  });

  it("sets the value with setValue, and refuses a value that is not true or false", () => {
    const toggle = make();
    toggle.setValue(true);
    expect(toggle.value).toBe(true);
    expect(() => toggle.setValue("yes" as unknown as boolean)).toThrow(RangeError);
    expect(toggle.value).toBe(true);
    expect(() => createToggle({ value: 1 as unknown as boolean })).toThrow(RangeError);
  });

  it("does not call onChange for a silent setValue, or for a value that is already set", () => {
    const onChange = vi.fn();
    const toggle = make({ onChange });
    toggle.setValue(true, { silent: true });
    expect(toggle.el.getAttribute("aria-checked")).toBe("true");
    toggle.setValue(true);
    expect(onChange).not.toHaveBeenCalled();
  });

  it("leaves a key alone when another control has already prevented it", () => {
    const toggle = make();
    const event = new KeyboardEvent("keydown", { key: "Enter", bubbles: true, cancelable: true });
    event.preventDefault();
    toggle.el.dispatchEvent(event);
    expect(toggle.value).toBe(false);
  });
});

describe("toggle disabled state and clean-up", () => {
  it("does nothing on a key or a click while it is disabled, and is out of the Tab order", () => {
    const toggle = make({ disabled: true });
    expect(toggle.el.tabIndex).toBe(-1);
    expect(toggle.el.getAttribute("aria-disabled")).toBe("true");
    press(toggle.el, "Enter");
    toggle.el.click();
    expect(toggle.value).toBe(false);
    toggle.setDisabled(false);
    expect(toggle.el.tabIndex).toBe(0);
    toggle.el.click();
    expect(toggle.value).toBe(true);
  });

  it("gives up the focus when it is disabled while it has it", () => {
    const toggle = make();
    toggle.el.focus();
    toggle.setDisabled(true);
    expect(toggle.el.contains(document.activeElement)).toBe(false);
  });

  it("removes the element, and its keys and clicks no longer reach it", () => {
    const onChange = vi.fn();
    const toggle = make({ onChange });
    const element = toggle.el;
    toggle.destroy();
    expect(element.isConnected).toBe(false);
    press(element, "Enter");
    element.click();
    expect(onChange).not.toHaveBeenCalled();
  });
});
