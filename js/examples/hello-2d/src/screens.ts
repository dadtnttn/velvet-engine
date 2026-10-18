// The menus of hello-2d: the title, the pause menu, the settings and the result of a level. Each is a screen of the
// stack (createScreenStack in @velvet/ui), built from the ui's plates, menus and buttons. The game decides what a
// pick does; the builders show the words, the values and the picks.

import type { Ticker } from "@velvet/tween";
import { tween } from "@velvet/tween";
import {
  createButton,
  createMenu,
  createPlate,
  createSettingsPanel,
  type Menu,
  type Plate,
  type Screen,
  type SettingsPanel,
} from "@velvet/ui";
import { VOLUME_STEPS } from "./config";
import type { TranslateFn } from "./view";
import type { GameSettingsStore } from "./settings";
import { formatTime } from "./world";

/** A screen of the stack, and how its words are written again when the language changes. */
export interface ScreenKit {
  /** The screen to push onto the stack. */
  readonly screen: Screen;
  /** Writes the words in the language in use. */
  refresh(): void;
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
  el.className = `h2d-screen ${className}`;
  const inner = document.createElement("div");
  inner.className = "h2d-page";
  const plate = createPlate({ title: null });
  inner.append(plate.el);
  el.append(inner);
  return { el, inner, plate };
}

/**
 * Fades a page in, on real time: the screen's own transition moves the screen, and this is the page's entrance.
 * @param inner - the page that fades.
 * @param ticker - the real-time ticker that runs the fade.
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

/** The title screen's callbacks. */
export interface TitleCallbacks {
  readonly onPlay: () => void;
  readonly onSettings: () => void;
}

/**
 * Builds the title screen: the name of the game, a line about it, the keys, and the two choices.
 * @param translate - the game's text lookup.
 * @param ticker - the real-time ticker for the entrance.
 * @param callbacks - what the choices do.
 */
export function buildTitle(translate: TranslateFn, ticker: Ticker, callbacks: TitleCallbacks): ScreenKit {
  const page = makePage("h2d-title");
  const heading = document.createElement("h1");
  heading.className = "h2d-title-text";
  const sub = document.createElement("p");
  sub.className = "h2d-title-sub";
  const hint = document.createElement("p");
  hint.className = "h2d-title-hint";
  const menu: Menu = createMenu({
    label: "",
    items: [],
    onSelect: (id) => {
      if (id === "play") callbacks.onPlay();
      else if (id === "settings") callbacks.onSettings();
    },
  });
  page.plate.body.append(heading, sub, hint, menu.el);

  const paint = (): void => {
    heading.textContent = translate("app.title");
    sub.textContent = translate("title.sub");
    hint.textContent = translate("title.hint");
    menu.update({
      label: translate("app.title"),
      items: [
        { id: "play", label: translate("title.play") },
        { id: "settings", label: translate("title.settings") },
      ],
    });
  };
  paint();
  return {
    screen: { id: "title", el: page.el, onEnter: () => fadeIn(page.inner, ticker) },
    refresh: paint,
  };
}

/** The pause menu's callbacks. */
export interface PauseCallbacks {
  readonly onContinue: () => void;
  readonly onRestart: () => void;
  readonly onSettings: () => void;
  readonly onToTitle: () => void;
}

/**
 * Builds the pause menu, an overlay over the level.
 * @param translate - the game's text lookup.
 * @param ticker - the real-time ticker for the entrance.
 * @param callbacks - what the choices do.
 */
export function buildPause(translate: TranslateFn, ticker: Ticker, callbacks: PauseCallbacks): ScreenKit {
  const page = makePage("h2d-pause");
  const menu: Menu = createMenu({
    label: "",
    items: [],
    onSelect: (id) => {
      if (id === "continue") callbacks.onContinue();
      else if (id === "restart") callbacks.onRestart();
      else if (id === "settings") callbacks.onSettings();
      else if (id === "toTitle") callbacks.onToTitle();
    },
  });
  page.plate.body.append(menu.el);

  const paint = (): void => {
    page.plate.update({ title: translate("pause.title") });
    menu.update({
      label: translate("pause.title"),
      items: [
        { id: "continue", label: translate("pause.continue") },
        { id: "restart", label: translate("pause.restart") },
        { id: "settings", label: translate("pause.settings") },
        { id: "toTitle", label: translate("pause.toTitle") },
      ],
    });
  };
  paint();
  return {
    screen: { id: "pause", el: page.el, kind: "overlay", onEnter: () => fadeIn(page.inner, ticker) },
    refresh: paint,
  };
}

