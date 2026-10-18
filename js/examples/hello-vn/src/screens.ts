// The menus of hello-vn: the title, the pause menu, the settings, the save list and the ending card. Each is a
// screen of the stack (createScreenStack in @velvet/ui), built from the ui's plates, menus and buttons. The game
// decides what a pick does; the builders show the words, the values and the picks.

import type { I18n } from "@velvet/i18n";
import type { Ticker } from "@velvet/tween";
import { tween } from "@velvet/tween";
import {
  createButton,
  createMenu,
  createPlate,
  type Menu,
  type MenuItem,
  type Plate,
  type SaveSlots,
  type Screen,
} from "@velvet/ui";
import type { Art } from "./art";
import { ART, LANGUAGE_NAMES, type Language } from "./config";
import type { GameSettings } from "./settings";

/** A screen of the stack, and how its words are written again when the language changes. */
export interface ScreenKit {
  /** The screen to push onto the stack. */
  readonly screen: Screen;
  /** Writes the words in the language in use, and the values again. */
  refresh(): void;
}

/** The ids of the settings rows, as the settings menu reports them. */
export type SettingId = "speed" | "delay" | "music" | "sfx" | "language" | "back";

/** What the settings menu shows: the values, and the language in use. */
export interface SettingsView {
  readonly settings: GameSettings;
  readonly language: Language;
}

/** The text keys of each ending, by the number the story stores in its variable. */
const ENDING_KEYS: Readonly<Record<number, { readonly title: string; readonly body: string }>> = {
  1: { title: "ending.luz.title", body: "ending.luz.body" },
  2: { title: "ending.campana.title", body: "ending.campana.body" },
};

/** The ending that the card shows, and how many boxes the player read to reach it. */
export interface EndingView {
  readonly ending: number;
  readonly boxes: number;
}

/** A page of a menu screen: the root element of the screen, the part that fades, and the plate. */
interface Page {
  readonly el: HTMLElement;
  readonly inner: HTMLElement;
  readonly plate: Plate;
}

/**
 * Makes the page of a menu screen: a plate in the middle of the screen, which the screen's fade moves.
 * @param className - the class of the screen's root, for the stylesheet.
 */
function makePage(className: string): Page {
  const el = document.createElement("section");
  el.className = `vn-screen-root ${className}`;
  const inner = document.createElement("div");
  inner.className = "vn-page";
  const plate = createPlate({ title: null });
  inner.append(plate.el);
  el.append(inner);
  return { el, inner, plate };
}

/**
 * Fades a page in, with a tween on real time. The screen's own transition only moves the screen; this one is
 * the page's entrance.
 * @param inner - the page that fades.
 * @param ticker - the real-time ticker that runs the tween.
 */
function fadeIn(inner: HTMLElement, ticker: Ticker): void {
  tween({
    from: 0,
    to: 1,
    duration: 0.35,
    ease: "quadOut",
    ticker,
    set: (value) => {
      inner.style.opacity = String(value);
    },
  });
}

/** The percentage that a volume shows, such as 50 for 0.5. */
function percent(volume: number): string {
  return String(Math.round(volume * 100));
}

/** The title screen: the game's name, and the three choices of a start. */
export interface TitleCallbacks {
  readonly onNew: () => void;
  readonly onLoad: () => void;
  readonly onSettings: () => void;
}

/**
 * Builds the title screen.
 * @param texts - the game's texts.
 * @param art - the art, for the title's picture.
 * @param ticker - the real-time ticker for the entrance.
 * @param callbacks - what the choices do.
 */
export function buildTitle(texts: I18n, art: Art, ticker: Ticker, callbacks: TitleCallbacks): ScreenKit {
  const page = makePage("vn-title");
  const picture = art.picture(ART.title);
  picture.classList.add("vn-bg");
  page.el.prepend(picture);

  const heading = document.createElement("h1");
  heading.className = "vn-title-text";
  const sub = document.createElement("p");
  sub.className = "vn-title-sub";
  const menu: Menu = createMenu({
    label: "",
    items: [],
    onSelect: (id) => {
      if (id === "new") callbacks.onNew();
      else if (id === "load") callbacks.onLoad();
      else if (id === "settings") callbacks.onSettings();
    },
  });
  page.plate.body.append(heading, sub, menu.el);

  const paint = (): void => {
    heading.textContent = texts.t("app.title");
    sub.textContent = texts.t("title.sub");
    const items: MenuItem[] = [
      { id: "new", label: texts.t("title.new") },
      { id: "load", label: texts.t("title.load") },
      { id: "settings", label: texts.t("title.settings") },
    ];
    menu.update({ label: texts.t("app.title"), items });
  };
  paint();
  return {
    screen: { id: "title", el: page.el, onEnter: () => fadeIn(page.inner, ticker) },
    refresh: paint,
  };
}

/** The pause menu: continue, the save list and the load list, the settings, and the way back to the title. */
export interface PauseCallbacks {
  readonly onContinue: () => void;
  readonly onSave: () => void;
  readonly onLoad: () => void;
  readonly onSettings: () => void;
  readonly onToTitle: () => void;
}

/**
 * Builds the pause menu, an overlay over the game.
 * @param texts - the game's texts.
 * @param ticker - the real-time ticker for the entrance.
 * @param callbacks - what the choices do.
 */
