// Keyboard helpers shared by the menus and the game's keys.
//
// Enter and Space do two jobs: a focused button or menu item takes them, and the game's own keys use
// them (Space pauses, Enter and Space go on through a dialogue). The two must not both act on one
// press, so a menu keeps its Enter and Space to itself, and the game's key reader skips them while a
// button or a menu item has the focus.

/**
 * Keeps Enter and Space inside a menu. The menu's own handler runs first (it was added first), and
 * then this stops the key from reaching the window, where the game's keys would act on it too. Call
 * it after the menu is created, and again for each new menu.
 *
 * @param menu - the menu's element.
 */
export function keepKeysInMenu(menu: HTMLElement): void {
  menu.addEventListener("keydown", (event) => {
    if (event.key === "Enter" || event.key === " ") event.stopPropagation();
  });
}

/** True when the focus is on a button or a menu item, which takes Space and Enter for itself. */
export function focusIsControl(): boolean {
  const active = document.activeElement;
  if (!(active instanceof HTMLElement)) return false;
  return active instanceof HTMLButtonElement || active.getAttribute("role") === "menuitem";
}
