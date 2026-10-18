// The HUD: the top bar (the day and its bar, the speed and pause buttons, the stock of materials,
// the coins and the menu button) and the line of key hints at the bottom. Numbers come from the clock
// and the board; every word comes from @velvet/i18n, so the HUD changes language as a whole.

import type { I18n } from "@velvet/i18n";
import {
  createButton,
  createChip,
  createKeycap,
  createMeter,
  type ButtonProps,
  type ChipProps,
  type MeterProps,
  type VuComponent,
} from "@velvet/ui";
import { DAY_LENGTH, SPEEDS, STOCK_DEFS } from "./config";

/** What the HUD's buttons do. The game owns the state; the HUD only shows it. */
export interface HudCallbacks {
  /** Pause or resume the clock. */
  readonly onPause: () => void;
  /** Set the clock's speed. */
  readonly onSpeed: (speed: number) => void;
  /** Open or close the pause menu. */
  readonly onMenu: () => void;
}

/** The numbers that the HUD shows, read from the clock and the board each frame. */
export interface HudState {
  /** The current day, from 1. */
  readonly day: number;
  /** Game seconds into the day, from 0 up to DAY_LENGTH. */
  readonly timeOfDay: number;
  /** Whether the clock is paused. */
  readonly paused: boolean;
  /** The clock's speed multiplier. */
  readonly speed: number;
  /** How many cards of each stock definition lie on the table. */
  readonly stock: ReadonlyMap<string, number>;
  /** How many coin cards lie on the table. */
  readonly coins: number;
}

/** A key hint: the keys to draw as keycaps, and the i18n id of the words after them. */
interface HintSpec {
  readonly keys: readonly string[];
  readonly text: string;
}

/** The hints of the bottom line, in reading order. */
const HINTS: readonly HintSpec[] = [
  { keys: ["Space"], text: "hint.pause" },
  { keys: ["Digit1", "Digit2", "Digit3"], text: "hint.speed" },
  { keys: ["F5"], text: "hint.quickSave" },
  { keys: ["F9"], text: "hint.quickLoad" },
  { keys: ["Escape"], text: "hint.menu" },
  { keys: [], text: "hint.drag" },
];

/** A hint on screen: its words, which are re-written when the language changes. */
interface HintView {
  readonly words: HTMLElement;
  readonly key: string;
}

/**
 * The HUD. It appends two elements to the parent it is given: the top bar and the hint line. Call
 * `update` each frame with the numbers, and `refresh` after the language changes.
 */
export class Hud {
  private readonly texts: I18n;
  private readonly labelOf: (def: string) => string;
  private readonly title: HTMLElement;
  private readonly dayLabel: HTMLElement;
  private readonly dayBar: VuComponent<MeterProps>;
  private readonly speedButtons: Array<{ readonly speed: number; readonly button: VuComponent<ButtonProps> }>;
  private readonly pauseButton: VuComponent<ButtonProps>;
  private readonly menuButton: VuComponent<ButtonProps>;
  private readonly chips = new Map<string, VuComponent<ChipProps>>();
  private readonly coinLabel: HTMLElement;
  private readonly hints: HintView[] = [];

  /** Values shown last, so that the texts are written only when they change. */
  private shownDay = 0;
  private shownPaused: boolean | null = null;
  private shownCoins = -1;

  /**
   * @param parent - the element that the top bar and the hint line are appended to.
   * @param texts - the game's texts.
   * @param callbacks - what the buttons do.
   * @param labelOf - the name of a card definition, for the stock chips.
   */
  constructor(parent: HTMLElement, texts: I18n, callbacks: HudCallbacks, labelOf: (def: string) => string) {
    this.texts = texts;
    this.labelOf = labelOf;

    const bar = document.createElement("header");
    bar.className = "hc-topbar";
    this.title = document.createElement("h1");
    this.title.className = "hc-title";

    const day = document.createElement("div");
    day.className = "hc-day";
    this.dayLabel = document.createElement("span");
    this.dayLabel.className = "hc-day-label";
    this.dayBar = createMeter({ label: "", value: 0, max: DAY_LENGTH, tone: "gold" });
    this.dayBar.el.classList.add("hc-daybar");
    day.append(this.dayLabel, this.dayBar.el);

    const speed = document.createElement("div");
    speed.className = "hc-speed";
    this.speedButtons = SPEEDS.map((value) => ({
      speed: value,
      button: createButton({
        label: `${value}×`,
        hotkey: String(value),
        pressed: false,
        onPress: () => callbacks.onSpeed(value),
      }),
    }));
    for (const { button } of this.speedButtons) speed.append(button.el);
    this.pauseButton = createButton({ label: "", hotkey: "Space", pressed: false, onPress: callbacks.onPause });
    speed.append(this.pauseButton.el);

    const stock = document.createElement("div");
    stock.className = "hc-stock";
    for (const def of STOCK_DEFS) {
      const chip = createChip({ label: this.labelOf(def), count: 0 });
      this.chips.set(def, chip);
      stock.append(chip.el);
    }
    this.coinLabel = document.createElement("p");
    this.coinLabel.className = "hc-coins";

    const menu = document.createElement("div");
    menu.className = "hc-menu";
    this.menuButton = createButton({ label: "", hotkey: "Escape", onPress: callbacks.onMenu });
    menu.append(this.menuButton.el);

    bar.append(this.title, day, speed, stock, this.coinLabel, menu);

    const hints = document.createElement("footer");
    hints.className = "hc-hints";
    for (const spec of HINTS) {
      const item = document.createElement("span");
      item.className = "hc-hint";
      for (const key of spec.keys) item.append(createKeycap({ key }).el);
      const words = document.createElement("span");
      words.className = "hc-hint-text";
      item.append(words);
      hints.append(item);
      this.hints.push({ words, key: spec.text });
    }

    parent.append(bar, hints);
    this.refresh();
  }

  /** Writes the numbers and states that changed since the last call. */
  update(state: HudState): void {
    if (state.day !== this.shownDay) {
      this.shownDay = state.day;
      this.dayLabel.textContent = this.texts.t("hud.day", { day: state.day });
    }
    this.dayBar.update({ value: state.timeOfDay });
    for (const { speed, button } of this.speedButtons) {
      button.update({ pressed: speed === state.speed });
    }
    if (state.paused !== this.shownPaused) {
      this.shownPaused = state.paused;
      this.pauseButton.update({
        label: this.texts.t(state.paused ? "hud.resume" : "hud.pause"),
        pressed: state.paused,
      });
    }
    for (const [def, chip] of this.chips) {
      chip.update({ count: state.stock.get(def) ?? 0 });
    }
    if (state.coins !== this.shownCoins) {
      this.shownCoins = state.coins;
      this.coinLabel.textContent = this.texts.plural("hud.coins", state.coins);
    }
  }

  /** Writes every word again, in the language in use. Call it after the language changes. */
  refresh(): void {
    this.title.textContent = this.texts.t("app.title");
    this.dayBar.update({ label: this.texts.t("hud.dayBar") });
    this.menuButton.update({ label: this.texts.t("hud.menu") });
    for (const [def, chip] of this.chips) chip.update({ label: this.labelOf(def) });
    for (const hint of this.hints) hint.words.textContent = this.texts.t(hint.key);
    // Force the parts that depend on the language to be written again on the next update.
    this.shownDay = 0;
    this.shownPaused = null;
    this.shownCoins = -1;
  }
}
