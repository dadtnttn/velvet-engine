// A choice: one option out of a short list, shown with its name between two arrows, as the settings of console
// games are. Left and Up go to the previous option, Right and Down to the next, Home and End to the first and the
// last, and a click on an arrow does the same as its key. With `wrap` (the default) the list goes round at the
// ends; without it, the choice stops at the first and the last option.
//
// The element has role "listbox", and its one visible option is its active descendant. A screen reader then
// says the name of the option and its place in the list. The arrows are hidden from the screen reader, because
// the keys already do their work.

import { createElement, releaseFocus, uniqueId } from "./dom";

/** The value that an option holds: a string or a number. */
export type ChoiceValue = string | number;

/** One option of a choice. */
export interface ChoiceOption {
  /** The value that the choice holds while this option is shown. */
  readonly value: ChoiceValue;
  /** The text that is shown for the option. */
  readonly label: string;
}

/** Options of {@link createChoice}. */
export interface ChoiceOptions {
  /** The name that a screen reader gives the choice, such as "Language". */
  readonly label?: string;
  /** The options, in order. At least one. Each value must be different from the others. */
  readonly options: readonly ChoiceOption[];
  /** The starting value. Default: the value of the first option. It must be the value of one of the options. */
  readonly value?: ChoiceValue;
  /** Whether the arrows go round from the last option to the first, and back. Default true. */
  readonly wrap?: boolean;
  /** Runs with the new value when the player changes it. A silent `setValue` does not run it. */
  readonly onChange?: (value: ChoiceValue) => void;
  /** True disables the choice: it leaves the Tab order, and the keys and the arrows do nothing. Default false. */
  readonly disabled?: boolean;
}

/** A choice. Create it with {@link createChoice}. */
export interface Choice {
  /** The element. Put it in the page, or in the stage's root. */
  readonly el: HTMLElement;
  /** The value of the option that is shown. */
  readonly value: ChoiceValue;
  /**
   * Shows the option with this value. Unless `silent` is true, `onChange` runs when the option changes.
   * @throws RangeError when no option has the value.
   */
  setValue(value: ChoiceValue, options?: { readonly silent?: boolean }): void;
  /** Runs with the new value when the player changes it. It can be set after the choice is made. */
  onChange: ((value: ChoiceValue) => void) | undefined;
  /** Disables or enables the choice. A choice that is disabled gives up the focus. */
  setDisabled(disabled: boolean): void;
  /** Removes the element and its listeners. */
  destroy(): void;
}

/**
 * Creates a choice.
 * @param options - the options and the callbacks, see {@link ChoiceOptions}.
 * @throws RangeError when there is no option, two options have the same value, or the starting value is not one of them.
 */
export function createChoice(options: ChoiceOptions): Choice {
  const list = options.options;
  if (list.length === 0) throw new RangeError("a choice needs at least one option");
  const seen = new Set<ChoiceValue>();
  for (const option of list) {
    if (seen.has(option.value)) throw new RangeError(`the value ${String(option.value)} is given to two options`);
    seen.add(option.value);
  }
  const wrap = options.wrap ?? true;

  const el = createElement("div", "vu-choice");
  el.setAttribute("role", "listbox");
  if (options.label !== undefined && options.label !== "") el.setAttribute("aria-label", options.label);
  const previous = createElement("div", "vu-choice-arrow is-prev", "‹");
  const next = createElement("div", "vu-choice-arrow is-next", "›");
  previous.setAttribute("aria-hidden", "true");
  next.setAttribute("aria-hidden", "true");
  const shown = createElement("div", "vu-choice-value");
  shown.id = uniqueId("choice-option");
  shown.setAttribute("role", "option");
  shown.setAttribute("aria-selected", "true");
  el.append(previous, shown, next);

  let index = indexOfValue(options.value ?? list[0]?.value ?? "");
  let disabled = options.disabled === true;

  /** Draws the option, its place in the list, and the focus rules from the current index. */
  const render = (): void => {
    shown.textContent = list[index]?.label ?? "";
    shown.setAttribute("aria-posinset", String(index + 1));
    shown.setAttribute("aria-setsize", String(list.length));
    el.setAttribute("aria-activedescendant", shown.id);
    el.tabIndex = disabled ? -1 : 0;
    if (disabled) el.setAttribute("aria-disabled", "true");
    else el.removeAttribute("aria-disabled");
  };

  /** The index of the option with a value. @throws RangeError when no option has it. */
  function indexOfValue(value: ChoiceValue): number {
    const found = list.findIndex((option) => option.value === value);
    if (found < 0) throw new RangeError(`${String(value)} is not an option of this choice`);
    return found;
  }

  /** Shows the option at an index, and tells the listeners unless the change is silent. */
  const select = (target: number, silent: boolean): void => {
    if (target === index) return;
    index = target;
    render();
    const picked = list[index];
    if (picked !== undefined && !silent) choice.onChange?.(picked.value);
  };

  /** Moves one option forward or back, round the ends when the choice wraps, and stopping at them when it does not. */
  const move = (direction: 1 | -1): void => {
    if (disabled) return;
    const count = list.length;
    const target = wrap ? (index + direction + count) % count : Math.min(count - 1, Math.max(0, index + direction));
    select(target, false);
  };

  /** The arrow keys, Home and End. A key that another control has taken (prevented) is left alone. */
  const onKeyDown = (event: KeyboardEvent): void => {
    if (disabled || event.defaultPrevented) return;
    switch (event.key) {
      case "ArrowLeft":
      case "ArrowUp":
        move(-1);
        break;
      case "ArrowRight":
      case "ArrowDown":
        move(1);
        break;
      case "Home":
        select(0, false);
        break;
      case "End":
        select(list.length - 1, false);
        break;
      default:
        return;
    }
    event.preventDefault();
  };

  const onPreviousClick = (): void => {
    move(-1);
  };
  const onNextClick = (): void => {
    move(1);
  };

  el.addEventListener("keydown", onKeyDown);
  previous.addEventListener("click", onPreviousClick);
  next.addEventListener("click", onNextClick);
  render();

  const choice: Choice = {
    el,
    get value(): ChoiceValue {
      return list[index]?.value ?? "";
    },
    setValue(value: ChoiceValue, setOptions: { readonly silent?: boolean } = {}): void {
      select(indexOfValue(value), setOptions.silent === true);
    },
    onChange: options.onChange,
    setDisabled(nextDisabled: boolean): void {
      disabled = nextDisabled;
      if (disabled) releaseFocus(el);
      render();
    },
    destroy(): void {
      el.removeEventListener("keydown", onKeyDown);
      previous.removeEventListener("click", onPreviousClick);
      next.removeEventListener("click", onNextClick);
      releaseFocus(el);
      el.remove();
    },
  };
  return choice;
}
