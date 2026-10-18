// The error overlay: a panel over the window that says the game broke. It shows the report text in a box that
// scrolls, and offers Copy, Continue and Reload. It opens at once, whatever state the game is in, so it needs
// nothing from the game but the report: its own element, its own focus rules and its own labels. It uses the
// modal's classes, so it looks like the rest of the HUD. This package does not import @velvet/core: the report is
// given as text, or as an object that holds its text. formatReport in @velvet/core makes that text.
//
// While the overlay is open, Tab and Shift+Tab stay inside it, and the left and right arrows move between its
// buttons, so a pad's d-pad works. Escape and Continue close it and then call onContinue. Reload calls onReload.
// Copy copies the text. When the overlay closes, the focus goes back to the element that had it, or is released.

import { createElement, focusableWithin, releaseFocus, uniqueId } from "./components/dom";

/** Translates a label by its key. Values such as {id} come in `args`. The same shape as the rebinding panel's translator. */
export type ErrorOverlayTranslate = (key: string, args?: Readonly<Record<string, string | number>>) => string;

/** A report as an object: its text, and optionally its id and headline. */
export interface ErrorOverlayReportObject {
  /** The report's id. It is shown under the headline, so that a player can name the report. */
  readonly id?: string;
  /** The error's message. It is shown as the headline. */
  readonly message?: string;
  /** The report text. The overlay shows it and copies it as it is. */
  readonly text: string;
}

/** The report to show: its text, or an object that holds its text. */
export type ErrorOverlayReport = string | ErrorOverlayReportObject;

/** Options for {@link createErrorOverlay}. */
export interface ErrorOverlayOptions {
  /** The element that the overlay is added to. The overlay covers the whole window, so any element will do. */
  readonly host: HTMLElement;
  /** The report to show. */
  readonly report: ErrorOverlayReport;
  /** Translates the labels. Default: the English labels. */
  readonly t?: ErrorOverlayTranslate;
  /**
   * Called after Copy, with whether the text reached the clipboard. When it did not, the text is selected, so the
   * player can press Ctrl+C.
   */
  readonly onCopy?: (copied: boolean) => void;
  /** Called after Continue, or Escape, has closed the overlay. */
  readonly onContinue?: () => void;
  /** Called by Reload. Default: reloads the page. */
  readonly onReload?: () => void;
}

/** An error overlay. It opens when it is made. Make a new one to show another report. */
export interface ErrorOverlay {
  /** The overlay's element, which is added to the host. */
  readonly el: HTMLElement;
  /** True until the overlay is closed or destroyed. */
  readonly isOpen: boolean;
  /** Hides the overlay and gives the focus back. This does not call onContinue. Calling it again does nothing. */
  close(): void;
  /** Closes the overlay if it is open, and removes its element from the host. Safe to call twice. */
  destroy(): void;
}

/** The English labels, by key. A label may have values in braces, such as {id}. */
const ENGLISH = {
  "error.title": "Something went wrong",
  "error.body": "The game hit an error. You can copy the report and send it to the developers, carry on, or reload.",
  "error.reportText": "Report text",
  "error.reportId": "Report {id}",
  "error.copy": "Copy report",
  "error.copied": "The report is on the clipboard.",
  "error.copyFailed": "The clipboard is not available. The report is selected: press Ctrl+C to copy it.",
  "error.continue": "Continue",
  "error.reload": "Reload",
} as const;

/** One of the label keys. */
type LabelKey = keyof typeof ENGLISH;

/** The fields of a report that the overlay uses, with the empty ones left out as null. */
interface ReportView {
  readonly text: string;
  readonly id: string | null;
  readonly message: string | null;
}

/** The text, or null when it is missing or empty. */
function nonEmpty(text: string | undefined): string | null {
  return text === undefined || text === "" ? null : text;
}

/** The report as the overlay reads it. */
function viewOf(report: ErrorOverlayReport): ReportView {
  if (typeof report === "string") return { text: report, id: null, message: null };
  return { text: report.text, id: nonEmpty(report.id), message: nonEmpty(report.message) };
}

