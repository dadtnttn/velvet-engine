// The look of the game screen around the dialogue box: the background (which crossfades when the stage changes),
// the top bar (the place, the sea glass and the menu button) and the key hints at the bottom. The dialogue box
// itself is appended by the game. Every word comes from the texts, and every number from the game.

import type { I18n } from "@velvet/i18n";
import type { Ticker } from "@velvet/tween";
import { tween } from "@velvet/tween";
import { createButton, createChip, createKeycap, type ButtonProps, type ChipProps, type KeycapProps, type VuComponent } from "@velvet/ui";
import type { Art } from "./art";
import { SCENE_KEYS } from "./texts";

/** How long a background takes to fade in, in seconds of real time. */
const FADE_SECONDS = 0.6;

/** The hints of the bottom line: the keys to draw, and the text key of the words after them. */
const HINTS: ReadonlyArray<{ readonly key: string; readonly text: string }> = [
  { key: "game.key.next", text: "game.hint.next" },
  { key: "game.key.menu", text: "game.hint.menu" },
  { key: "game.key.fast", text: "game.hint.fast" },
];

/** The game screen's own look. `el` is the screen's element; the game appends the dialogue box to it. */
export class GameView {
  /** The element of the game screen. */
  readonly el: HTMLElement;
  private readonly art: Art;
  private readonly texts: I18n;
  private readonly layer: HTMLElement;
  private readonly place: VuComponent<ChipProps>;
  private readonly glass: VuComponent<ChipProps>;
  private readonly menu: VuComponent<ButtonProps>;
  /** The keycaps of the key hints, in the order of HINTS. */
  private readonly hintKeys: VuComponent<KeycapProps>[] = [];
  private readonly hintWords: HTMLElement[] = [];
  private placeLabel: string | null = null;
  private backgroundId: string | null = null;
  private picture: Element | null = null;

  /**
   * @param art - the art of the game, for the backgrounds.
   * @param texts - the game's texts.
   * @param onMenu - runs when the menu button is pressed.
   */
  constructor(art: Art, texts: I18n, onMenu: () => void) {
    this.art = art;
    this.texts = texts;
    this.el = document.createElement("section");
    this.el.className = "vn-game";

    this.layer = document.createElement("div");
    this.layer.className = "vn-backdrop";

    const bar = document.createElement("header");
    bar.className = "vn-topbar";
    this.place = createChip({ label: "", count: null });
    this.glass = createChip({ label: "", count: 0 });
    this.menu = createButton({ label: "", hotkey: "Escape", onPress: onMenu });
    bar.append(this.place.el, this.glass.el, this.menu.el);

    const hints = document.createElement("footer");
    hints.className = "vn-hints";
    for (let index = 0; index < HINTS.length; index += 1) {
      const item = document.createElement("span");
      item.className = "vn-hint";
      const words = document.createElement("span");
      words.className = "vn-hint-text";
      // The key's name is a text of the language in use ("Intro" in Spanish), shown as written.
      const keycap = createKeycap({ key: "", textCase: "as-is" });
      item.append(keycap.el, words);
      hints.append(item);
      this.hintKeys.push(keycap);
      this.hintWords.push(words);
    }
    this.el.append(this.layer, bar, hints);
    this.refresh();
  }

  /**
   * Shows a background. A new one fades in over the old one, which is removed when the fade ends. The same id
   * again changes nothing.
   * @param id - the art id of the background, or null for none.
   * @param ticker - the real-time ticker that runs the fade.
   */
  setBackground(id: string | null, ticker: Ticker): void {
    if (id === this.backgroundId) return;
    this.backgroundId = id;
    const previous = this.picture;
    this.picture = null;
    if (id === null) {
      previous?.remove();
      return;
    }
    const picture = this.art.picture(id);
    picture.classList.add("vn-bg");
    picture.style.opacity = "0";
    this.layer.append(picture);
    this.picture = picture;
    tween({
      from: 0,
      to: 1,
      duration: FADE_SECONDS,
      ease: "quadOut",
      ticker,
      set: (value) => {
        picture.style.opacity = String(value);
      },
      onComplete: () => previous?.remove(),
    });
  }

  /**
   * Names the place of the scene on screen.
   * @param scene - the label of the scene, or null for none.
   */
  setPlace(scene: string | null): void {
    this.placeLabel = scene;
    this.paintPlace();
  }

  /**
   * Shows the sea glass that the player carries: a badge with a count of 1, or none.
   * @param count - how many pieces the player holds.
   */
  setGlass(count: number): void {
    this.glass.update({ count });
  }

  /** Writes the words of the screen again, in the language in use. */
  refresh(): void {
    this.menu.update({ label: this.texts.t("game.menu") });
    this.glass.update({ label: this.texts.t("game.item.lente") });
    HINTS.forEach((hint, index) => {
      const words = this.hintWords[index];
      if (words !== undefined) words.textContent = this.texts.t(hint.text);
      this.hintKeys[index]?.update({ key: this.texts.t(hint.key) });
    });
    this.paintPlace();
  }

  /** Writes the name of the place, in the language in use. */
  private paintPlace(): void {
    const scene = this.placeLabel;
    if (scene === null) {
      this.place.update({ label: "" });
      return;
    }
    const key = SCENE_KEYS[scene];
    this.place.update({ label: key === undefined ? scene : this.texts.t(key) });
  }
}