/** What the result of a level says: the time, the coins and the hits. */
export interface ResultView {
  /** Seconds of play. */
  readonly time: number;
  readonly coins: number;
  readonly total: number;
  readonly hits: number;
}

/** The result screen's callbacks. */
export interface ResultCallbacks {
  readonly onAgain: () => void;
  readonly onToTitle: () => void;
}

/**
 * Builds the result screen that shows when the flag is reached: the time, the coins and the hits, and two choices.
 * @param translate - the game's text lookup.
 * @param ticker - the real-time ticker for the entrance.
 * @param callbacks - what the choices do.
 */
export function buildResult(
  translate: TranslateFn,
  ticker: Ticker,
  callbacks: ResultCallbacks,
): ScreenKit & { show(view: ResultView): void } {
  const page = makePage("h2d-result");
  const heading = document.createElement("h2");
  heading.className = "h2d-result-heading";
  const lines = document.createElement("div");
  lines.className = "h2d-result-lines";
  const time = document.createElement("p");
  const coins = document.createElement("p");
  const hits = document.createElement("p");
  lines.append(time, coins, hits);
  const menu: Menu = createMenu({
    label: "",
    items: [],
    onSelect: (id) => {
      if (id === "again") callbacks.onAgain();
      else if (id === "toTitle") callbacks.onToTitle();
    },
  });
  page.plate.body.append(heading, lines, menu.el);
  let view: ResultView = { time: 0, coins: 0, total: 0, hits: 0 };

  const paint = (): void => {
    page.plate.update({ title: translate("hud.level") });
    heading.textContent = translate("complete.heading");
    time.textContent = translate("complete.time", { time: formatTime(view.time) });
    coins.textContent = translate("complete.coins", { coins: view.coins, total: view.total });
    hits.textContent = translate("complete.hits", { hits: view.hits });
    menu.update({
      label: translate("complete.heading"),
      items: [
        { id: "again", label: translate("complete.again") },
        { id: "toTitle", label: translate("complete.toTitle") },
      ],
    });
  };
  paint();
  return {
    screen: { id: "result", el: page.el, kind: "overlay", onEnter: () => fadeIn(page.inner, ticker) },
    refresh: paint,
    show(next: ResultView): void {
      view = next;
      paint();
    },
  };
}

/**
 * Builds the settings screen: the settings panel of @velvet/ui, which edits the effects volume and the language, and a
 * button that goes back. The panel holds the texts of one language, so it is made again when the language changes.
 * @param translate - the game's text lookup.
 * @param ticker - the real-time ticker for the entrance.
 * @param store - the settings.
 * @param onBack - runs when the back button is pressed.
 */
export function buildSettings(
  translate: TranslateFn,
  ticker: Ticker,
  store: GameSettingsStore,
  onBack: () => void,
): ScreenKit {
  const page = makePage("h2d-settings");
  const container = document.createElement("div");
  container.className = "h2d-settings-panel";
  const back = createButton({ label: "", onPress: onBack });
  page.plate.body.append(container, back.el);
  let panel: SettingsPanel | null = null;

  const build = (): void => {
    panel?.destroy();
    panel = createSettingsPanel({
      store,
      t: translate,
      sections: [
        {
          title: "settings.sound",
          items: [
            {
              kind: "slider",
              key: "sfxVolume",
              label: "settings.sfx",
              min: 0,
              max: VOLUME_STEPS,
              step: 1,
              format: (value: number) => `${value * 10} %`,
            },
          ],
        },
        {
          title: "settings.language",
          items: [
            {
              kind: "choice",
              key: "language",
              label: "settings.languageLabel",
              options: [
                { value: "es", label: "language.name.es" },
                { value: "en", label: "language.name.en" },
              ],
              wrap: false,
            },
          ],
        },
      ],
    });
    container.replaceChildren(panel.el);
  };

  const paint = (): void => {
    page.plate.update({ title: translate("settings.title") });
    back.update({ label: translate("settings.back") });
    build();
  };
  paint();
  return {
    screen: { id: "settings", el: page.el, kind: "overlay", onEnter: () => fadeIn(page.inner, ticker) },
    refresh: paint,
  };
}
