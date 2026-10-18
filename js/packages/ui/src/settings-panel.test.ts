// @vitest-environment happy-dom
// Tests for the settings panel, with a fake store that has the methods the panel declares. The rows and their
// controls, the values the store holds, writes, changes that the store makes (silent), the keys that move between
// the rows, the reset button, the texts, the wrong types, the duplicate keys and the clean-up.

import { afterEach, describe, expect, it, vi } from "vitest";
import { createSettingsPanel, type SettingsPanel, type SettingsPanelStore, type SettingsSection } from "./settings-panel";

type Listener = (value: unknown) => void;

/** A store in memory with the methods that the panel uses, and the means for a test to see and change it. */
interface FakeStore extends SettingsPanelStore {
  /** Every write of the panel, in order. */
  readonly writes: Array<[string, unknown]>;
  /** Changes a value as another part of the game would, so that the subscribers hear of it. */
  change(key: string, value: unknown): void;
  /** How many listeners follow a key. */
  subscribers(key: string): number;
}

/** A store with the values given, whose reset (when it has one) goes back to them. */
function fakeStore(initial: Record<string, unknown>, withReset = false): FakeStore {
  const values = new Map<string, unknown>(Object.entries(initial));
  const defaults = new Map<string, unknown>(Object.entries(initial));
  const listeners = new Map<string, Set<Listener>>();
  const writes: Array<[string, unknown]> = [];
  const notify = (key: string, value: unknown): void => {
    for (const listener of [...(listeners.get(key) ?? [])]) listener(value);
  };
  const store: FakeStore = {
    writes,
    get(key: string): unknown {
      if (!values.has(key)) throw new RangeError(`no setting "${key}"`);
      return values.get(key);
    },
    set(key: string, value: unknown): void {
      writes.push([key, value]);
      values.set(key, value);
      notify(key, value);
    },
    subscribe(key: string, listener: Listener): () => void {
      const set = listeners.get(key) ?? new Set<Listener>();
      set.add(listener);
      listeners.set(key, set);
      return () => {
        set.delete(listener);
      };
    },
    change(key: string, value: unknown): void {
      values.set(key, value);
      notify(key, value);
    },
    subscribers(key: string): number {
      return listeners.get(key)?.size ?? 0;
    },
    ...(withReset
      ? {
          reset(): void {
            for (const [key, value] of defaults) {
              values.set(key, value);
              notify(key, value);
            }
          },
        }
      : {}),
  };
  return store;
}

/** Two sections with a slider, a toggle and a choice. */
const SECTIONS: readonly SettingsSection[] = [
  {
    title: "Sonido",
    items: [
      { kind: "slider", key: "volume", label: "Volumen", min: 0, max: 10, step: 1, format: (value) => `${value} / 10` },
      { kind: "toggle", key: "subtitles", label: "Subtítulos" },
    ],
  },
  {
    title: "Pantalla",
    items: [
      {
        kind: "choice",
        key: "language",
        label: "Idioma",
        options: [
          { value: "es", label: "Español" },
          { value: "en", label: "English" },
        ],
      },
    ],
  },
];

afterEach(() => {
  vi.restoreAllMocks();
  document.body.innerHTML = "";
});

/** A panel in the document, with the store and the options given. */
function make(store: SettingsPanelStore, options: Partial<Parameters<typeof createSettingsPanel>[0]> = {}): SettingsPanel {
  const panel = createSettingsPanel({ store, sections: SECTIONS, ...options });
  document.body.append(panel.el);
  return panel;
}

/** Presses a key on an element, the way a keyboard does while it has the focus. */
function press(target: HTMLElement, key: string): KeyboardEvent {
  const event = new KeyboardEvent("keydown", { key, bubbles: true, cancelable: true });
  target.dispatchEvent(event);
  return event;
}

/** The control of a kind in the panel, which must exist. */
function control(panel: SettingsPanel, selector: string): HTMLElement {
  const node = panel.el.querySelector<HTMLElement>(selector);
  if (node === null) throw new Error(`no control for ${selector}`);
  return node;
}

const slider = (panel: SettingsPanel): HTMLElement => control(panel, '[role="slider"]');
const toggle = (panel: SettingsPanel): HTMLElement => control(panel, '[role="switch"]');
const choice = (panel: SettingsPanel): HTMLElement => control(panel, '[role="listbox"]');

/** The reset button, which must exist. */
function resetButton(panel: SettingsPanel): HTMLButtonElement {
  const node = panel.el.querySelector<HTMLButtonElement>(".vu-settings-footer button");
  if (node === null) throw new Error("no reset button");
  return node;
}