/** Replaces each {name} in a template with its value. A name with no value is left as it is. */
function fill(template: string, args: Readonly<Record<string, string | number>> | undefined): string {
  return template.replace(/\{(\w+)\}/g, (whole: string, name: string) => {
    const value = args?.[name];
    return value === undefined ? whole : String(value);
  });
}

/** A label: from the translator when one is given, otherwise the English text with its values filled in. */
function labelOf(
  translate: ErrorOverlayTranslate | undefined,
  key: LabelKey,
  args?: Readonly<Record<string, string | number>>,
): string {
  if (translate !== undefined) return translate(key, args);
  return fill(ENGLISH[key], args);
}

/** A button of the overlay, with the action it stands for in its data-action attribute. */
function actionButton(action: string, text: string): HTMLButtonElement {
  const button = createElement("button", "vu-button", text);
  button.type = "button";
  button.setAttribute("data-action", action);
  return button;
}

/**
 * Shows the error overlay over the host. It opens at once, with the focus on Continue.
 * @param options - the report, the host, the labels and the callbacks.
 * @returns the overlay, which can be closed or destroyed.
 */
export function createErrorOverlay(options: ErrorOverlayOptions): ErrorOverlay {
  const doc = options.host.ownerDocument;
  const report = viewOf(options.report);
  const label = (key: LabelKey, args?: Readonly<Record<string, string | number>>): string =>
    labelOf(options.t, key, args);

  // The root carries the colour and type tokens, so the overlay looks the same outside a stage.
  const el = createElement("div", "vu-root vu-error-root");
  const backdrop = createElement("div", "vu-modal");
  const dialog = createElement("div", "vu-plate vu-modal-dialog vu-error-dialog");
  dialog.setAttribute("role", "alertdialog");
  dialog.setAttribute("aria-modal", "true");
  dialog.tabIndex = -1;

  const title = createElement("h2", "vu-modal-title", label("error.title"));
  title.id = uniqueId("error-title");
  const body = createElement("p", "vu-modal-body", label("error.body"));
  body.id = uniqueId("error-body");
  dialog.setAttribute("aria-labelledby", title.id);
  dialog.setAttribute("aria-describedby", body.id);

  const parts: HTMLElement[] = [title, body];
  if (report.message !== null) parts.push(createElement("p", "vu-error-message", report.message));
  if (report.id !== null) parts.push(createElement("p", "vu-error-id", label("error.reportId", { id: report.id })));

  // The report box is focusable, so that a keyboard or a pad can scroll the text with the arrow keys.
  const box = createElement("div", "vu-error-report", report.text);
  box.tabIndex = 0;
  box.setAttribute("role", "region");
  box.setAttribute("aria-label", label("error.reportText"));
  const status = createElement("p", "vu-error-status");
  status.setAttribute("role", "status");
  status.setAttribute("aria-live", "polite");
  parts.push(box, status);

  const copyButton = actionButton("copy", label("error.copy"));
  const continueButton = actionButton("continue", label("error.continue"));
  const reloadButton = actionButton("reload", label("error.reload"));
  const actions = createElement("div", "vu-modal-actions");
  actions.append(copyButton, continueButton, reloadButton);
  parts.push(actions);

  dialog.append(...parts);
  backdrop.append(dialog);
  el.append(backdrop);
  options.host.append(el);

  const buttons = [copyButton, continueButton, reloadButton];
  let open = true;
  let returnFocusTo: HTMLElement | null = doc.activeElement instanceof HTMLElement ? doc.activeElement : null;

  /** Hides the overlay, removes its listeners, and gives the focus back to the element that had it. */
  const close = (): void => {
    if (!open) return;
    open = false;
    el.hidden = true;
    doc.removeEventListener("keydown", onKeyDown);
    doc.removeEventListener("focusin", onFocusIn);
    const back = returnFocusTo;
    returnFocusTo = null;
    if (back !== null && back.isConnected && !el.contains(back)) back.focus({ preventScroll: true });
    releaseFocus(el);
  };

  /** Closes the overlay the way Continue does: it closes, and then the game hears about it. */
  const dismiss = (): void => {
    if (!open) return;
    close();
    options.onContinue?.();
  };

  /** Reports the outcome of a copy, in the status line, and tells the game. */
  const finishCopy = (copied: boolean): void => {
    status.textContent = label(copied ? "error.copied" : "error.copyFailed");
    options.onCopy?.(copied);
  };

  /** Selects the report text, so that Ctrl+C copies it, and tries the copy command. Used when the clipboard is not there. */
  const copyBySelection = (): void => {
    selectText(box);
    let copied = false;
    try {
      copied = doc.execCommand("copy");
    } catch {
      copied = false;
    }
    finishCopy(copied);
  };

  /** Copies the report text through the clipboard, or by selection when the clipboard refuses or is missing. */
  const copyReport = (): void => {
    const clipboard = typeof navigator === "undefined" ? undefined : navigator.clipboard;
    if (clipboard === undefined || typeof clipboard.writeText !== "function") {
      copyBySelection();
      return;
    }
    clipboard.writeText(report.text).then(
      () => {
        finishCopy(true);
      },
      () => {
        copyBySelection();
      },
    );
  };

  /** Selects everything inside an element. A page without selection support simply shows the text. */
  const selectText = (node: HTMLElement): void => {
    try {
      const selection = doc.defaultView?.getSelection();
      if (selection === null || selection === undefined) return;
      const range = doc.createRange();
      range.selectNodeContents(node);
      selection.removeAllRanges();
      selection.addRange(range);
    } catch {
      // The text stays on screen, so the player can still select it by hand.
    }
  };

  /** Keeps Tab and Shift+Tab inside the overlay, moves between the buttons with the arrows, and closes on Escape. */
  const onKeyDown = (event: KeyboardEvent): void => {
    if (!open) return;
    if (event.key === "Escape") {
      event.preventDefault();
      dismiss();
    } else if (event.key === "Tab") {
      trapTab(event);
    } else if (event.key === "ArrowLeft" || event.key === "ArrowRight") {
      moveAmongButtons(event);
    }
  };

  /** Wraps Tab and Shift+Tab around the focusable parts of the dialog, so the focus never leaves it. */
  const trapTab = (event: KeyboardEvent): void => {
    const items = focusableWithin(dialog);
    const first = items[0];
    const last = items[items.length - 1];
    if (first === undefined || last === undefined) {
      event.preventDefault();
      dialog.focus();
      return;
    }
    const active = doc.activeElement;
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

  /** The arrows move the focus along the buttons, and wrap. Elsewhere, such as in the report box, they are left alone. */
  const moveAmongButtons = (event: KeyboardEvent): void => {
    const active = doc.activeElement;
    const index = buttons.findIndex((button) => button === active);
    if (index < 0) return;
    event.preventDefault();
    const step = event.key === "ArrowRight" ? 1 : -1;
    buttons[(index + step + buttons.length) % buttons.length]?.focus();
  };

  /** Brings the focus back to Continue when something outside the overlay takes it while it is open. */
  const onFocusIn = (event: FocusEvent): void => {
    if (!open) return;
    if (event.target instanceof Node && el.contains(event.target)) return;
    continueButton.focus();
  };

  const destroy = (): void => {
    close();
    el.remove();
  };

  copyButton.addEventListener("click", () => {
    copyReport();
  });
  continueButton.addEventListener("click", () => {
    dismiss();
  });
  reloadButton.addEventListener("click", () => {
    if (options.onReload !== undefined) options.onReload();
    else doc.defaultView?.location.reload();
  });
  doc.addEventListener("keydown", onKeyDown);
  doc.addEventListener("focusin", onFocusIn);
  continueButton.focus();

  return {
    el,
    get isOpen(): boolean {
      return open;
    },
    close,
    destroy,
  };
}
