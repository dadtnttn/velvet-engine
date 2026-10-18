// The settings panel: the screen where the player changes a game's settings. It is made from sections, and each
// item of a section is one row: a label, and a slider, a toggle or a choice that reads and writes one setting of
// a store. The store is the settings store of @velvet/core (createSettingsStore). The panel needs only the methods
// that `SettingsPanelStore` declares, so this package does not import core.
//
// Writing. A change of a control writes the store at once. A change that the store made itself (a reset, or
// another part of the game) moves the control silently, so that the panel does not write the same value back.
//
// Keys. Up and Down move the focus through the controls and the reset button, and wrap at the ends. The other
// keys go to the focused control: Left and Right change a slider or a choice, and Enter or Space turn a toggle.
// A pad drives the panel through the same keys (startPadMenuKeys in @velvet/core).
//
// Texts. Section titles, row labels, choice options and the reset button's text go through `t`, as in the rebinding
// panel. A lookup that has no text returns the key, so the panel then shows the key as it is (the reset button falls
// back to English instead).

import { createButton, type ButtonProps } from "./components/button";
import { createChoice, type Choice, type ChoiceOption } from "./components/choice";
import type { VuComponent } from "./components/component";
import { createElement, focusableWithin, releaseFocus } from "./components/dom";
import { createSlider, type Slider } from "./components/slider";
import { createToggle, type Toggle } from "./components/toggle";
import type { TranslateFn } from "./rebind-panel";

/**
 * The part of a settings store that the panel uses. The store of @velvet/core has these methods, with the same
 * names and meanings. `reset` is optional: when a store has it, the panel shows a "reset to defaults" button.
 */
export interface SettingsPanelStore {
  /** The value of a setting. Throws when the store has no setting with that key. */
  get(key: string): unknown;
  /** Sets the value of a setting, and the store's subscribers hear of it. */
  set(key: string, value: unknown): void;
  /**
   * Runs the listener with the new value each time the setting changes.
   * @returns a function that removes the listener.
   */
  subscribe(key: string, listener: (value: unknown) => void): () => void;
  /** Puts every setting back to its default. Optional: a store without it gives the panel no reset button. */
  reset?(): void;
}

/** A row with a slider. The options are those of `createSlider`, and the key is the setting's key. */
export interface SliderItem {
  /** The kind of control. */
  readonly kind: "slider";
  /** The key of the setting in the store. */
  readonly key: string;
  /** The label of the row, which goes through `t`. */
  readonly label: string;
  /** The smallest value. */
  readonly min: number;
  /** The largest value. */
  readonly max: number;
  /** The distance between two values. Default 1. */
  readonly step?: number;
  /** The text beside the track for a value. */
  readonly format?: (value: number) => string;
}

/** A row with a toggle. */
export interface ToggleItem {
  /** The kind of control. */
  readonly kind: "toggle";
  /** The key of the setting in the store. */
  readonly key: string;
  /** The label of the row, which goes through `t`. */
  readonly label: string;
}

/** A row with a choice. The options are those of `createChoice`, and the key is the setting's key. */
export interface ChoiceItem {
  /** The kind of control. */
  readonly kind: "choice";
  /** The key of the setting in the store. */
  readonly key: string;
  /** The label of the row, which goes through `t`. */
  readonly label: string;
  /** The options, in order. Their labels go through `t`. */
  readonly options: readonly ChoiceOption[];
  /** Whether the arrows go round the ends. Default true. */
  readonly wrap?: boolean;
}

/** One row of a settings section. */
export type SettingItem = SliderItem | ToggleItem | ChoiceItem;

/** A group of rows under a title. */
export interface SettingsSection {
  /** The title of the section, which goes through `t`. */
  readonly title: string;
  /** The rows, in order. */
  readonly items: readonly SettingItem[];
}

