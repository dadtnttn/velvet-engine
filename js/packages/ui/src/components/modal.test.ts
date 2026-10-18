// @vitest-environment happy-dom
// Tests for the modal: the promise of the chosen action, Escape, the focus trap (Tab, Shift+Tab
// and focus that lands outside), the focus returning to the element that opened it, and update.

import { afterEach, describe, expect, it } from "vitest";
import { createModal, type ModalProps } from "./modal";

afterEach(() => {
  document.body.innerHTML = "";
});

/** A trigger button and a modal with three actions, both in the document. */
function setup(props: ModalProps = {}): { trigger: HTMLButtonElement; modal: ReturnType<typeof createModal> } {
  const trigger = document.createElement("button");
  trigger.textContent = "Abrir";
  document.body.append(trigger);
  const modal = createModal({
    title: "¿Seguro?",
    body: "Se perderá el progreso.",
    actions: [
      { id: "yes", label: "Sí" },
      { id: "no", label: "No" },
      { id: "later", label: "Luego" },
    ],
    ...props,
  });
  document.body.append(modal.el);
  return { trigger, modal };
}

/** Presses a key on the focused element, the way the keyboard does. */
function press(key: string, shiftKey = false): KeyboardEvent {
  const target = document.activeElement ?? document.body;
  const event = new KeyboardEvent("keydown", { key, shiftKey, bubbles: true, cancelable: true });
  target.dispatchEvent(event);
  return event;
}

/** The text of the focused element. */
function focusedText(): string | null {
  return document.activeElement?.textContent ?? null;
}

/** The action button with the given id. */
function action(modal: ReturnType<typeof createModal>, id: string): HTMLButtonElement {
  const button = modal.el.querySelector<HTMLButtonElement>(`[data-action="${id}"]`);
  if (button === null) throw new Error(`no action ${id}`);
  return button;
}

describe("modal opening and choosing", () => {
  it("is a dialog that opens with its first action focused", () => {
    const { trigger, modal } = setup();
    trigger.focus();
    expect(modal.el.hidden).toBe(true);
    void modal.open();
    expect(modal.isOpen).toBe(true);
    expect(modal.el.hidden).toBe(false);
    expect(focusedText()).toBe("Sí");
    const dialog = modal.el.querySelector('[role="dialog"]');
    expect(dialog?.getAttribute("aria-modal")).toBe("true");
    expect(dialog?.getAttribute("aria-labelledby")).toBeTruthy();
    modal.destroy();
  });

  it("resolves with the chosen action, closes, and gives the focus back", async () => {
    const { trigger, modal } = setup();
    trigger.focus();
    const result = modal.open();
    action(modal, "no").click();
    await expect(result).resolves.toBe("no");
    expect(modal.isOpen).toBe(false);
    expect(modal.el.hidden).toBe(true);
    expect(document.activeElement).toBe(trigger);
  });

  it("returns the same promise when it is opened again while open", () => {
    const { modal } = setup();
    const first = modal.open();
    expect(modal.open()).toBe(first);
    modal.destroy();
  });

  it("resolves with null when Escape closes it", async () => {
    const { modal } = setup();
    const result = modal.open();
    const event = press("Escape");
    expect(event.defaultPrevented).toBe(true);
    await expect(result).resolves.toBeNull();
    expect(modal.isOpen).toBe(false);
  });

  it("stays open on Escape when closeOnEscape is false, until an action or close() ends it", async () => {
    const { modal } = setup({ closeOnEscape: false });
    const result = modal.open();
    press("Escape");
    expect(modal.isOpen).toBe(true);
    modal.close("later");
    await expect(result).resolves.toBe("later");
  });

  it("resolves a pending promise with null when it is destroyed", async () => {
    const { modal } = setup();
    const result = modal.open();
    modal.destroy();
    await expect(result).resolves.toBeNull();
  });

  it("does nothing when close() is called while it is shut", () => {
    const { modal } = setup();
    expect(() => modal.close("yes")).not.toThrow();
    expect(modal.isOpen).toBe(false);
  });
});

describe("modal focus trap", () => {
  it("wraps Tab from the last action to the first", () => {
    const { modal } = setup();
    void modal.open();
    action(modal, "later").focus();
    const event = press("Tab");
    expect(event.defaultPrevented).toBe(true);
    expect(focusedText()).toBe("Sí");
    modal.destroy();
  });

  it("wraps Shift+Tab from the first action to the last", () => {
    const { modal } = setup();
    void modal.open();
    const event = press("Tab", true);
    expect(event.defaultPrevented).toBe(true);
    expect(focusedText()).toBe("Luego");
    modal.destroy();
  });

  it("leaves Tab alone between the middle actions", () => {
    const { modal } = setup();
    void modal.open();
    action(modal, "no").focus();
    expect(press("Tab").defaultPrevented).toBe(false);
    modal.destroy();
  });

  it("brings the focus back inside when something outside takes it", () => {
    const { modal } = setup();
    void modal.open();
    const outside = document.createElement("button");
    outside.textContent = "Fuera";
    document.body.append(outside);
    outside.focus();
    outside.dispatchEvent(new FocusEvent("focusin", { bubbles: true }));
    expect(focusedText()).toBe("Sí");
    modal.destroy();
  });

  it("stops trapping the focus once it is closed", async () => {
    const { trigger, modal } = setup();
    trigger.focus();
    const result = modal.open();
    modal.close(null);
    await result;
    const outside = document.createElement("button");
    outside.textContent = "Fuera";
    document.body.append(outside);
    outside.focus();
    expect(focusedText()).toBe("Fuera");
  });
});

describe("modal content", () => {
  it("shows the title and the body as text", () => {
    const { modal } = setup({ title: "Atención", body: "Texto" });
    expect(modal.el.querySelector(".vu-modal-title")?.textContent).toBe("Atención");
    expect(modal.el.querySelector(".vu-modal-body")?.textContent).toBe("Texto");
  });

  it("re-renders its actions when update changes them", () => {
    const { modal } = setup();
    modal.update({ title: "Otra", actions: [{ id: "ok", label: "Vale" }] });
    expect(modal.el.querySelector(".vu-modal-title")?.textContent).toBe("Otra");
    expect(modal.el.querySelectorAll("[data-action]")).toHaveLength(1);
    expect(action(modal, "ok").textContent).toBe("Vale");
  });

  it("disables an action that is marked disabled", () => {
    const { modal } = setup({ actions: [{ id: "x", label: "No disponible", disabled: true }] });
    expect(action(modal, "x").disabled).toBe(true);
  });
});
