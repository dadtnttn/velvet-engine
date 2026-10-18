// @vitest-environment happy-dom
// Tests for the choice: the listbox and its active option, the arrow keys with and without wrapping, Home and
// End, the arrows for the mouse, values by name, the validation of the options, the disabled state and the clean-up.

import { afterEach, describe, expect, it, vi } from "vitest";
import { createChoice, type Choice, type ChoiceOption, type ChoiceOptions } from "./choice";

const LANGUAGES: readonly ChoiceOption[] = [
  { value: "es", label: "Español" },
  { value: "en", label: "English" },
  { value: "fr", label: "Français" },
];

afterEach(() => {
  document.body.innerHTML = "";
});

/** A choice in the document, with the options given. Its options default to the three languages. */
function make(options: Partial<ChoiceOptions> = {}): Choice {
  const choice = createChoice({ options: LANGUAGES, ...options });
  document.body.append(choice.el);
  return choice;
}

/** Presses a key on the choice, the way a keyboard does while it has the focus. */
function press(target: HTMLElement, key: string): KeyboardEvent {
  const event = new KeyboardEvent("keydown", { key, bubbles: true, cancelable: true });
  target.dispatchEvent(event);
  return event;
}

/** The text of the option that is shown. */
function shownText(choice: Choice): string {
  return choice.el.querySelector(".vu-choice-value")?.textContent ?? "";
}

describe("choice structure", () => {
  it("is a focusable listbox, whose active option shows the value and its place in the list", () => {
    const choice = make({ label: "Idioma", value: "en" });
    expect(choice.el.getAttribute("role")).toBe("listbox");
    expect(choice.el.tabIndex).toBe(0);
    expect(choice.el.getAttribute("aria-label")).toBe("Idioma");
    const option = choice.el.querySelector<HTMLElement>('[role="option"]');
    expect(option?.textContent).toBe("English");
    expect(option?.getAttribute("aria-selected")).toBe("true");
    expect(option?.getAttribute("aria-posinset")).toBe("2");
    expect(option?.getAttribute("aria-setsize")).toBe("3");
    expect(choice.el.getAttribute("aria-activedescendant")).toBe(option?.id);
    expect(choice.value).toBe("en");
  });

  it("starts on the first option when no value is given, and draws two arrows hidden from the screen reader", () => {
    const choice = make();
    expect(choice.value).toBe("es");
    expect(shownText(choice)).toBe("Español");
    const arrows = choice.el.querySelectorAll('[aria-hidden="true"]');
    expect(arrows).toHaveLength(2);
  });
});

describe("choice keys", () => {
  it("moves to the next option with Right and Down, and to the previous one with Left and Up", () => {
    const onChange = vi.fn();
    const choice = make({ onChange });
    press(choice.el, "ArrowRight");
    expect(choice.value).toBe("en");
    press(choice.el, "ArrowDown");
    expect(choice.value).toBe("fr");
    press(choice.el, "ArrowLeft");
    expect(choice.value).toBe("en");
    press(choice.el, "ArrowUp");
    expect(choice.value).toBe("es");
    expect(onChange.mock.calls).toEqual([["en"], ["fr"], ["en"], ["es"]]);
  });

  it("goes round the ends when it wraps, which is the default", () => {
    const choice = make({ value: "fr" });
    press(choice.el, "ArrowRight");
    expect(choice.value).toBe("es");
    press(choice.el, "ArrowLeft");
    expect(choice.value).toBe("fr");
  });

  it("stops at the ends when it does not wrap, and still takes the key", () => {
    const onChange = vi.fn();
    const choice = make({ value: "fr", wrap: false, onChange });
    expect(press(choice.el, "ArrowRight").defaultPrevented).toBe(true);
    expect(choice.value).toBe("fr");
    press(choice.el, "ArrowLeft");
    press(choice.el, "ArrowLeft");
    press(choice.el, "ArrowLeft");
    expect(choice.value).toBe("es");
    expect(onChange).toHaveBeenCalledTimes(2);
  });

  it("goes to the first and the last option with Home and End", () => {
    const choice = make({ value: "en" });
    press(choice.el, "End");
    expect(choice.value).toBe("fr");
    press(choice.el, "Home");
    expect(choice.value).toBe("es");
  });

  it("leaves the keys it does not use to the page, and a key another control has prevented", () => {
    const choice = make();
    expect(press(choice.el, "Tab").defaultPrevented).toBe(false);
    expect(press(choice.el, "Enter").defaultPrevented).toBe(false);
    expect(choice.value).toBe("es");
    const event = new KeyboardEvent("keydown", { key: "ArrowRight", bubbles: true, cancelable: true });
    event.preventDefault();
    choice.el.dispatchEvent(event);
    expect(choice.value).toBe("es");
  });
});