/** Options of {@link createSettingsPanel}. */
export interface SettingsPanelOptions {
  /** The store that holds the settings. The panel reads it, writes it and follows its changes. */
  readonly store: SettingsPanelStore;
  /** The sections, in order. A key may appear in one row only. */
  readonly sections: readonly SettingsSection[];
  /** The text lookup for the titles, labels and the reset button. Default: the texts are shown as they are. */
  readonly t?: TranslateFn;
  /**
   * The key of the reset button's text, which goes through `t`. Default "settings.reset". When the lookup has no text
   * for the key, or there is no lookup, the button says "Reset to defaults".
   */
  readonly resetLabel?: string;
  /** Runs after the player resets the settings, once the controls show the store's values again. */
  readonly onReset?: () => void;
}

/** The settings panel. Create it with {@link createSettingsPanel}. */
export interface SettingsPanel {
  /** The panel's element. Put it in the page, or in the stage's root. */
  readonly el: HTMLElement;
  /** Removes the element, and stops following the store. The store keeps its values. */
  destroy(): void;
}

/** The text of the reset button when the lookup has none. */
const RESET_FALLBACK = "Reset to defaults";

/** The key of the reset button's text. */
const RESET_KEY = "settings.reset";

/** One control of a row, as the panel holds it. */
interface Bound {
  /** The element of the control. */
  readonly el: HTMLElement;
  /** Shows a value of the store, without writing it back. A value of the wrong type is left out. */
  show(value: unknown): void;
  /** Removes the control. */
  destroy(): void;
}

/**
 * Creates a settings panel. It is built at once, with a row for each item and its control at the store's value.
 * @param options - the store, the sections and the texts, see {@link SettingsPanelOptions}.
 * @throws RangeError when a key is in the panel twice, a choice has no option, or the store has no setting with a key.
 */
export function createSettingsPanel(options: SettingsPanelOptions): SettingsPanel {
  const { store, sections } = options;
  const text = (key: string): string => (options.t === undefined ? key : options.t(key));
  checkKeys(sections);

  const el = createElement("div", "vu-settings");
  el.setAttribute("role", "group");
  const bound: Array<{ readonly key: string; readonly control: Bound }> = [];
  const unsubscribes: Array<() => void> = [];

  try {
    for (const section of sections) {
      const block = document.createElement("section");
      block.className = "vu-settings-section";
      block.append(createElement("h2", "vu-settings-title", text(section.title)));
      for (const item of section.items) {
        const row = createElement("div", "vu-settings-row");
        const holder = createElement("div", "vu-settings-control");
        const control = bindItem(item, store, text);
        holder.append(control.el);
        row.append(createElement("span", "vu-settings-label", text(item.label)), holder);
        block.append(row);
        bound.push({ key: item.key, control });
        unsubscribes.push(store.subscribe(item.key, (value) => control.show(value)));
      }
      el.append(block);
    }
  } catch (error) {
    // A row that cannot be made (the store has no such setting, say) leaves nothing behind: the rows before it are undone.
    for (const unsubscribe of unsubscribes.splice(0)) unsubscribe();
    for (const entry of bound.splice(0)) entry.control.destroy();
    throw error;
  }

  let resetButton: VuComponent<ButtonProps> | null = null;
  if (typeof store.reset === "function") {
    const footer = createElement("div", "vu-settings-footer");
    resetButton = createButton({ label: resetText(), onPress: () => resetAll() });
    footer.append(resetButton.el);
    el.append(footer);
  }

  /** Resets the store, then shows its values again in every control, and tells the game. */
  function resetAll(): void {
    store.reset?.();
    for (const entry of bound) entry.control.show(store.get(entry.key));
    options.onReset?.();
  }

  /** The text of the reset button, or the English one when the lookup has none. */
  function resetText(): string {
    const key = options.resetLabel ?? RESET_KEY;
    const shown = options.t?.(key);
    return shown === undefined || shown === key ? RESET_FALLBACK : shown;
  }

  /**
   * Up and Down move between the controls. This runs before the control gets the key, and it prevents the default,
   * so the focused control leaves the arrow to the panel.
   */
  const onKeyDown = (event: KeyboardEvent): void => {
    if (event.key !== "ArrowUp" && event.key !== "ArrowDown") return;
    if (event.defaultPrevented) return;
    const stops = focusableWithin(el);
    if (stops.length === 0) return;
    event.preventDefault();
    const at = stops.findIndex((node) => node === document.activeElement);
    const direction = event.key === "ArrowDown" ? 1 : -1;
    const target = at < 0 ? (direction > 0 ? 0 : stops.length - 1) : (at + direction + stops.length) % stops.length;
    stops[target]?.focus({ preventScroll: true });
  };
  el.addEventListener("keydown", onKeyDown, true);

  return {
    el,
    destroy(): void {
      el.removeEventListener("keydown", onKeyDown, true);
      for (const unsubscribe of unsubscribes.splice(0)) unsubscribe();
      for (const entry of bound.splice(0)) entry.control.destroy();
      resetButton?.destroy();
      resetButton = null;
      releaseFocus(el);
      el.remove();
    },
  };
}

