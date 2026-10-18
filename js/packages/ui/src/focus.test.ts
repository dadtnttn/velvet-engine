// @vitest-environment happy-dom
// Tests for the keyboard focus when a part of the interface is hidden or removed. A hidden element must never
// keep the focus: a game found its F3 key dead because the focus sat on a hidden button. Each surface is driven
// to the state where it holds the focus, then hidden or destroyed, and the test checks that the focus is outside
// it. The helper itself, releaseFocus, is tested first.

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { createBoardView } from "./board";
import { releaseFocus } from "./components/dom";
import { createButton } from "./components/button";
import { createMenu } from "./components/menu";
import { createModal } from "./components/modal";
import { createPlate } from "./components/plate";
import { createDialogueBox, type DialogueChoice, type DialogueLine, type DialogueSession, type DialogueShown } from "./dialogue-box";
import { setShown } from "./dirty";
import { createNotices } from "./notices";
import { createRebindPanel, type RebindPort } from "./rebind-panel";
import { createSaveSlots, type SaveSlotStore } from "./save-slots";
import { createScreenStack, type Screen } from "./screens";
import { createStage } from "./stage";
import { installFakeGeometry, setWindowSize } from "./test-support";

let host: HTMLElement;

beforeEach(() => {
  installFakeGeometry();
  setWindowSize(1920, 1080);
  host = document.createElement("div");
  document.body.append(host);
});

afterEach(() => {
  vi.restoreAllMocks();
  document.body.innerHTML = "";
});

/** True when the focus is on an element inside `container`. */
function focusInside(container: Element): boolean {
  return container.contains(document.activeElement);
}

/** A button in the document, with a label. */
function button(label: string, parent: HTMLElement = host): HTMLButtonElement {
  const node = document.createElement("button");
  node.type = "button";
  node.textContent = label;
  parent.append(node);
  return node;
}

/** A screen with one button, for the screen stack. */
function screenWith(id: string): { screen: Screen; el: HTMLElement } {
  const el = document.createElement("div");
  button(`${id}-action`, el);
  return { screen: { id, el }, el };
}

/** A line of dialogue. */
function line(text: string): DialogueLine {
  return { type: "line", who: "ana", name: "Ana", text, mood: null };
}

/** A choice with one option that can be taken. */
function choice(): DialogueChoice {
  return { type: "choice", prompt: null, options: [{ label: "Ir", available: true }] };
}

/** A session that plays a list of boxes in order, and is over past the list. */
function sessionOf(items: readonly DialogueShown[]): DialogueSession {
  let index = 0;
  const at = (): DialogueShown => items[index] ?? { type: "end" };
  return {
    current: at,
    next: () => {
      index += 1;
      return at();
    },
    choose: () => {
      index += 1;
      return at();
    },
  };
}

/** A save store with one manual save in slot-1, held in memory. */
function storeWithSave(): SaveSlotStore {
  const data = new Map<string, unknown>([["slot-1", { meta: { title: "Bosque", playTimeMs: 60000 }, state: {} }]]);
  return {
    async list() {
      return [...data.keys()];
    },
    async load(name) {
      return data.has(name) ? { savedAt: "2026-10-18T10:00:00.000Z", data: data.get(name) } : null;
    },
    async save(name, value) {
      data.set(name, value);
    },
    async remove(name) {
      data.delete(name);
    },
  };
}

/** Lets the pending promises of a component run. */
async function settle(): Promise<void> {
  await new Promise((resolve) => setTimeout(resolve, 0));
}

describe("releaseFocus", () => {
  it("blurs a focused element inside the container when there is no fallback", () => {
    const container = document.createElement("div");
    const inner = button("Dentro", container);
    host.append(container);
    inner.focus();
    expect(focusInside(container)).toBe(true);
    releaseFocus(container);
    expect(focusInside(container)).toBe(false);
  });

  it("moves the focus to a fallback that can take it, outside the container", () => {
    const container = document.createElement("div");
    const inner = button("Dentro", container);
    const outside = button("Fuera");
    host.append(container);
    inner.focus();
    releaseFocus(container, outside);
    expect(document.activeElement).toBe(outside);
  });

  it("blurs instead of using a fallback that is inside the container, hidden or disabled", () => {
    const container = document.createElement("div");
    const inner = button("Dentro", container);
    const other = button("Otro", container);
    const hidden = button("Oculto");
    hidden.hidden = true;
    const disabled = button("Desactivado");
    disabled.disabled = true;
    host.append(container, hidden, disabled);
    for (const fallback of [other, hidden, disabled]) {
      inner.focus();
      releaseFocus(container, fallback);
      expect(focusInside(container), fallback.textContent ?? "").toBe(false);
      expect(document.activeElement, fallback.textContent ?? "").not.toBe(fallback);
    }
  });

  it("leaves a focus outside the container alone", () => {
    const container = document.createElement("div");
    button("Dentro", container);
    host.append(container);
    const outside = button("Fuera");
    outside.focus();
    releaseFocus(container);
    expect(document.activeElement).toBe(outside);
  });
});

