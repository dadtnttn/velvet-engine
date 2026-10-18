// @vitest-environment happy-dom
// Tests of the components that touch the page: the board's elements, the intro plate, and the pause
// menu. They check what the DOM holds, and how the keys behave.

import { afterEach, beforeAll, describe, expect, it, vi } from "vitest";
import { Board, loadCatalog } from "@velvet/cards";
import { ActionMap } from "@velvet/core";
import { GuionLibrary } from "@velvet/guion";
import { I18n, type CatalogFile } from "@velvet/i18n";
import { BoardView, type ViewLabels } from "./board-view";
import { LANGUAGES, LOCALE_URLS, type Language } from "./config";
import { Dialogue } from "./dialogue";
import { PauseMenu } from "./pause-menu";
import { loadWasmForTests, readPublic } from "./test-support";

beforeAll(async () => {
  await loadWasmForTests();
});

afterEach(() => {
  document.body.replaceChildren();
});

/** Both languages of the game, loaded the way the game loads them. */
async function loadTexts(): Promise<I18n> {
  const texts = new I18n({ source: "es", fallback: "en" });
  for (const language of LANGUAGES) {
    const files: CatalogFile[] = [];
    for (const url of LOCALE_URLS[language]) {
      files.push({ name: url.slice(url.lastIndexOf("/") + 1), text: await readPublic(url) });
    }
    texts.addLanguage(language, files);
  }
  return texts;
}

/** Plain labels for the view: the definitions name themselves. */
const plainLabels: ViewLabels = {
  cardName: (def) => def,
  cardKind: () => "resource",
  recipeName: (recipe) => recipe,
  progressLabel: "Progress",
};

/** The elements of a root that match a selector, as an array. */
function all(root: ParentNode, selector: string): HTMLElement[] {
  return Array.from(root.querySelectorAll<HTMLElement>(selector));
}

describe("the board view", () => {
  it("draws every card on its stack's place, and a bar under a stack that runs a job", async () => {
    const catalog = await loadCatalog(await readPublic("data/catalog.json"));
    const board = await Board.create(catalog, 1);
    const forest = board.spawn("forest", 0, 0);
    const villager = board.spawn("villager", 0, 1);
    board.dropOn(villager, forest);
    board.spawn("wood", 3, 0);

    const view = new BoardView(document.createElement("div"));
    view.render(board, plainLabels, null);

    expect(all(view.table, ".hc-card")).toHaveLength(3);
    const villagerCard = view.table.querySelector<HTMLElement>('[data-def="villager"]');
    expect(villagerCard?.style.top).toBe("34px");
    expect(villagerCard?.getAttribute("aria-label")).toBe("villager");
    expect(all(view.table, ".hc-job")).toHaveLength(1);

    // Taking the villager away stops the job: its card and its bar leave the table.
    board.remove(villager);
    view.render(board, plainLabels, null);
    expect(all(view.table, ".hc-card")).toHaveLength(2);
    expect(all(view.table, ".hc-job")).toHaveLength(0);
    board.free();
  });

  it("keeps the dragged stack above the others", async () => {
    const catalog = await loadCatalog(await readPublic("data/catalog.json"));
    const board = await Board.create(catalog, 2);
    const wood = board.spawn("wood", 0, 0);
    board.spawn("stone", 2, 0);
    const view = new BoardView(document.createElement("div"));
    view.render(board, plainLabels, wood);
    const woodCard = view.table.querySelector<HTMLElement>(`[data-card="${wood}"]`);
    expect(Number(woodCard?.style.zIndex)).toBeGreaterThan(900);
    board.free();
  });
});

describe("the intro plate", () => {
  it("plays lines with Next, offers the choice, and ends when an option leads to the end", async () => {
    const texts = await loadTexts();
    const library = await GuionLibrary.create({});
    library.add("intro.md", await readPublic("data/guion/intro.md"));

    const parent = document.createElement("div");
    document.body.append(parent);
    const onEnd = vi.fn();
    const onError = vi.fn();
    const dialogue = new Dialogue(parent, texts, library, onEnd, onError);
    const panel = parent.querySelector<HTMLElement>(".hc-dialogue");
    expect(panel?.hidden).toBe(true);

    dialogue.start("intro.inicio");
    expect(dialogue.isOpen).toBe(true);
    expect(panel?.hidden).toBe(false);
    expect(parent.querySelector(".hc-line")?.textContent).toContain("Soy Lupe");

    // Three lines, each with its Next button, and then the choice.
    for (let index = 0; index < 3; index += 1) {
      parent.querySelector<HTMLElement>(".hc-dialogue .vu-button")?.click();
    }
    expect(parent.querySelector(".hc-prompt")?.textContent).toBe("¿Qué te apetece?");
    expect(all(parent, '[role="menuitem"]')).toHaveLength(2);

    // Enter chooses the first option. The game's own keys must not act on that Enter: the menu uses it
    // (it prevents the default), and the action map does not count a press that a menu item took. The
    // press still reaches the window, which is why the check is on the map and not on the window.
    const keys = new ActionMap<"confirm">([{ action: "confirm", control: { type: "key", key: "Enter" } }]);
    keys.attach(window);
    const first = parent.querySelector<HTMLElement>('[role="menuitem"]');
    first?.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true, cancelable: true }));
    expect(keys.pressed("confirm")).toBe(false);
    keys.detach();
    expect(parent.querySelector(".hc-line")?.textContent).toContain("Arrastra una carta");

    // The controls scene jumps back to the choice; the second option ends the conversation.
    for (let index = 0; index < 5; index += 1) {
      parent.querySelector<HTMLElement>(".hc-dialogue .vu-button")?.click();
    }
    parent.querySelector<HTMLElement>('[data-id="1"]')?.click();
    expect(onEnd).toHaveBeenCalledTimes(1);
    expect(onError).not.toHaveBeenCalled();
    expect(dialogue.isOpen).toBe(false);
    expect(panel?.hidden).toBe(true);
  });
});

describe("the pause menu", () => {
  it("lists its choices in the language in use, and closes on Continue", async () => {
    const texts = await loadTexts();
    const parent = document.createElement("div");
    document.body.append(parent);
    const actions = {
      onSave: vi.fn(),
      onLoad: vi.fn(),
      onLanguage: vi.fn<(language: Language) => void>(),
      onFullscreen: vi.fn(),
      onQuit: vi.fn(),
    };
    let fullscreen = false;
    const menu = new PauseMenu(parent, texts, actions, () => ({ language: "es", fullscreen }));

    menu.open();
    expect(menu.isOpen).toBe(true);
    const labels = all(parent, '[role="menuitem"]').map((item) => item.textContent);
    expect(labels).toEqual([
      "Continuar",
      "Guardar partida",
      "Cargar partida",
      "Español (activo)",
      "English",
      "Pantalla completa: no",
      "Salir",
    ]);

    fullscreen = true;
    menu.refresh();
    expect(all(parent, '[role="menuitem"]').at(5)?.textContent).toBe("Pantalla completa: sí");

    parent.querySelector<HTMLElement>('[data-id="save"]')?.click();
    expect(actions.onSave).toHaveBeenCalledTimes(1);
    expect(menu.isOpen).toBe(true);

    parent.querySelector<HTMLElement>('[data-id="en"]')?.click();
    expect(actions.onLanguage).toHaveBeenCalledWith("en");

    parent.querySelector<HTMLElement>('[data-id="continue"]')?.click();
    expect(menu.isOpen).toBe(false);
  });
});