/**
 * Makes the control of one row. Its value is the store's value when that has the right type, and otherwise the
 * control's own default. A change of the control writes the store.
 * @param item - the row.
 * @param store - the store that holds the setting.
 * @param text - the text lookup: the row's label, which is also the control's name for a screen reader, and the
 *   choice's option labels go through it.
 * @throws RangeError when the item has an unknown kind, a choice has no option, or the store has no such setting.
 */
function bindItem(item: SettingItem, store: SettingsPanelStore, text: (key: string) => string): Bound {
  const stored = store.get(item.key);
  const label = text(item.label);
  switch (item.kind) {
    case "slider": {
      const slider: Slider = createSlider({
        label,
        min: item.min,
        max: item.max,
        step: item.step,
        format: item.format,
        value: typeof stored === "number" && Number.isFinite(stored) ? stored : undefined,
        onChange: (value) => store.set(item.key, value),
      });
      return {
        el: slider.el,
        show(value: unknown): void {
          if (typeof value === "number" && Number.isFinite(value)) slider.setValue(value, { silent: true });
        },
        destroy: () => slider.destroy(),
      };
    }
    case "toggle": {
      const toggle: Toggle = createToggle({
        label,
        value: typeof stored === "boolean" ? stored : undefined,
        onChange: (value) => store.set(item.key, value),
      });
      return {
        el: toggle.el,
        show(value: unknown): void {
          if (typeof value === "boolean") toggle.setValue(value, { silent: true });
        },
        destroy: () => toggle.destroy(),
      };
    }
    case "choice": {
      const options = item.options.map((option) => ({ value: option.value, label: text(option.label) }));
      const choice: Choice = createChoice({
        label,
        options,
        value: options.find((option) => option.value === stored)?.value,
        wrap: item.wrap,
        onChange: (value) => store.set(item.key, value),
      });
      return {
        el: choice.el,
        show(value: unknown): void {
          const found = options.find((option) => option.value === value);
          if (found !== undefined) choice.setValue(found.value, { silent: true });
        },
        destroy: () => choice.destroy(),
      };
    }
    default:
      throw new RangeError(`a setting of kind "${String((item as { readonly kind: unknown }).kind)}" is not known`);
  }
}

/**
 * Checks that no key is in the panel twice: a setting has one row, or two controls would fight over it.
 * @throws RangeError when a key appears twice.
 */
function checkKeys(sections: readonly SettingsSection[]): void {
  const seen = new Set<string>();
  for (const section of sections) {
    for (const item of section.items) {
      if (seen.has(item.key)) throw new RangeError(`the setting "${item.key}" is in the panel twice`);
      seen.add(item.key);
    }
  }
}