describe("settings panel structure", () => {
  it("makes a section for each group, a row for each item, and the control of its kind", () => {
    const panel = make(fakeStore({ volume: 5, subtitles: false, language: "es" }));
    expect(panel.el.querySelectorAll(".vu-settings-section")).toHaveLength(2);
    expect(panel.el.querySelectorAll(".vu-settings-row")).toHaveLength(3);
    expect(panel.el.querySelector(".vu-settings-title")?.textContent).toBe("Sonido");
    const labels = Array.from(panel.el.querySelectorAll(".vu-settings-label"), (node) => node.textContent);
    expect(labels).toEqual(["Volumen", "Subtítulos", "Idioma"]);
    expect(slider(panel).getAttribute("aria-label")).toBe("Volumen");
    expect(toggle(panel).getAttribute("aria-label")).toBe("Subtítulos");
    expect(choice(panel).getAttribute("aria-label")).toBe("Idioma");
  });

  it("shows each control at the value that the store holds", () => {
    const panel = make(fakeStore({ volume: 7, subtitles: true, language: "en" }));
    expect(slider(panel).getAttribute("aria-valuenow")).toBe("7");
    expect(slider(panel).getAttribute("aria-valuetext")).toBe("7 / 10");
    expect(toggle(panel).getAttribute("aria-checked")).toBe("true");
    expect(choice(panel).querySelector('[role="option"]')?.textContent).toBe("English");
  });

  it("has no reset button when the store cannot reset", () => {
    const panel = make(fakeStore({ volume: 5, subtitles: false, language: "es" }));
    expect(panel.el.querySelector(".vu-settings-footer")).toBeNull();
  });
});

describe("settings panel writes", () => {
  it("writes the store when the player changes a control", () => {
    const store = fakeStore({ volume: 5, subtitles: false, language: "es" });
    const panel = make(store);
    press(slider(panel), "ArrowRight");
    toggle(panel).click();
    press(choice(panel), "ArrowRight");
    expect(store.writes).toEqual([
      ["volume", 6],
      ["subtitles", true],
      ["language", "en"],
    ]);
    expect(store.get("volume")).toBe(6);
  });

  it("moves a control silently when the store changes, without writing the same value back", () => {
    const store = fakeStore({ volume: 5, subtitles: false, language: "es" });
    const panel = make(store);
    store.change("volume", 2);
    store.change("subtitles", true);
    store.change("language", "en");
    expect(slider(panel).getAttribute("aria-valuenow")).toBe("2");
    expect(toggle(panel).getAttribute("aria-checked")).toBe("true");
    expect(choice(panel).querySelector('[role="option"]')?.textContent).toBe("English");
    expect(store.writes).toEqual([]);
  });

  it("follows one subscription per setting, and gives them up when destroyed", () => {
    const store = fakeStore({ volume: 5, subtitles: false, language: "es" });
    const panel = make(store);
    expect(store.subscribers("volume")).toBe(1);
    panel.destroy();
    expect(store.subscribers("volume")).toBe(0);
    expect(store.subscribers("subtitles")).toBe(0);
    store.change("volume", 9);
    expect(store.writes).toEqual([]);
  });
});

describe("settings panel keys", () => {
  it("moves between the rows with Up and Down, and wraps at the ends", () => {
    const store = fakeStore({ volume: 5, subtitles: false, language: "es" });
    const panel = make(store);
    slider(panel).focus();
    press(slider(panel), "ArrowDown");
    expect(document.activeElement).toBe(toggle(panel));
    press(toggle(panel), "ArrowDown");
    expect(document.activeElement).toBe(choice(panel));
    press(choice(panel), "ArrowDown");
    expect(document.activeElement).toBe(slider(panel));
    press(slider(panel), "ArrowUp");
    expect(document.activeElement).toBe(choice(panel));
  });

  it("leaves Up and Down to the panel, so the focused control does not change its value", () => {
    const store = fakeStore({ volume: 5, subtitles: false, language: "es" });
    const panel = make(store);
    slider(panel).focus();
    press(slider(panel), "ArrowDown");
    expect(store.writes).toEqual([]);
    expect(slider(panel).getAttribute("aria-valuenow")).toBe("5");
  });

  it("lets Left and Right change the focused control, and Enter turn a toggle", () => {
    const store = fakeStore({ volume: 5, subtitles: false, language: "es" });
    const panel = make(store);
    toggle(panel).focus();
    press(toggle(panel), "Enter");
    expect(store.writes).toEqual([["subtitles", true]]);
    press(slider(panel), "ArrowLeft");
    expect(store.writes.at(-1)).toEqual(["volume", 4]);
  });

  it("includes the reset button in the rows that Up and Down reach", () => {
    const panel = make(fakeStore({ volume: 5, subtitles: false, language: "es" }, true));
    choice(panel).focus();
    press(choice(panel), "ArrowDown");
    expect(document.activeElement).toBe(resetButton(panel));
  });
});

