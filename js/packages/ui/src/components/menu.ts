// A menu: a vertical list of items to choose from, with a keyboard and a mouse.
//
// Up and Down arrows move the focus, skip disabled items and wrap at the ends. Home and End go to
// the first and last enabled item. Enter or Space chooses the focused item. Moving the mouse over
// an item moves the focus to it, and a click chooses it. The focus is the DOM focus, so a screen
// reader announces each item as it is reached. Only the focused item is in the Tab order.

import { mergeDefined, type VuComponent } from "./component";
import { createElement } from "./dom";

/** One item of a menu. */
export interface MenuItem {
  /** The id given to `onSelect` when the item is chosen. */
  readonly id: string;
  /** The text of the item. */
  readonly label: string;
  /** True disables the item: the focus skips it and a click does nothing. */
  readonly disabled?: boolean;
}

/** Props of a menu. */
export interface MenuProps {
  /** The items, in order. */
  readonly items?: readonly MenuItem[];
  /** The name of the menu, read by screen readers. */
  readonly label?: string;
  /** Runs with the id of an item when the item is chosen. */
  readonly onSelect?: (id: string) => void;
}

const DEFAULTS: MenuProps = { items: [], label: "" };

/**
 * Creates a menu.
 * @param props - the starting props.
 */
export function createMenu(props: MenuProps = {}): VuComponent<MenuProps> {
  const el = createElement("div", "vu-menu");
  el.setAttribute("role", "menu");
  el.setAttribute("aria-orientation", "vertical");
  let current = mergeDefined(DEFAULTS, props);
  /** The item elements, in the order of the items. */
  let rows: HTMLElement[] = [];
  /** The index of the focused item, or -1 when no item can take the focus. */
  let focusIndex = -1;

  const itemList = (): readonly MenuItem[] => current.items ?? [];

  const isEnabled = (index: number): boolean => {
    const item = itemList()[index];
    return item !== undefined && item.disabled !== true;
  };

  /** The next enabled index from `from` in `direction`, wrapping at the ends; -1 when none. */
  const step = (from: number, direction: 1 | -1): number => {
    const count = itemList().length;
    if (count === 0) return -1;
    let index = from < 0 ? (direction === 1 ? -1 : count) : from;
    for (let tries = 0; tries < count; tries += 1) {
      index = (index + direction + count) % count;
      if (isEnabled(index)) return index;
    }
    return -1;
  };

  /** The last enabled index, or -1 when none. */
  const lastEnabled = (): number => {
    for (let index = itemList().length - 1; index >= 0; index -= 1) {
      if (isEnabled(index)) return index;
    }
    return -1;
  };

  /** Only the focused item is in the Tab order. */
  const applyTabStops = (): void => {
    rows.forEach((node, index) => {
      node.tabIndex = index === focusIndex ? 0 : -1;
    });
  };

  /** Moves the focus to an item and gives it the DOM focus. */
  const focusItem = (index: number): void => {
    if (index < 0) return;
    focusIndex = index;
    applyTabStops();
    rows[index]?.focus();
  };

  /** The first enabled index, or -1 when none. */
  const firstEnabled = (): number => step(-1, 1);

  const choose = (index: number): void => {
    if (!isEnabled(index)) return;
    const item = itemList()[index];
    if (item !== undefined) current.onSelect?.(item.id);
  };

  const onKeyDown = (event: KeyboardEvent): void => {
    let handled = true;
    switch (event.key) {
      case "ArrowDown":
        focusItem(step(focusIndex, 1));
        break;
      case "ArrowUp":
        focusItem(step(focusIndex, -1));
        break;
      case "Home":
        focusItem(step(-1, 1));
        break;
      case "End":
        focusItem(lastEnabled());
        break;
      case "Enter":
      case " ":
        if (focusIndex >= 0) choose(focusIndex);
        break;
      default:
        handled = false;
    }
    if (handled) event.preventDefault();
  };
  el.addEventListener("keydown", onKeyDown);

  const render = (): void => {
    const hadFocus = el.contains(document.activeElement);
    const label = current.label ?? "";
    if (label === "") el.removeAttribute("aria-label");
    else el.setAttribute("aria-label", label);

    rows = itemList().map((item, index) => {
      const node = createElement("div", "vu-menu-item", item.label);
      node.setAttribute("role", "menuitem");
      node.setAttribute("data-id", item.id);
      if (item.disabled === true) node.setAttribute("aria-disabled", "true");
      node.addEventListener("pointerenter", () => {
        if (isEnabled(index)) focusItem(index);
      });
      node.addEventListener("click", () => {
        choose(index);
      });
      return node;
    });
    el.replaceChildren(...rows);

    // Keep the focus on the same index when it is still a valid item; otherwise take the first.
    if (!isEnabled(focusIndex)) focusIndex = firstEnabled();
    applyTabStops();
    if (hadFocus && focusIndex >= 0) rows[focusIndex]?.focus();
  };

  render();

  return {
    el,
    update(next: Partial<MenuProps>): void {
      current = mergeDefined(current, next);
      render();
    },
    destroy(): void {
      el.removeEventListener("keydown", onKeyDown);
      el.remove();
    },
  };
}
