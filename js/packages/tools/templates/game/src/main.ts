// The entry point of the game: the stage, the texts, the save slot and the game loop. The rules are in
// game.ts; this file draws the two screens and reacts to the buttons.

import "@velvet/ui/velvet-ui.css";
import "./style.css";
import { GameLoop, createSaveStore } from "@velvet/core";
import { I18n } from "@velvet/i18n";
import { createButton, createStage } from "@velvet/ui";
import { fromSave, newGame, SAVE_VERSION, takeTurn, toSave, type GameState } from "./game";

/** The game's id. It must equal the `id` in velvet-game.json: saves and logs use it. */
const GAME_ID = "{{GAME_ID}}";

/** The save slot that the game writes and reads. */
const SLOT = "slot-1";

/** The languages the game ships. Spanish is the source: its catalog holds every id. */
const LANGUAGES = ["es", "en"] as const;

/** The two screens. */
type Screen = "title" | "game";

/**
 * The text of an error, for a notice on the screen.
 * @param error - what was thrown.
 */
function reasonOf(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

/** Starts the game inside the page's host element. */
async function start(host: HTMLElement): Promise<void> {
  const texts = new I18n({
    source: "es",
    fallback: "en",
    onMissing: (key, language) => console.warn(`no text for ${key} in ${language}`),
  });
  for (const language of LANGUAGES) {
    await texts.addLanguageFromUrls(language, [`data/locales/${language}/00_ui.json`]);
  }
  texts.setLanguage(navigator.language.startsWith("en") ? "en" : "es");

  const saves = createSaveStore({ game: GAME_ID, version: SAVE_VERSION });
  const stage = createStage(host, { width: 1920, height: 1080 });
  const panel = document.createElement("main");
  panel.className = "panel";
  stage.root.append(panel);

  let screen: Screen = "title";
  let state: GameState = newGame();
  let saved = (await saves.list()).includes(SLOT);
  let message = "";
  let dirty = true;

  const text = (tag: "h1" | "p", content: string): HTMLElement => {
    const element = document.createElement(tag);
    element.textContent = content;
    return element;
  };
  const button = (label: string, onPress: () => void, disabled = false): HTMLElement =>
    createButton({ label, disabled, onPress }).el;

  const resume = async (): Promise<void> => {
    try {
      const restored = fromSave((await saves.load(SLOT))?.data);
      if (restored === null) {
        saved = false;
      } else {
        state = restored;
        screen = "game";
        message = "";
      }
    } catch (error) {
      message = texts.t("error.load", { reason: reasonOf(error) });
    }
    dirty = true;
  };

  const save = async (): Promise<void> => {
    try {
      await saves.save(SLOT, toSave(state));
      saved = true;
      message = texts.t("game.saved");
    } catch (error) {
      message = texts.t("error.save", { reason: reasonOf(error) });
    }
    dirty = true;
  };

  const paint = (): void => {
    dirty = false;
    const items: HTMLElement[] = [];
    if (screen === "title") {
      items.push(
        text("h1", texts.t("title.heading")),
        text("p", texts.t("title.subtitle")),
        button(texts.t("title.new"), () => {
          state = newGame();
          screen = "game";
          message = "";
          dirty = true;
        }),
        button(texts.t("title.continue"), () => void resume(), !saved),
        button(texts.t("title.language"), () => {
          texts.setLanguage(texts.language === "es" ? "en" : "es");
        }),
      );
    } else {
      items.push(
        text("p", texts.plural("game.turns", state.turns)),
        button(texts.t("game.turn"), () => {
          state = takeTurn(state);
          dirty = true;
        }),
        button(texts.t("game.save"), () => void save()),
        button(texts.t("game.menu"), () => {
          screen = "title";
          message = "";
          dirty = true;
        }),
      );
      if (message !== "") items.push(text("p", message));
    }
    panel.replaceChildren(...items);
  };

  // A new language repaints the screen, in the next frame.
  texts.onChange(() => {
    dirty = true;
  });

  // The rules run from the buttons, so the steps have nothing to do; a frame repaints when it is dirty.
  const loop = new GameLoop({
    update: () => undefined,
    render: () => {
      if (dirty) paint();
    },
  });
  loop.start();
}

const host = document.getElementById("game");
if (host === null) {
  throw new Error(`${GAME_ID}: the page has no #game element`);
}
start(host).catch((error: unknown) => {
  const notice = document.createElement("p");
  notice.setAttribute("role", "alert");
  notice.textContent = `The game could not start: ${reasonOf(error)}`;
  host.replaceChildren(notice);
});
