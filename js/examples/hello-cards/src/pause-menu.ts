// The pause menu: a plate over the dimmed table, with a menu of choices. It opens with Escape or the
// menu button, and closes with Continue or Escape. Its labels follow the language and the screen mode,
// so `refresh` rewrites them without closing the menu.

import type { I18n } from "@velvet/i18n";
import { createMenu, createPlate, type MenuItem, type MenuProps, type Plate, type VuComponent } from "@velvet/ui";
import { LANGUAGE_NAMES, LANGUAGES, type Language } from "./config";
import { keepKeysInMenu } from "./keys";

/** What the menu's choices do. Continue is handled by the menu itself. */
export interface PauseActions {
  /** Writes the game to the manual save slot. */
  readonly onSave: () => void;
  /** Reads the game from the manual save slot. */
  readonly onLoad: () => void;
  /** Switches the language in use. */
  readonly onLanguage: (language: Language) => void;
  /** Switches full screen on or off. */
  readonly onFullscreen: () => void;
  /** Closes the game. */
  readonly onQuit: () => void;
}

/** The settings that the labels show. */
export interface PauseState {
  /** The language in use. */
  readonly language: Language;
  /** Whether the window is full screen. */
  readonly fullscreen: boolean;
}

/** The pause menu. It starts closed. */
export class PauseMenu {
  private readonly texts: I18n;
  private readonly actions: PauseActions;
  private readonly stateOf: () => PauseState;
  private readonly overlay: HTMLElement;
  private readonly plate: Plate;
  private menu: VuComponent<MenuProps> | null = null;

  /**
   * @param parent - the element that the overlay is appended to (the stage's root).
   * @param texts - the game's texts.
   * @param actions - what the choices do.
   * @param stateOf - the settings that the labels show, read each time the labels are written.
   */
  constructor(parent: HTMLElement, texts: I18n, actions: PauseActions, stateOf: () => PauseState) {
    this.texts = texts;
    this.actions = actions;
    this.stateOf = stateOf;
    this.overlay = document.createElement("div");
    this.overlay.className = "vu-modal hc-pause";
    this.overlay.hidden = true;
    this.plate = createPlate({ title: texts.t("pause.title") });
    this.overlay.append(this.plate.el);
    parent.append(this.overlay);
  }

  /** True while the menu is open. */
  get isOpen(): boolean {
    return this.menu !== null;
  }

  /** Opens the menu, with its first choice focused. Does nothing when it is open already. */
  open(): void {
    if (this.menu !== null) return;
    this.plate.update({ title: this.texts.t("pause.title") });
    this.menu = createMenu({
      label: this.texts.t("pause.title"),
      items: this.items(),
      onSelect: (id) => this.select(id),
    });
    keepKeysInMenu(this.menu.el);
    this.plate.body.replaceChildren(this.menu.el);
    this.overlay.hidden = false;
    this.menu.el.querySelector<HTMLElement>('[role="menuitem"]')?.focus();
  }

  /** Closes the menu. Focus goes back to the page. */
  close(): void {
    if (this.menu === null) return;
    this.menu.destroy();
    this.menu = null;
    this.plate.body.replaceChildren();
    this.overlay.hidden = true;
  }

  /** Rewrites the labels in the language in use, and the state of full screen. */
  refresh(): void {
    this.plate.update({ title: this.texts.t("pause.title") });
    this.menu?.update({ label: this.texts.t("pause.title"), items: this.items() });
  }

  /** The choices, in order, with labels for the current state. */
  private items(): MenuItem[] {
    const state = this.stateOf();
    const languages: MenuItem[] = LANGUAGES.map((language) => ({
      id: language,
      label:
        language === state.language
          ? this.texts.t("pause.langOn", { name: LANGUAGE_NAMES[language] })
          : LANGUAGE_NAMES[language],
    }));
    return [
      { id: "continue", label: this.texts.t("pause.continue") },
      { id: "save", label: this.texts.t("pause.save") },
      { id: "load", label: this.texts.t("pause.load") },
      ...languages,
      { id: "fullscreen", label: this.texts.t(state.fullscreen ? "pause.fullscreen.on" : "pause.fullscreen.off") },
      { id: "quit", label: this.texts.t("pause.quit") },
    ];
  }

  /** A choice was taken. Continue closes the menu; the others keep it open, unless they close the game. */
  private select(id: string): void {
    switch (id) {
      case "continue":
        this.close();
        return;
      case "save":
        this.actions.onSave();
        return;
      case "load":
        this.actions.onLoad();
        return;
      case "fullscreen":
        this.actions.onFullscreen();
        return;
      case "quit":
        this.actions.onQuit();
        return;
      default: {
        const language = LANGUAGES.find((candidate) => candidate === id);
        if (language !== undefined) this.actions.onLanguage(language);
      }
    }
  }
}
