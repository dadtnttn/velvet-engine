// A toggle: a switch that is on or off. The player turns it with Enter or Space, or with a click. The element
// has role "switch" and aria-checked, so a screen reader says whether it is on or off. The knob moves with a
// transform when the value changes, and hover and focus change colour only.
//
// The switch is a div and not a button on purpose. The pad's A reaches the game as an Enter key, and
// startPadMenuKeys clicks a focused button for Enter, so a button would turn twice on each press.

import { createElement, releaseFocus } from "./dom";

/** Options of {@link createToggle}. */
export interface ToggleOptions {
  /** The name that a screen reader gives the switch, such as "Subtitles". */
  readonly label?: string;
  /** The starting value. Default false. */
  readonly value?: boolean;
  /** Runs with the new value when the player turns the switch. A silent `setValue` does not run it. */
  readonly onChange?: (value: boolean) => void;
  /** True disables the switch: it leaves the Tab order, and the keys and the pointer do nothing. Default false. */
  readonly disabled?: boolean;
}

/** A toggle. Create it with {@link createToggle}. */
export interface Toggle {
  /** The element. Put it in the page, or in the stage's root. */
  readonly el: HTMLElement;
  /** The current value: true when the switch is on. */
  readonly value: boolean;
  /**
   * Sets the value. Unless `silent` is true, `onChange` runs when the value changes.
   * @throws RangeError when the value is not true or false.
   */
  setValue(value: boolean, options?: { readonly silent?: boolean }): void;
  /** Runs with the new value when the player turns the switch. It can be set after the switch is made. */
  onChange: ((value: boolean) => void) | undefined;
  /** Disables or enables the switch. A switch that is disabled gives up the focus. */
  setDisabled(disabled: boolean): void;
  /** Removes the element and its listeners. */
  destroy(): void;
}

/**
 * Creates a toggle.
 * @param options - the label, the starting value and the callbacks, see {@link ToggleOptions}.
 * @throws RangeError when the starting value is not true or false.
 */
export function createToggle(options: ToggleOptions = {}): Toggle {
  const el = createElement("div", "vu-toggle");
  el.setAttribute("role", "switch");
  if (options.label !== undefined && options.label !== "") el.setAttribute("aria-label", options.label);
  const knob = createElement("span", "vu-toggle-knob");
  knob.setAttribute("aria-hidden", "true");
  el.append(knob);

  let current = checkedValue(options.value ?? false);
  let disabled = options.disabled === true;

  /** Draws the ARIA state and the focus rules from the current value. */
  const render = (): void => {
    el.setAttribute("aria-checked", String(current));
    el.tabIndex = disabled ? -1 : 0;
    if (disabled) el.setAttribute("aria-disabled", "true");
    else el.removeAttribute("aria-disabled");
  };

  const setValue = (value: boolean, setOptions: { readonly silent?: boolean } = {}): void => {
    const next = checkedValue(value);
    if (next === current) return;
    current = next;
    render();
    if (setOptions.silent !== true) toggle.onChange?.(current);
  };

  /** Turns the switch over, by a key or a click, unless it is disabled. */
  const flip = (): void => {
    if (!disabled) setValue(!current);
  };

  /** Enter and Space turn the switch. A key that another control has taken (prevented) is left alone. */
  const onKeyDown = (event: KeyboardEvent): void => {
    if (disabled || event.defaultPrevented) return;
    if (event.key !== "Enter" && event.key !== " ") return;
    event.preventDefault();
    flip();
  };

  el.addEventListener("keydown", onKeyDown);
  el.addEventListener("click", flip);
  render();

  const toggle: Toggle = {
    el,
    get value(): boolean {
      return current;
    },
    setValue,
    onChange: options.onChange,
    setDisabled(next: boolean): void {
      disabled = next;
      if (disabled) releaseFocus(el);
      render();
    },
    destroy(): void {
      el.removeEventListener("keydown", onKeyDown);
      el.removeEventListener("click", flip);
      releaseFocus(el);
      el.remove();
    },
  };
  return toggle;
}

/**
 * Returns the value when it is true or false.
 * @throws RangeError when it is anything else.
 */
function checkedValue(value: boolean): boolean {
  if (typeof value !== "boolean") throw new RangeError(`a toggle takes true or false, got ${String(value)}`);
  return value;
}
