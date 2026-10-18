// @vitest-environment happy-dom
// Tests for the error overlay: what it shows and how its labels are chosen, Copy with and without a clipboard,
// Continue, Escape and Reload, the focus that stays inside while it is open and goes back when it closes, the arrow
// keys between the buttons, destroy, and the stylesheet rules of its classes.

import { readFileSync } from "node:fs";
import { afterEach, describe, expect, it, vi } from "vitest";
import {
  createErrorOverlay,
  type ErrorOverlay,
  type ErrorOverlayOptions,
  type ErrorOverlayReport,
} from "./error-overlay";

const css = readFileSync(`${__dirname}/velvet-ui.css`, "utf8");

afterEach(() => {
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
  Reflect.deleteProperty(document, "execCommand");
  document.body.innerHTML = "";
});

/** A host in the document, and an overlay made on it with the report and the options. */
function openOverlay(
  report: ErrorOverlayReport = "Report text",
  extra: Omit<ErrorOverlayOptions, "host" | "report"> = {},
): { host: HTMLElement; overlay: ErrorOverlay } {
  const host = document.createElement("div");
  document.body.append(host);
  const overlay = createErrorOverlay({ host, report, ...extra });
  return { host, overlay };
}

/** The button of an action in the overlay. */
function action(overlay: ErrorOverlay, id: string): HTMLButtonElement {
  const button = overlay.el.querySelector<HTMLButtonElement>(`[data-action="${id}"]`);
  if (button === null) throw new Error(`no action ${id}`);
  return button;
}

/** The report box, which holds the report text. */
function reportBox(overlay: ErrorOverlay): HTMLElement {
  const box = overlay.el.querySelector<HTMLElement>(".vu-error-report");
  if (box === null) throw new Error("no report box");
  return box;
}

/** The status line, which says what a copy did. */
function statusText(overlay: ErrorOverlay): string | null | undefined {
  return overlay.el.querySelector(".vu-error-status")?.textContent;
}

/** A key press on the focused element, the way the keyboard sends it. */
function press(key: string, shiftKey = false): KeyboardEvent {
  const target = document.activeElement ?? document.body;
  const event = new KeyboardEvent("keydown", { key, shiftKey, bubbles: true, cancelable: true });
  target.dispatchEvent(event);
  return event;
}

/** Lets the callbacks of resolved promises run. */
function flush(): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, 0));
}

describe("what the overlay shows", () => {
  it("opens at once in its host, with the title, the report text and the focus on Continue", () => {
    const { host, overlay } = openOverlay("Stack line");
    expect(overlay.isOpen).toBe(true);
    expect(overlay.el.hidden).toBe(false);
    expect(overlay.el.parentElement).toBe(host);
    expect(overlay.el.querySelector(".vu-modal-title")?.textContent).toBe("Something went wrong");
    expect(reportBox(overlay).textContent).toBe("Stack line");
    expect(document.activeElement).toBe(action(overlay, "continue"));
    overlay.destroy();
  });

  it("is an alert dialog named after its title and described by its body", () => {
    const { overlay } = openOverlay();
    const dialog = overlay.el.querySelector('[role="alertdialog"]');
    expect(dialog?.getAttribute("aria-modal")).toBe("true");
    const titleId = dialog?.getAttribute("aria-labelledby") ?? "";
    expect(document.getElementById(titleId)?.textContent).toBe("Something went wrong");
    const bodyId = dialog?.getAttribute("aria-describedby") ?? "";
    expect(document.getElementById(bodyId)?.tagName).toBe("P");
    overlay.destroy();
  });

  it("shows the report as text, never as markup", () => {
    const { overlay } = openOverlay('<img src="x" onerror="alert(1)">');
    expect(reportBox(overlay).querySelector("img")).toBeNull();
    expect(reportBox(overlay).textContent).toBe('<img src="x" onerror="alert(1)">');
    overlay.destroy();
  });

  it("shows the headline and the id of a report object", () => {
    const { overlay } = openOverlay({ text: "Full text", id: "r-7", message: "Boom" });
    expect(overlay.el.querySelector(".vu-error-message")?.textContent).toBe("Boom");
    expect(overlay.el.querySelector(".vu-error-id")?.textContent).toBe("Report r-7");
    expect(reportBox(overlay).textContent).toBe("Full text");
    overlay.destroy();
  });

  it("leaves out the headline and the id when the object has none", () => {
    const { overlay } = openOverlay({ text: "Only text", id: "", message: "" });
    expect(overlay.el.querySelector(".vu-error-message")).toBeNull();
    expect(overlay.el.querySelector(".vu-error-id")).toBeNull();
    expect(reportBox(overlay).textContent).toBe("Only text");
    overlay.destroy();
  });

  it("takes every label from the translator, with the values that a label names", () => {
    const t = vi.fn((key: string, args?: Readonly<Record<string, string | number>>) =>
      args === undefined ? `[${key}]` : `[${key} ${JSON.stringify(args)}]`,
    );
    const { overlay } = openOverlay({ text: "T", id: "r-1" }, { t });
    expect(overlay.el.querySelector(".vu-modal-title")?.textContent).toBe("[error.title]");
    expect(action(overlay, "copy").textContent).toBe("[error.copy]");
    expect(action(overlay, "continue").textContent).toBe("[error.continue]");
    expect(action(overlay, "reload").textContent).toBe("[error.reload]");
    expect(overlay.el.querySelector(".vu-error-id")?.textContent).toBe('[error.reportId {"id":"r-1"}]');
    overlay.destroy();
  });
});