describe("hidden and removed parts give the focus up", () => {
  it("a menu that is destroyed while an item has the focus", () => {
    const menu = createMenu({ items: [{ id: "a", label: "A" }, { id: "b", label: "B" }] });
    host.append(menu.el);
    menu.focus(1);
    expect(focusInside(menu.el)).toBe(true);
    menu.destroy();
    expect(focusInside(menu.el)).toBe(false);
  });

  it("a button that is destroyed while it has the focus", () => {
    const pressable = createButton({ label: "Guardar" });
    host.append(pressable.el);
    pressable.el.focus();
    pressable.destroy();
    expect(focusInside(pressable.el)).toBe(false);
  });

  it("a plate that is destroyed while one of its contents has the focus", () => {
    const plate = createPlate({ title: "Inventario" });
    const inner = button("Usar", plate.body);
    host.append(plate.el);
    inner.focus();
    plate.destroy();
    expect(focusInside(plate.el)).toBe(false);
  });

  it("a dialog that closes after its opener was removed does not keep the focus", async () => {
    const opener = button("Salir");
    opener.focus();
    const modal = createModal({ title: "¿Salir?", actions: [{ id: "yes", label: "Sí" }, { id: "no", label: "No" }] });
    host.append(modal.el);
    const answer = modal.open();
    expect(focusInside(modal.el)).toBe(true);
    opener.remove();
    modal.close("no");
    expect(focusInside(modal.el)).toBe(false);
    await expect(answer).resolves.toBe("no");
  });

  it("a dialog that is destroyed while it is open gives up the focus", () => {
    const modal = createModal({ title: "Aviso", actions: [{ id: "ok", label: "OK" }] });
    host.append(modal.el);
    void modal.open();
    modal.destroy();
    expect(focusInside(modal.el)).toBe(false);
  });

  it("a container that is hidden with setShown gives up the focus inside it", () => {
    const panel = document.createElement("div");
    const inner = button("Dentro", panel);
    host.append(panel);
    inner.focus();
    expect(setShown(panel, false)).toBe(true);
    expect(panel.hidden).toBe(true);
    expect(focusInside(panel)).toBe(false);
  });

  it("a screen that another screen covers gives up the focus", () => {
    const screens = createScreenStack(host, { transitionMs: 0 });
    const first = screenWith("title");
    const second = screenWith("pause");
    screens.push(first.screen);
    expect(focusInside(first.el)).toBe(true);
    screens.push(second.screen);
    expect(first.el.hidden).toBe(true);
    expect(focusInside(first.el)).toBe(false);
    screens.destroy();
  });

  it("the screen stack gives up the focus when it is destroyed", () => {
    const screens = createScreenStack(host, { transitionMs: 0 });
    const first = screenWith("title");
    screens.push(first.screen);
    expect(focusInside(first.el)).toBe(true);
    screens.destroy();
    expect(focusInside(first.el)).toBe(false);
  });

  it("a save list that is destroyed while a row has the focus", async () => {
    const slots = createSaveSlots({ store: storeWithSave(), mode: "load" });
    host.append(slots.el);
    await settle();
    slots.focusTarget()?.focus();
    expect(focusInside(slots.el)).toBe(true);
    slots.destroy();
    expect(focusInside(slots.el)).toBe(false);
  });

  it("a notice that leaves the column with its button focused", () => {
    const notices = createNotices({ host, fadeMs: 0 });
    const id = notices.notify({ message: "¿Aceptar?", actions: [{ id: "yes", label: "Sí" }], durationMs: null });
    notices.el.querySelector<HTMLElement>('[data-action="yes"]')?.focus();
    expect(focusInside(notices.el)).toBe(true);
    notices.dismiss(id);
    expect(focusInside(notices.el)).toBe(false);
    notices.destroy();
  });

  it("a history drawer that closes after its opener was removed gives up the focus", () => {
    const notices = createNotices({ host, fadeMs: 0 });
    const opener = button("Abrir");
    opener.focus();
    notices.openHistory();
    const drawer = host.querySelector<HTMLElement>(".vu-history");
    expect(drawer).not.toBeNull();
    if (drawer === null) return;
    expect(focusInside(drawer)).toBe(true);
    opener.remove();
    notices.closeHistory();
    expect(focusInside(drawer)).toBe(false);
    notices.destroy();
  });

  it("the notices give up the focus when they are destroyed", () => {
    const notices = createNotices({ host, fadeMs: 0 });
    notices.notify({ message: "Pendiente", actions: [{ id: "ok", label: "OK" }], durationMs: null });
    notices.el.querySelector<HTMLElement>('[data-action="ok"]')?.focus();
    expect(focusInside(notices.el)).toBe(true);
    notices.destroy();
    expect(focusInside(notices.el)).toBe(false);
  });

  it("the dialogue box gives up the focus when it is stopped with a choice on screen", () => {
    const box = createDialogueBox({ speed: 0 });
    host.append(box.el);
    box.play(sessionOf([choice()]));
    expect(focusInside(box.el)).toBe(true);
    box.stop();
    expect(focusInside(box.el)).toBe(false);
    box.destroy();
  });

  it("the dialogue box gives up the focus when the conversation ends", () => {
    const box = createDialogueBox({ speed: 0 });
    host.append(box.el);
    box.play(sessionOf([line("Hola")]));
    expect(focusInside(box.el)).toBe(true);
    box.advance();
    expect(box.el.hidden).toBe(true);
    expect(focusInside(box.el)).toBe(false);
    box.destroy();
  });

  it("the dialogue box gives up the focus when it is destroyed", () => {
    const box = createDialogueBox({ speed: 0 });
    host.append(box.el);
    box.play(sessionOf([line("Hola")]));
    box.destroy();
    expect(focusInside(box.el)).toBe(false);
  });

  it("a rebinding panel that is destroyed while a row has the focus", () => {
    const port: RebindPort<"confirm"> = {
      slots: () => [{ slot: 0, display: null }],
      capture: () => new Promise<null>(() => undefined),
      clear: () => undefined,
      resetAction: () => undefined,
      resetAll: () => undefined,
    };
    const panel = createRebindPanel({ port, actions: [{ id: "confirm", labelKey: "action.confirm" }], t: (key) => key });
    host.append(panel.el);
    panel.el.querySelector<HTMLElement>('[role="menuitem"]')?.focus();
    expect(focusInside(panel.el)).toBe(true);
    panel.destroy();
    expect(focusInside(panel.el)).toBe(false);
  });

  it("an item that is taken off the board gives up the focus", () => {
    const stage = createStage(host, { width: 1920, height: 1080 });
    const board = createBoardView(stage, { worldBounds: { x: 0, y: 0, w: 3000, h: 2000 } });
    const item = document.createElement("button");
    board.addItem(item, { x: 0, y: 0, w: 100, h: 50 });
    item.focus();
    board.removeItem(item);
    expect(item.contains(document.activeElement)).toBe(false);
    board.destroy();
    stage.destroy();
  });

  it("the board gives up the focus of its items when it is destroyed", () => {
    const stage = createStage(host, { width: 1920, height: 1080 });
    const board = createBoardView(stage, { worldBounds: { x: 0, y: 0, w: 3000, h: 2000 } });
    const item = document.createElement("button");
    board.addItem(item, { x: 0, y: 0, w: 100, h: 50 });
    item.focus();
    board.destroy();
    expect(board.el.contains(document.activeElement)).toBe(false);
    stage.destroy();
  });

  it("the stage gives up the focus of its game when it is destroyed", () => {
    const stage = createStage(host, { width: 1920, height: 1080 });
    const inner = button("Juego", stage.root);
    inner.focus();
    const viewport = host.querySelector<HTMLElement>(".vu-viewport");
    expect(viewport).not.toBeNull();
    if (viewport === null) return;
    stage.destroy();
    expect(viewport.contains(document.activeElement)).toBe(false);
  });
});