describe("settings panel reset", () => {
  it("shows a reset button when the store can reset, and resets every control to the store's values", () => {
    const store = fakeStore({ volume: 5, subtitles: false, language: "es" }, true);
    const onReset = vi.fn();
    const panel = make(store, { onReset });
    expect(resetButton(panel).textContent).toBe("Reset to defaults");
    press(slider(panel), "ArrowRight");
    toggle(panel).click();
    resetButton(panel).click();
    expect(slider(panel).getAttribute("aria-valuenow")).toBe("5");
    expect(toggle(panel).getAttribute("aria-checked")).toBe("false");
    expect(onReset).toHaveBeenCalledTimes(1);
  });
});

describe("settings panel texts", () => {
  it("translates the titles, the labels, the options and the reset button through t", () => {
    const words: Record<string, string> = {
      Sonido: "Sound",
      Volumen: "Volume",
      "settings.reset": "Restore",
      Español: "Spanish",
    };
    const panel = make(fakeStore({ volume: 5, subtitles: false, language: "es" }, true), {
      t: (key) => words[key] ?? key,
    });
    expect(panel.el.querySelector(".vu-settings-title")?.textContent).toBe("Sound");
    expect(slider(panel).getAttribute("aria-label")).toBe("Volume");
    expect(panel.el.querySelector(".vu-settings-label")?.textContent).toBe("Volume");
    expect(choice(panel).querySelector('[role="option"]')?.textContent).toBe("Spanish");
    expect(resetButton(panel).textContent).toBe("Restore");
  });

  it("reads the reset button's text from resetLabel, and falls back to English when the lookup has no text for it", () => {
    const words: Record<string, string> = { "menu.restore": "Restaurar" };
    const named = make(fakeStore({ volume: 5, subtitles: false, language: "es" }, true), {
      t: (key) => words[key] ?? key,
      resetLabel: "menu.restore",
    });
    expect(resetButton(named).textContent).toBe("Restaurar");
    const missing = make(fakeStore({ volume: 5, subtitles: false, language: "es" }, true), {
      t: (key) => key,
      resetLabel: "menu.missing",
    });
    expect(resetButton(missing).textContent).toBe("Reset to defaults");
  });

  it("shows a text as it is when the lookup has no translation for it", () => {
    const panel = make(fakeStore({ volume: 5, subtitles: false, language: "es" }, true), {
      t: (key) => (key === "Volumen" ? "Volume" : key),
    });
    expect(panel.el.querySelector(".vu-settings-title")?.textContent).toBe("Sonido");
    expect(resetButton(panel).textContent).toBe("Reset to defaults");
  });
});

describe("settings panel values and checks", () => {
  it("shows the control's own default for a stored value of the wrong type or outside the options", () => {
    const panel = make(fakeStore({ volume: "loud", subtitles: 1, language: "fr" }));
    expect(slider(panel).getAttribute("aria-valuenow")).toBe("0");
    expect(toggle(panel).getAttribute("aria-checked")).toBe("false");
    expect(choice(panel).querySelector('[role="option"]')?.textContent).toBe("Español");
  });

  it("refuses a key that is in the panel twice, and a setting that the store does not have", () => {
    const store = fakeStore({ volume: 5, subtitles: false, language: "es" });
    const twice: SettingsSection[] = [
      { title: "A", items: [{ kind: "toggle", key: "subtitles", label: "Uno" }] },
      { title: "B", items: [{ kind: "toggle", key: "subtitles", label: "Dos" }] },
    ];
    expect(() => createSettingsPanel({ store, sections: twice })).toThrow(RangeError);
    const missing: SettingsSection[] = [{ title: "A", items: [{ kind: "toggle", key: "shadows", label: "Sombras" }] }];
    expect(() => createSettingsPanel({ store, sections: missing })).toThrow(RangeError);
    // A row that fails after a good one leaves no subscription behind.
    const partly: SettingsSection[] = [
      {
        title: "A",
        items: [
          { kind: "toggle", key: "subtitles", label: "Subtítulos" },
          { kind: "toggle", key: "shadows", label: "Sombras" },
        ],
      },
    ];
    expect(() => createSettingsPanel({ store, sections: partly })).toThrow(RangeError);
    expect(store.subscribers("subtitles")).toBe(0);
  });

  it("removes its elements and releases the focus when it is destroyed", () => {
    const panel = make(fakeStore({ volume: 5, subtitles: false, language: "es" }));
    const element = panel.el;
    slider(panel).focus();
    panel.destroy();
    expect(element.isConnected).toBe(false);
    expect(element.contains(document.activeElement)).toBe(false);
    expect(press(element, "ArrowDown").defaultPrevented).toBe(false);
  });
});