describe("closing", () => {
  it("Continue closes the overlay, and calls onContinue once it is closed", () => {
    let openWhenCalled: boolean | null = null;
    const made = openOverlay("T", {
      onContinue: () => {
        openWhenCalled = made.overlay.isOpen;
      },
    });
    action(made.overlay, "continue").click();
    expect(made.overlay.isOpen).toBe(false);
    expect(made.overlay.el.hidden).toBe(true);
    expect(openWhenCalled).toBe(false);
    made.overlay.destroy();
  });

  it("Escape closes it the same way, and cancels the key", () => {
    const onContinue = vi.fn();
    const { overlay } = openOverlay("T", { onContinue });
    const event = press("Escape");
    expect(event.defaultPrevented).toBe(true);
    expect(overlay.isOpen).toBe(false);
    expect(onContinue).toHaveBeenCalledTimes(1);
  });

  it("close() hides it without calling onContinue, and can be called again", () => {
    const onContinue = vi.fn();
    const { overlay } = openOverlay("T", { onContinue });
    overlay.close();
    overlay.close();
    expect(overlay.isOpen).toBe(false);
    expect(overlay.el.hidden).toBe(true);
    expect(onContinue).not.toHaveBeenCalled();
    overlay.destroy();
  });

  it("gives the focus back to the element that had it before it opened", () => {
    const trigger = document.createElement("button");
    document.body.append(trigger);
    trigger.focus();
    const { overlay } = openOverlay();
    expect(document.activeElement).not.toBe(trigger);
    overlay.close();
    expect(document.activeElement).toBe(trigger);
    overlay.destroy();
  });

  it("releases the focus when the element that had it is gone", () => {
    const trigger = document.createElement("button");
    document.body.append(trigger);
    trigger.focus();
    const { overlay } = openOverlay();
    trigger.remove();
    overlay.close();
    expect(overlay.el.contains(document.activeElement)).toBe(false);
    overlay.destroy();
  });

  it("removes its element from the host on destroy, is safe to destroy twice, and ignores keys afterwards", () => {
    const onContinue = vi.fn();
    const { host, overlay } = openOverlay("T", { onContinue });
    overlay.destroy();
    overlay.destroy();
    expect(host.children).toHaveLength(0);
    expect(overlay.isOpen).toBe(false);
    press("Escape");
    expect(onContinue).not.toHaveBeenCalled();
  });
});

describe("keys and focus", () => {
  it("wraps Tab from the last part to the first, and Shift+Tab back again", () => {
    const { overlay } = openOverlay();
    action(overlay, "reload").focus();
    expect(press("Tab").defaultPrevented).toBe(true);
    expect(document.activeElement).toBe(reportBox(overlay));
    expect(press("Tab", true).defaultPrevented).toBe(true);
    expect(document.activeElement).toBe(action(overlay, "reload"));
    overlay.destroy();
  });

  it("leaves Tab alone between the parts in the middle", () => {
    const { overlay } = openOverlay();
    action(overlay, "copy").focus();
    expect(press("Tab").defaultPrevented).toBe(false);
    overlay.destroy();
  });

  it("moves between the buttons with the left and right arrows, and wraps at both ends", () => {
    const { overlay } = openOverlay();
    action(overlay, "continue").focus();
    expect(press("ArrowRight").defaultPrevented).toBe(true);
    expect(document.activeElement).toBe(action(overlay, "reload"));
    press("ArrowRight");
    expect(document.activeElement).toBe(action(overlay, "copy"));
    press("ArrowLeft");
    expect(document.activeElement).toBe(action(overlay, "reload"));
    overlay.destroy();
  });

  it("leaves the arrows alone in the report box, so that the text scrolls", () => {
    const { overlay } = openOverlay("a long report");
    reportBox(overlay).focus();
    expect(press("ArrowDown").defaultPrevented).toBe(false);
    expect(press("ArrowLeft").defaultPrevented).toBe(false);
    overlay.destroy();
  });

  it("brings the focus back to Continue when something outside takes it", () => {
    const { overlay } = openOverlay();
    const outside = document.createElement("button");
    document.body.append(outside);
    outside.focus();
    outside.dispatchEvent(new FocusEvent("focusin", { bubbles: true }));
    expect(document.activeElement).toBe(action(overlay, "continue"));
    overlay.destroy();
  });
});

