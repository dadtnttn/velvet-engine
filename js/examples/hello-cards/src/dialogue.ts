// The dialogue plate: a guion conversation (@velvet/guion), shown box by box in a plate at the bottom
// of the table. A line waits for "Next" (the button, Enter or Space). A choice is a menu of its
// options; a closed option is shown but cannot be taken. The guion text is written in its own
// language; the words around it (the button, the errors) come from @velvet/i18n.

import type { Conversation, GuionLibrary, Shown, ShownChoice, ShownLine } from "@velvet/guion";
import type { I18n } from "@velvet/i18n";
import {
  createButton,
  createMenu,
  createPlate,
  type ButtonProps,
  type Menu,
  type Plate,
  type VuComponent,
} from "@velvet/ui";

/**
 * The plate. It is created once, appended to the parent, and hidden until a conversation starts.
 * `onEnd` runs when the conversation ends (its `fin`, or the end of its scene).
 */
export class Dialogue {
  private readonly texts: I18n;
  private readonly library: GuionLibrary;
  private readonly onEnd: () => void;
  private readonly onError: (error: unknown) => void;
  private readonly panel: HTMLElement;
  private readonly plate: Plate;
  private conversation: Conversation | null = null;
  private box: Shown | null = null;
  private button: VuComponent<ButtonProps> | null = null;
  private menu: Menu | null = null;

  /**
   * @param parent - the element that the plate is appended to (the stage's root).
   * @param texts - the game's texts, for the button and the labels.
   * @param library - the guiones of the game.
   * @param onEnd - runs when a conversation ends.
   * @param onError - runs with an error that the plate could not show.
   */
  constructor(
    parent: HTMLElement,
    texts: I18n,
    library: GuionLibrary,
    onEnd: () => void,
    onError: (error: unknown) => void,
  ) {
    this.texts = texts;
    this.library = library;
    this.onEnd = onEnd;
    this.onError = onError;
    this.panel = document.createElement("section");
    this.panel.className = "hc-dialogue";
    this.panel.hidden = true;
    this.plate = createPlate({ title: null });
    this.panel.append(this.plate.el);
    parent.append(this.panel);
  }

  /** True while a conversation is on screen. */
  get isOpen(): boolean {
    return this.conversation !== null;
  }

  /**
   * Starts a conversation at `scene` and shows its first box.
   * @throws {GuionError} when the scene does not exist.
   */
  start(scene: string): void {
    const conversation = this.library.start(scene);
    this.conversation = conversation;
    this.panel.hidden = false;
    this.show(conversation.current());
  }

  /** Goes on past the line on screen. Does nothing on a choice or when no conversation runs. */
  advance(): void {
    if (this.conversation === null || this.box?.type !== "line") return;
    this.show(this.conversation.next());
  }

  /** Rewrites the words of the plate in the language in use. The guion text stays as it is. */
  refresh(): void {
    this.button?.update({ label: this.texts.t("dialogue.next") });
  }

  /** Shows a box: a line with its button, a choice with its menu, or the end of the conversation. */
  private show(box: Shown): void {
    this.box = box;
    if (box.type === "end" || box.type === "vn") {
      this.finish();
      return;
    }
    this.clearBody();
    if (box.type === "line") this.showLine(box);
    else this.showChoice(box);
  }

  /** A line: its speaker's name on the tab, its text, and the button that goes on. */
  private showLine(line: ShownLine): void {
    this.plate.update({ title: line.name === "" ? null : line.name });
    const text = document.createElement("p");
    text.className = line.kind === "thought" ? "hc-line is-thought" : "hc-line";
    text.textContent = line.text;

    const row = document.createElement("div");
    row.className = "hc-dialogue-actions";
    this.button = createButton({ label: this.texts.t("dialogue.next"), onPress: () => this.advance() });
    row.append(this.button.el);
    this.plate.body.append(text, row);
  }

  /** A choice: its prompt, and a menu of its options. The first option that can be taken has the focus. */
  private showChoice(choice: ShownChoice): void {
    this.plate.update({ title: null });
    const prompt = document.createElement("p");
    prompt.className = "hc-prompt";
    prompt.textContent = choice.prompt ?? "";
    this.menu = createMenu({
      label: choice.prompt ?? "",
      items: choice.options.map((option, index) => ({
        id: String(index),
        label: option.label,
        disabled: !option.available,
      })),
      onSelect: (id) => this.choose(Number(id)),
    });
    this.menu.el.classList.add("hc-options");
    this.plate.body.append(prompt, this.menu.el);
    this.menu.focus(0);
  }

  /** Takes an option of the choice on screen. */
  private choose(index: number): void {
    if (this.conversation === null) return;
    try {
      this.show(this.conversation.choose(index));
    } catch (error) {
      this.onError(error);
    }
  }

  /** Hides the plate and tells the game that the conversation is over. */
  private finish(): void {
    this.clearBody();
    this.panel.hidden = true;
    this.conversation = null;
    this.box = null;
    this.onEnd();
  }

  /** Removes the button or the menu of the box on screen. */
  private clearBody(): void {
    this.button?.destroy();
    this.button = null;
    this.menu?.destroy();
    this.menu = null;
    this.plate.body.replaceChildren();
  }
}
