// @vitest-environment happy-dom
// Tests for the menu: keyboard navigation with the arrows, Home and End, wrapping and skipping
// disabled items, choosing with Enter, Space or a click, and the focus that follows the mouse.

import { afterEach, describe, expect, it, vi } from "vitest";
import { createMenu, type Menu, type MenuItem } from "./menu";

const ITEMS: readonly MenuItem[] = [
  { id: "new", label: "Nueva partida" },
  { id: "load", label: "Cargar", disabled: true },
  { id: "quit", label: "Salir" },
];

afterEach(() => {
  document.body.innerHTML = "";
});

/** A menu in the document, with a spy for its selections. */
function setup(): {
  el: HTMLElement;
  menu: Menu;
  onSelect: ReturnType<typeof vi.fn>;
  update: (items: readonly MenuItem[]) => void;
} {
  const onSelect = vi.fn();
  const menu = createMenu({ items: ITEMS, label: "Menú principal", onSelect });
  document.body.append(menu.el);
  return { el: menu.el, menu, onSelect, update: (items) => menu.update({ items }) };
}

/** Presses a key on the menu, the way a keyboard does while an item has the focus. */
function press(target: HTMLElement, key: string): KeyboardEvent {
  const event = new KeyboardEvent("keydown", { key, bubbles: true, cancelable: true });
  target.dispatchEvent(event);
  return event;
}

/** The id of the item that has the DOM focus, or null. */
function focusedId(): string | null {
  return document.activeElement?.getAttribute("data-id") ?? null;
}

/** The item element with the given id. */
function item(root: HTMLElement, id: string): HTMLElement {
  const node = root.querySelector<HTMLElement>(`[data-id="${id}"]`);
  if (node === null) throw new Error(`no menu item ${id}`);
  return node;
}

describe("menu structure", () => {
  it("is a vertical menu of menuitems, named by its label", () => {
    const { el } = setup();
    expect(el.getAttribute("role")).toBe("menu");
    expect(el.getAttribute("aria-orientation")).toBe("vertical");
    expect(el.getAttribute("aria-label")).toBe("Menú principal");
    expect(el.querySelectorAll('[role="menuitem"]')).toHaveLength(3);
  });

  it("marks disabled items with aria-disabled", () => {
    const { el } = setup();
    expect(item(el, "load").getAttribute("aria-disabled")).toBe("true");
    expect(item(el, "new").hasAttribute("aria-disabled")).toBe(false);
  });

  it("puts only the first enabled item in the tab order", () => {
    const { el } = setup();
    expect(item(el, "new").tabIndex).toBe(0);
    expect(item(el, "load").tabIndex).toBe(-1);
    expect(item(el, "quit").tabIndex).toBe(-1);
  });
});

describe("menu keyboard", () => {
  it("moves the focus down with ArrowDown, skipping disabled items", () => {
    const { el } = setup();
    press(el, "ArrowDown");
    expect(focusedId()).toBe("quit");
  });

  it("wraps at the ends with the arrows", () => {
    const { el } = setup();
    press(el, "ArrowDown");
    press(el, "ArrowDown");
    expect(focusedId()).toBe("new");
    press(el, "ArrowUp");
    expect(focusedId()).toBe("quit");
  });

  it("jumps to the first and last enabled items with Home and End", () => {
    const { el } = setup();
    press(el, "End");
    expect(focusedId()).toBe("quit");
    press(el, "Home");
    expect(focusedId()).toBe("new");
  });

  it("moves the tab stop with the focus", () => {
    const { el } = setup();
    press(el, "ArrowDown");
    expect(item(el, "quit").tabIndex).toBe(0);
    expect(item(el, "new").tabIndex).toBe(-1);
  });

  it("chooses the focused item with Enter or Space", () => {
    const { el, onSelect } = setup();
    press(el, "ArrowDown");
    press(el, "Enter");
    expect(onSelect).toHaveBeenLastCalledWith("quit");
    press(el, " ");
    expect(onSelect).toHaveBeenCalledTimes(2);
  });

  it("stops the keys it uses from doing anything else, and ignores the rest", () => {
    const { el } = setup();
    expect(press(el, "ArrowDown").defaultPrevented).toBe(true);
    expect(press(el, "q").defaultPrevented).toBe(false);
  });

  it("moves the focus to the first enabled item when the focused item disappears", () => {
    const { el, update } = setup();
    press(el, "End");
    update([
      { id: "a", label: "A" },
      { id: "b", label: "B" },
    ]);
    expect(el.querySelectorAll('[role="menuitem"]')).toHaveLength(2);
    expect(item(el, "a").tabIndex).toBe(0);
  });
});

