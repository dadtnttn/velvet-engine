// A modal dialog: a plate over a dimmed backdrop, for a choice that must be made now.
//
// While it is open, the keyboard focus stays inside the dialog: Tab and Shift+Tab wrap around, and
// a click that moves the focus outside brings it back. Escape closes the dialog, unless
// `closeOnEscape` is false. When the dialog closes, the focus goes back to the element that had it.
// `open()` returns a promise of the id of the action that was chosen.

import { mergeDefined, type VuComponent } from "./component";
import { createElement, focusableWithin, uniqueId } from "./dom";

/** One button of a modal. */
export interface ModalAction {
  /** Returned by the promise of `open()` when this action is chosen. */
  readonly id: string;
  /** The text on the button. */
  readonly label: string;
  /** True disables the button. */
  readonly disabled?: boolean;
}

/** Props of a modal. */
export interface ModalProps {
  /** The title, read by screen readers as the name of the dialog. */
  readonly title?: string;
  /** The text under the title. */
  readonly body?: string;
  /** The buttons, in order. The first enabled one gets the focus when the dialog opens. */
  readonly actions?: readonly ModalAction[];
  /** Whether Escape closes the dialog. Default true. When false, only an action closes it. */
  readonly closeOnEscape?: boolean;
}

/** A modal dialog. It starts closed. */
export interface Modal extends VuComponent<ModalProps> {
  /**
   * Opens the dialog. The promise resolves with the id of the chosen action, or with null when
   * the dialog closes without one (Escape, or `close()` with no result). Calling it while the
   * dialog is open returns the same promise.
   */
  open(): Promise<string | null>;
  /** Closes the dialog and resolves its promise with `result`. Does nothing when it is closed. */
  close(result?: string | null): void;
  /** True while the dialog is open. */
  readonly isOpen: boolean;
}

const DEFAULTS: ModalProps = { title: "", body: "", actions: [], closeOnEscape: true };

/**
 * Creates a modal dialog. Append its `el` to the stage's root. It starts hidden.
 * @param props - the starting props.
 */
export function createModal(props: ModalProps = {}): Modal {
  // The backdrop is the component's element. The dialog is a plate inside it.
  const el = createElement("div", "vu-modal");
  el.hidden = true;
  const dialog = createElement("div", "vu-plate vu-modal-dialog");
  dialog.setAttribute("role", "dialog");
  dialog.setAttribute("aria-modal", "true");
  dialog.tabIndex = -1;
  const heading = createElement("h2", "vu-modal-title");
  heading.id = uniqueId("modal-title");
  const text = createElement("p", "vu-modal-body");
  text.id = uniqueId("modal-body");
  const row = createElement("div", "vu-modal-actions");
  dialog.append(heading, text, row);
  dialog.setAttribute("aria-labelledby", heading.id);
  dialog.setAttribute("aria-describedby", text.id);
  el.append(dialog);

  let current = mergeDefined(DEFAULTS, props);
  let pending: Promise<string | null> | null = null;
  let resolveOpen: ((result: string | null) => void) | null = null;
  let returnFocusTo: HTMLElement | null = null;

  const focusFirst = (): void => {
    (focusableWithin(dialog)[0] ?? dialog).focus();
  };

  const render = (): void => {
    const hadFocus = el.contains(document.activeElement);
    heading.textContent = current.title ?? "";
    text.textContent = current.body ?? "";
    text.hidden = (current.body ?? "") === "";
    const buttons = (current.actions ?? []).map((action) => {
      const button = createElement("button", "vu-button", action.label);
      button.type = "button";
      button.disabled = action.disabled === true;
      button.setAttribute("data-action", action.id);
      button.addEventListener("click", () => {
        close(action.id);
      });
      return button;
    });
    row.replaceChildren(...buttons);
    if (hadFocus && pending !== null) focusFirst();
  };

  /** Keeps Tab and Shift+Tab inside the dialog, and closes it on Escape. */
  const onKeyDown = (event: KeyboardEvent): void => {
    if (pending === null) return;
    if (event.key === "Escape") {
      if (current.closeOnEscape !== false) {
        event.preventDefault();
        close(null);
      }
      return;
    }
    if (event.key !== "Tab") return;
    const items = focusableWithin(dialog);
    const first = items[0];
    const last = items[items.length - 1];
    if (first === undefined || last === undefined) {
      event.preventDefault();
      dialog.focus();
      return;
    }
    const active = document.activeElement;
    const inside = active !== null && dialog.contains(active);
    if (event.shiftKey) {
      if (!inside || active === first) {
        event.preventDefault();
        last.focus();
      }
    } else if (!inside || active === last) {
      event.preventDefault();
      first.focus();
    }
  };

  /** Pulls the focus back in when something outside the dialog takes it. */
  const onFocusIn = (event: FocusEvent): void => {
    if (pending === null) return;
    if (event.target instanceof Node && dialog.contains(event.target)) return;
    focusFirst();
  };

  const open = (): Promise<string | null> => {
    if (pending !== null) return pending;
    returnFocusTo = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    const promise = new Promise<string | null>((resolve) => {
      resolveOpen = resolve;
    });
    pending = promise;
    el.hidden = false;
    document.addEventListener("keydown", onKeyDown);
    document.addEventListener("focusin", onFocusIn);
    focusFirst();
    return promise;
  };

  const close = (result: string | null = null): void => {
    if (pending === null) return;
    const resolve = resolveOpen;
    pending = null;
    resolveOpen = null;
    el.hidden = true;
    document.removeEventListener("keydown", onKeyDown);
    document.removeEventListener("focusin", onFocusIn);
    const back = returnFocusTo;
    returnFocusTo = null;
    if (back !== null && back.isConnected) back.focus();
    resolve?.(result);
  };

  render();

  return {
    el,
    get isOpen(): boolean {
      return pending !== null;
    },
    open,
    close,
    update(next: Partial<ModalProps>): void {
      current = mergeDefined(current, next);
      render();
    },
    destroy(): void {
      close(null);
      el.remove();
    },
  };
}