describe("buttons", () => {
  it("Reload calls onReload and leaves the overlay open, for the game to decide", () => {
    const onReload = vi.fn();
    const { overlay } = openOverlay("T", { onReload });
    action(overlay, "reload").click();
    expect(onReload).toHaveBeenCalledTimes(1);
    expect(overlay.isOpen).toBe(true);
    overlay.destroy();
  });
});

describe("Copy", () => {
  it("copies the text through the clipboard, and says so", async () => {
    const writeText = vi.fn(async () => undefined);
    vi.stubGlobal("navigator", { userAgent: "test", clipboard: { writeText } });
    const onCopy = vi.fn();
    const { overlay } = openOverlay("Full report", { onCopy });
    action(overlay, "copy").click();
    await flush();
    expect(writeText).toHaveBeenCalledWith("Full report");
    expect(onCopy).toHaveBeenCalledWith(true);
    expect(statusText(overlay)).toBe("The report is on the clipboard.");
    overlay.destroy();
  });

  it("selects the text and reports the failure when there is no clipboard", async () => {
    vi.stubGlobal("navigator", { userAgent: "test" });
    const execCommand = vi.fn(() => false);
    Object.defineProperty(document, "execCommand", { configurable: true, value: execCommand });
    const onCopy = vi.fn();
    const { overlay } = openOverlay("Full report", { onCopy });
    action(overlay, "copy").click();
    await flush();
    expect(execCommand).toHaveBeenCalledWith("copy");
    expect(onCopy).toHaveBeenCalledWith(false);
    expect(statusText(overlay)).toBe(
      "The clipboard is not available. The report is selected: press Ctrl+C to copy it.",
    );
    overlay.destroy();
  });

  it("falls back to the copy command when the clipboard refuses", async () => {
    const writeText = vi.fn(() => Promise.reject(new Error("denied")));
    vi.stubGlobal("navigator", { userAgent: "test", clipboard: { writeText } });
    const execCommand = vi.fn(() => true);
    Object.defineProperty(document, "execCommand", { configurable: true, value: execCommand });
    const onCopy = vi.fn();
    const { overlay } = openOverlay("Full report", { onCopy });
    action(overlay, "copy").click();
    await flush();
    expect(writeText).toHaveBeenCalledTimes(1);
    expect(execCommand).toHaveBeenCalledWith("copy");
    expect(onCopy).toHaveBeenCalledWith(true);
    overlay.destroy();
  });
});

describe("the stylesheet", () => {
  const rulesText = css.replace(/\/\*[\s\S]*?\*\//g, "");

  it("has a rule for every class that the overlay creates", () => {
    const names = ["vu-error-root", "vu-error-dialog", "vu-error-message", "vu-error-id", "vu-error-report", "vu-error-status"];
    for (const name of names) {
      expect(rulesText, name).toMatch(new RegExp(`\\.${name}(?![\\w-])[^{]*\\{`));
    }
  });

  it("keeps the overlay root hidden when it carries the hidden attribute", () => {
    expect(rulesText).toMatch(/\.vu-error-root\[hidden\]\s*\{\s*display:\s*none/);
  });

  it("changes only the colour of the report box when it has the focus", () => {
    const body = /\.vu-error-report:focus-visible\s*\{([^}]*)\}/.exec(rulesText)?.[1] ?? "";
    const properties = body
      .split(";")
      .map((part) => part.split(":")[0]?.trim() ?? "")
      .filter((name) => name !== "");
    expect(properties).toEqual(["background-color"]);
  });
});
