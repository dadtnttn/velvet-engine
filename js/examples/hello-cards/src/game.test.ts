// @vitest-environment happy-dom
// The whole game in a page, with real timers: the intro, a villager gathering through the live loop,
// Space and Escape, and the quick save and quick load. The data files come from the public folder.
// The saves are checked through the notices the player sees: the storage of the test page is not a
// browser storage, so its contents are not inspected here.

import { afterAll, beforeAll, describe, expect, it, vi } from "vitest";
import { Game } from "./game";
import {
  centreOf,
  fetchFromPublicFolder,
  firePointer,
  loadWasmForTests,
  must,
  pressKey,
  stubLayout,
  waitUntil,
} from "./test-support";

/** The cards of a definition on the table, as drawn. */
function cardsOf(def: string): HTMLElement[] {
  return Array.from(document.querySelectorAll<HTMLElement>(`.hc-card[data-def="${def}"]`));
}

/** The text of the notices on screen, all of them together. */
function noticesText(): string {
  return document.querySelector(".vu-toasts")?.textContent ?? "";
}

/** The text of the HUD's pause button: "Seguir" while the clock is paused, "Pausa" while it runs. */
function pauseLabel(): string {
  const buttons = Array.from(document.querySelectorAll<HTMLElement>(".hc-speed .vu-button"));
  return buttons[buttons.length - 1]?.querySelector(".vu-button-label")?.textContent ?? "";
}

/** A keydown sent to one element, as a keyboard sends it to the element that has the focus. */
function keyDownOn(target: EventTarget, key: string, code: string): void {
  target.dispatchEvent(new KeyboardEvent("keydown", { key, code, bubbles: true, cancelable: true }));
}

/** Lets the game loop run for a few frames, so that the keys pressed so far have been read. */
function settle(): Promise<void> {
  return new Promise((done) => setTimeout(done, 150));
}

beforeAll(async () => {
  await loadWasmForTests();
  fetchFromPublicFolder();
});

afterAll(() => {
  vi.unstubAllGlobals();
});

describe("the game in a page", () => {
  it("plays its intro, gathers wood through the live loop, pauses, and saves and loads", async () => {
    stubLayout();
    const host = document.createElement("div");
    host.id = "game";
    document.body.append(host);
    const game = await Game.start(host);
    try {
      // The intro: three lines, then the choice. Its second option ends the conversation.
      const panel = must<HTMLElement>(document, ".hc-dialogue");
      expect(panel.hidden).toBe(false);
      for (let index = 0; index < 3; index += 1) must<HTMLElement>(panel, ".vu-button").click();
      must<HTMLElement>(panel, '[data-id="1"]').click();
      expect(panel.hidden).toBe(true);

      // A villager dropped on a forest gathers: one more piece of wood after the recipe's two seconds.
      const woodBefore = cardsOf("wood").length;
      const villager = centreOf(must<HTMLElement>(document, '.hc-card[data-def="villager"]'));
      const forest = centreOf(must<HTMLElement>(document, '.hc-card[data-def="forest"]'));
      const villagerCard = must<HTMLElement>(document, '.hc-card[data-def="villager"]');
      firePointer(villagerCard, "pointerdown", villager.x, villager.y);
      firePointer(window, "pointermove", villager.x + 20, villager.y - 60);
      firePointer(window, "pointermove", forest.x, forest.y);
      firePointer(window, "pointerup", forest.x, forest.y);
      await waitUntil(() => cardsOf("wood").length > woodBefore);

      // Space pauses the clock, and the pause button then offers to resume.
      pressKey(" ", "Space");
      await waitUntil(() => pauseLabel() === "Seguir");
      pressKey(" ", "Space");
      await waitUntil(() => pauseLabel() === "Pausa");

      // Escape opens the pause menu, and Continue closes it.
      pressKey("Escape", "Escape");
      await waitUntil(() => !must<HTMLElement>(document, ".hc-pause").hidden);
      must<HTMLElement>(document, '[data-id="continue"]').click();
      expect(must<HTMLElement>(document, ".hc-pause").hidden).toBe(true);

      // F5 quick-saves, and the game says so.
      pressKey("F5", "F5");
      await waitUntil(() => noticesText().includes("Partida guardada"));

      // F9 loads it back, and the game says so.
      pressKey("F9", "F9");
      await waitUntil(() => noticesText().includes("Partida cargada"));
      expect(cardsOf("wood").length).toBeGreaterThan(0);
    } finally {
      game.dispose();
    }
  });

  it("lets a focused control keep Space, and keeps the shortcuts working while it has the focus", async () => {
    stubLayout();
    const host = document.createElement("div");
    document.body.append(host);
    const game = await Game.start(host);
    try {
      // Finish the intro: three lines, then the choice that ends it.
      const panel = must<HTMLElement>(document, ".hc-dialogue");
      for (let index = 0; index < 3; index += 1) must<HTMLElement>(panel, ".vu-button").click();
      must<HTMLElement>(panel, '[data-id="1"]').click();

      // Escape opens the pause menu, and its first item ("Continue") takes the focus. Space on that item
      // chooses it: the menu closes, and the clock keeps running, because the menu kept the key.
      pressKey("Escape", "Escape");
      await waitUntil(() => !must<HTMLElement>(document, ".hc-pause").hidden);
      const continueItem = must<HTMLElement>(document, '[data-id="continue"]');
      expect(document.activeElement).toBe(continueItem);
      keyDownOn(continueItem, " ", "Space");
      expect(must<HTMLElement>(document, ".hc-pause").hidden).toBe(true);
      await settle();
      expect(pauseLabel()).toBe("Pausa");

      // The HUD's pause button takes the focus. Space on it does not toggle the clock from the key map (in a
      // browser, the button's own click does that, once), while F5 still quick-saves.
      const buttons = Array.from(document.querySelectorAll<HTMLElement>(".hc-speed .vu-button"));
      const pauseButton = buttons[buttons.length - 1];
      if (pauseButton === undefined) throw new Error("the HUD has no pause button");
      pauseButton.focus();
      keyDownOn(pauseButton, " ", "Space");
      await settle();
      expect(pauseLabel()).toBe("Pausa");
      keyDownOn(pauseButton, "F5", "F5");
      await waitUntil(() => noticesText().includes("Partida guardada"));
    } finally {
      game.dispose();
      host.remove();
    }
  });

  it("mounts the developer tools in a development build, and removes them when the game is disposed", async () => {
    expect(import.meta.env.DEV).toBe(true);
    stubLayout();
    const host = document.createElement("div");
    document.body.append(host);
    const game = await Game.start(host);
    try {
      expect(host.querySelector(".vdt-root")).not.toBeNull();
    } finally {
      game.dispose();
      expect(host.querySelector(".vdt-root")).toBeNull();
      host.remove();
    }
  });
});