export function buildPause(texts: I18n, ticker: Ticker, callbacks: PauseCallbacks): ScreenKit {
  const page = makePage("vn-pause");
  const menu: Menu = createMenu({
    label: "",
    items: [],
    onSelect: (id) => {
      if (id === "continue") callbacks.onContinue();
      else if (id === "save") callbacks.onSave();
      else if (id === "load") callbacks.onLoad();
      else if (id === "settings") callbacks.onSettings();
      else if (id === "toTitle") callbacks.onToTitle();
    },
  });
  page.plate.body.append(menu.el);

  const paint = (): void => {
    page.plate.update({ title: texts.t("pause.title") });
    menu.update({
      label: texts.t("pause.title"),
      items: [
        { id: "continue", label: texts.t("pause.continue") },
        { id: "save", label: texts.t("pause.save") },
        { id: "load", label: texts.t("pause.load") },
        { id: "settings", label: texts.t("pause.settings") },
        { id: "toTitle", label: texts.t("pause.toTitle") },
      ],
    });
  };
  paint();
  return {
    screen: { id: "pause", el: page.el, kind: "overlay", onEnter: () => fadeIn(page.inner, ticker) },
    refresh: paint,
  };
}

/**
 * Builds the settings menu, an overlay. Each row shows its value; a pick reports the row, and the game changes
 * the value and calls `refresh` again.
 * @param texts - the game's texts.
 * @param ticker - the real-time ticker for the entrance.
 * @param read - the values to show, read on each refresh.
 * @param onPick - runs with the row that was picked.
 */
export function buildSettings(
  texts: I18n,
  ticker: Ticker,
  read: () => SettingsView,
  onPick: (id: SettingId) => void,
): ScreenKit {
  const page = makePage("vn-settings");
  const menu: Menu = createMenu({
    label: "",
    items: [],
    onSelect: (id) => onPick(id as SettingId),
  });
  page.plate.body.append(menu.el);

  const paint = (): void => {
    const { settings, language } = read();
    const speed = settings.textSpeed === 0 ? texts.t("settings.instant") : String(settings.textSpeed);
    page.plate.update({ title: texts.t("settings.title") });
    menu.update({
      label: texts.t("settings.title"),
      items: [
        { id: "speed", label: texts.t("settings.speed", { value: speed }) },
        { id: "delay", label: texts.t("settings.autoDelay", { value: String(settings.autoDelay) }) },
        { id: "music", label: texts.t("settings.music", { value: percent(settings.musicVolume) }) },
        { id: "sfx", label: texts.t("settings.sfx", { value: percent(settings.sfxVolume) }) },
        { id: "language", label: texts.t("settings.language", { name: LANGUAGE_NAMES[language] }) },
        { id: "back", label: texts.t("settings.back") },
      ],
    });
  };
  paint();
  return {
    screen: { id: "settings", el: page.el, kind: "overlay", onEnter: () => fadeIn(page.inner, ticker) },
    refresh: paint,
  };
}

/**
 * Builds the save list screen around the list of slots that the game made. The mode decides the title.
 * @param texts - the game's texts.
 * @param ticker - the real-time ticker for the entrance.
 * @param slots - the list of slots.
 * @param onBack - runs when the back button is pressed.
 */
export function buildSaves(
  texts: I18n,
  ticker: Ticker,
  slots: SaveSlots,
  onBack: () => void,
): ScreenKit & { setMode(mode: "save" | "load"): void } {
  const page = makePage("vn-saves");
  const back = createButton({ label: "", onPress: onBack });
  page.plate.body.append(slots.el, back.el);
  let mode: "save" | "load" = "load";

  const paint = (): void => {
    page.plate.update({ title: texts.t(mode === "save" ? "pause.save" : "pause.load") });
    back.update({ label: texts.t("settings.back") });
  };
  paint();
  return {
    screen: {
      id: "saves",
      el: page.el,
      kind: "overlay",
      onEnter: () => fadeIn(page.inner, ticker),
      focusTarget: () => slots.focusTarget() ?? back.el,
    },
    refresh: paint,
    setMode(next: "save" | "load"): void {
      mode = next;
      paint();
    },
  };
}

/**
 * Builds the ending card: the ending's title and text, and how many boxes the player read. Escape does not close
 * it; its button goes back to the title.
 * @param texts - the game's texts.
 * @param ticker - the real-time ticker for the entrance.
 * @param onBack - runs when the button is pressed.
 */
export function buildEnding(
  texts: I18n,
  ticker: Ticker,
  onBack: () => void,
): ScreenKit & { show(view: EndingView): void } {
  const page = makePage("vn-ending");
  const title = document.createElement("h2");
  title.className = "vn-ending-title";
  const body = document.createElement("p");
  body.className = "vn-ending-body";
  const boxes = document.createElement("p");
  boxes.className = "vn-ending-boxes";
  const back = createButton({ label: "", onPress: onBack });
  page.plate.body.append(title, body, boxes, back.el);
  let view: EndingView = { ending: 1, boxes: 0 };

  const paint = (): void => {
    page.plate.update({ title: texts.t("ending.heading") });
    const keys = ENDING_KEYS[view.ending];
    title.textContent = keys === undefined ? "" : texts.t(keys.title);
    body.textContent = keys === undefined ? "" : texts.t(keys.body);
    boxes.textContent = texts.plural("ending.boxes", view.boxes);
    back.update({ label: texts.t("ending.back") });
  };
  paint();
  return {
    screen: {
      id: "ending",
      el: page.el,
      kind: "overlay",
      onEnter: () => fadeIn(page.inner, ticker),
      onBack: () => false,
    },
    refresh: paint,
    show(next: EndingView): void {
      view = next;
      paint();
    },
  };
}