describe("menu focus by index", () => {
  it("gives the focus to the item at an index, and reports that index", () => {
    const { el, menu } = setup();
    menu.focus(2);
    expect(focusedId()).toBe("quit");
    expect(menu.focusedIndex).toBe(2);
    expect(item(el, "quit").tabIndex).toBe(0);
    expect(item(el, "new").tabIndex).toBe(-1);
  });

  it("starts with the first enabled item as the focused index, without taking the DOM focus", () => {
    const { menu } = setup();
    expect(menu.focusedIndex).toBe(0);
    expect(focusedId()).toBeNull();
  });

  it("takes the next enabled item when the index is disabled", () => {
    const { menu } = setup();
    menu.focus(1); // "load" is disabled
    expect(focusedId()).toBe("quit");
    expect(menu.focusedIndex).toBe(2);
  });

  it("takes the enabled item before the index when none after it is enabled", () => {
    const menu = createMenu({
      items: [
        { id: "a", label: "A" },
        { id: "b", label: "B" },
        { id: "c", label: "C", disabled: true },
      ],
    });
    document.body.append(menu.el);
    menu.focus(2);
    expect(focusedId()).toBe("b");
    expect(menu.focusedIndex).toBe(1);
  });

  it("clamps an index past either end to the first or the last item", () => {
    const { menu } = setup();
    menu.focus(-4);
    expect(focusedId()).toBe("new");
    expect(menu.focusedIndex).toBe(0);
    menu.focus(99);
    expect(focusedId()).toBe("quit");
    expect(menu.focusedIndex).toBe(2);
  });

  it("ignores an index that is not a number", () => {
    const { menu } = setup();
    menu.focus(2);
    menu.focus(Number.NaN);
    expect(focusedId()).toBe("quit");
    expect(menu.focusedIndex).toBe(2);
  });

  it("has no focused index when no item can take the focus", () => {
    const menu = createMenu({ items: [{ id: "a", label: "A", disabled: true }] });
    document.body.append(menu.el);
    expect(menu.focusedIndex).toBe(-1);
    menu.focus(0);
    expect(menu.focusedIndex).toBe(-1);
    expect(focusedId()).toBeNull();
  });

  it("makes the arrows and Enter act on the item that was focused by index", () => {
    const { el, menu, onSelect } = setup();
    menu.focus(2);
    press(el, "ArrowDown");
    expect(focusedId()).toBe("new");
    menu.focus(2);
    press(el, "Enter");
    expect(onSelect).toHaveBeenLastCalledWith("quit");
  });
});

describe("menu follows the DOM focus", () => {
  it("takes the focused index from an item that the page focuses itself", () => {
    const { el, menu } = setup();
    item(el, "quit").focus();
    expect(menu.focusedIndex).toBe(2);
    expect(item(el, "quit").tabIndex).toBe(0);
    expect(item(el, "new").tabIndex).toBe(-1);
  });

  it("moves from that item with the arrows, not from the item it last had", () => {
    const { el } = setup();
    item(el, "quit").focus();
    press(el, "ArrowDown"); // from "quit" the next enabled item wraps to "new"
    expect(focusedId()).toBe("new");
  });

  it("chooses the item with the focus on Enter after an outside focus", () => {
    const { el, onSelect } = setup();
    item(el, "quit").focus();
    press(el, "Enter");
    expect(onSelect).toHaveBeenCalledTimes(1);
    expect(onSelect).toHaveBeenLastCalledWith("quit");
  });
});

describe("menu mouse", () => {
  it("moves the focus to an item the mouse enters", () => {
    const { el } = setup();
    item(el, "quit").dispatchEvent(new PointerEvent("pointerenter", { bubbles: true }));
    expect(focusedId()).toBe("quit");
  });

  it("does not move the focus onto a disabled item", () => {
    const { el } = setup();
    item(el, "quit").dispatchEvent(new PointerEvent("pointerenter", { bubbles: true }));
    item(el, "load").dispatchEvent(new PointerEvent("pointerenter", { bubbles: true }));
    expect(focusedId()).toBe("quit");
  });

  it("chooses an enabled item on a click, and not a disabled one", () => {
    const { el, onSelect } = setup();
    item(el, "load").click();
    expect(onSelect).not.toHaveBeenCalled();
    item(el, "quit").click();
    expect(onSelect).toHaveBeenCalledWith("quit");
  });
});