describe("choice arrows and values", () => {
  it("moves with a click on either arrow, the same way as the keys", () => {
    const choice = make();
    const previous = choice.el.querySelector<HTMLElement>(".is-prev");
    const next = choice.el.querySelector<HTMLElement>(".is-next");
    next?.click();
    expect(choice.value).toBe("en");
    next?.click();
    next?.click();
    expect(choice.value).toBe("es");
    previous?.click();
    expect(choice.value).toBe("fr");
  });

  it("sets the value by name with setValue, and refuses a value that is not an option", () => {
    const choice = make();
    choice.setValue("fr");
    expect(choice.value).toBe("fr");
    expect(shownText(choice)).toBe("Français");
    expect(() => choice.setValue("de")).toThrow(RangeError);
    expect(choice.value).toBe("fr");
  });

  it("does not call onChange for a silent setValue, or for the value it already has", () => {
    const onChange = vi.fn();
    const choice = make({ onChange });
    choice.setValue("en", { silent: true });
    expect(choice.value).toBe("en");
    choice.setValue("en");
    expect(onChange).not.toHaveBeenCalled();
  });

  it("works with numbers as values", () => {
    const choice = make({ options: [{ value: 1, label: "Pequeño" }, { value: 2, label: "Grande" }] });
    press(choice.el, "ArrowRight");
    expect(choice.value).toBe(2);
    expect(shownText(choice)).toBe("Grande");
  });

  it("refuses a choice with no option, two options with one value, or a starting value that is not an option", () => {
    expect(() => createChoice({ options: [] })).toThrow(RangeError);
    expect(() => createChoice({ options: [{ value: "a", label: "A" }, { value: "a", label: "B" }] })).toThrow(RangeError);
    expect(() => createChoice({ options: LANGUAGES, value: "de" })).toThrow(RangeError);
  });
});

describe("choice disabled state and clean-up", () => {
  it("is out of the Tab order and does nothing when disabled, and its arrows do nothing either", () => {
    const onChange = vi.fn();
    const choice = make({ disabled: true, onChange });
    expect(choice.el.tabIndex).toBe(-1);
    expect(choice.el.getAttribute("aria-disabled")).toBe("true");
    press(choice.el, "ArrowRight");
    choice.el.querySelector<HTMLElement>(".is-next")?.click();
    expect(choice.value).toBe("es");
    expect(onChange).not.toHaveBeenCalled();
    choice.setDisabled(false);
    press(choice.el, "ArrowRight");
    expect(choice.value).toBe("en");
  });

  it("gives up the focus when it is disabled while it has it", () => {
    const choice = make();
    choice.el.focus();
    choice.setDisabled(true);
    expect(choice.el.contains(document.activeElement)).toBe(false);
  });

  it("removes the element, and its keys and arrows no longer reach it", () => {
    const onChange = vi.fn();
    const choice = make({ onChange });
    const element = choice.el;
    const next = element.querySelector<HTMLElement>(".is-next");
    choice.destroy();
    expect(element.isConnected).toBe(false);
    press(element, "ArrowRight");
    next?.click();
    expect(onChange).not.toHaveBeenCalled();
  });
});
