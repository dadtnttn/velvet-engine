// @vitest-environment happy-dom
// Tests for the Log tab of the devtools: the entries and their two filters, the error reports above them (closed
// until opened, and kept open when a new report arrives), the copy button of a report, the log.level command, and
// the error count that a watch can show. The frames come from a scheduler that the test drives.

import { createLogger, installErrorReporter, type FrameScheduler, type Logger } from "@velvet/core";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { createDevtools } from "./devtools";
import { STYLE_ID } from "./styles";
import type { Devtools, DevtoolsOptions } from "./types";

/** A frame scheduler that the test drives. `tick` runs the frames that are waiting, with the time it gives. */
interface ManualScheduler extends FrameScheduler {
  tick(timeMs: number): void;
}

/** A scheduler whose frames run only when the test ticks it. */
function manualScheduler(): ManualScheduler {
  let next = 1;
  let waiting = new Map<number, (timeMs: number) => void>();
  return {
    request(callback) {
      const handle = next;
      next += 1;
      waiting.set(handle, callback);
      return handle;
    },
    cancel(handle) {
      waiting.delete(handle);
    },
    tick(timeMs) {
      const frames = [...waiting.values()];
      waiting = new Map();
      for (const frame of frames) frame(timeMs);
    },
  };
}

/** The first element that matches, or an error that names the selector. */
function query<Element extends HTMLElement>(selector: string): Element {
  const found = document.querySelector<Element>(selector);
  if (found === null) throw new Error(`no element matches ${selector}`);
  return found;
}

/** The tab button with a label. */
function tabButton(label: string): HTMLButtonElement {
  const found = [...document.querySelectorAll<HTMLButtonElement>('[role="tab"]')].find(
    (button) => button.textContent === label,
  );
  if (found === undefined) throw new Error(`no tab ${label}`);
  return found;
}

/** The page of the tab with a label. The pages are in the same order as the tabs. */
function tabPage(label: string): HTMLElement {
  const index = [...document.querySelectorAll('[role="tab"]')].findIndex((button) => button.textContent === label);
  const page = document.querySelectorAll<HTMLElement>('[role="tabpanel"]')[index];
  if (page === undefined) throw new Error(`no page for ${label}`);
  return page;
}

/** The button with a label, anywhere in the document. */
function buttonLabelled(label: string): HTMLButtonElement {
  const found = [...document.querySelectorAll<HTMLButtonElement>("button")].find(
    (button) => button.textContent === label,
  );
  if (found === undefined) throw new Error(`no button ${label}`);
  return found;
}

/** The text of each entry row's message, newest first, as the tab shows them. */
function messages(): Array<string | null> {
  return [...document.querySelectorAll(".vdt-entry-message")].map((node) => node.textContent);
}

/** An error event as the browser sends it for an uncaught error. */
function errorEvent(error: Error): Event {
  const event = new Event("error");
  Object.defineProperty(event, "error", { value: error });
  return event;
}

/** A key press on the console prompt: the line is typed, then Enter runs it. */
function runCommand(line: string): void {
  const prompt = query<HTMLInputElement>('input[aria-label="Console command"]');
  prompt.value = line;
  prompt.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true, cancelable: true }));
}

/** The text of everything the console printed. */
function consoleText(): string {
  return query(".vdt-log").textContent ?? "";
}

/** Lets the callbacks of resolved promises run. */
function flush(): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, 0));
}

let host: HTMLElement;
let scheduler: ManualScheduler;
let reporters: Array<{ uninstall(): void }> = [];

beforeEach(() => {
  host = document.createElement("div");
  document.body.append(host);
  scheduler = manualScheduler();
});

afterEach(() => {
  for (const reporter of reporters.splice(0)) reporter.uninstall();
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
  document.body.innerHTML = "";
  document.getElementById(STYLE_ID)?.remove();
});

/** Enabled devtools on the test host, driven by the test's scheduler. */
function tools(options: Partial<DevtoolsOptions> = {}): Devtools {
  return createDevtools({ host, enabled: true, scheduler, ...options });
}

/** Opens the panel on its Log tab, and draws it. */
function openLogTab(devtools: Devtools): void {
  devtools.show();
  tabButton("Log").click();
  scheduler.tick(0);
}

/** Opens the panel on its Console tab. */
function openConsole(devtools: Devtools): void {
  devtools.show();
  tabButton("Console").click();
}

/** A logger with no console output, for the tests that read the Log tab. */
function quietRoot(): Logger {
  return createLogger({ name: "game", sinks: [] });
}

describe("the entries", () => {
  it("lists the entries newest first, with their level, logger, message and fields, and counts them", () => {
    const logger = quietRoot();
    logger.info("loading");
    logger.error("broken", { zone: 3 });
    openLogTab(tools({ logger }));
    const rows = [...document.querySelectorAll(".vdt-entry")].map((row) => row.textContent ?? "");
    expect(rows).toHaveLength(2);
    expect(rows[0]).toContain("ERROR");
    expect(rows[0]).toContain("game");
    expect(rows[0]).toContain("broken");
    expect(rows[0]).toContain('{"zone":3}');
    expect(rows[1]).toContain("INFO");
    expect(tabPage("Log").textContent).toContain("2 of 2 entries shown");
  });

  it("filters by the lowest level shown", () => {
    const logger = quietRoot();
    logger.setLevel("debug");
    logger.debug("tick");
    logger.info("loaded");
    logger.warn("low health");
    logger.error("miss");
    openLogTab(tools({ logger }));
    const level = query<HTMLSelectElement>('select[aria-label="Lowest level shown"]');
    level.value = "warn";
    level.dispatchEvent(new Event("change"));
    expect(messages()).toEqual(["miss", "low health"]);
    level.value = "debug";
    level.dispatchEvent(new Event("change"));
    expect(messages()).toHaveLength(4);
  });

  it("filters by text in the logger, the message or the fields, ignoring case", () => {
    const logger = quietRoot();
    logger.info("loaded zone", { zone: "mine" });
    logger.warn("low health");
    logger.child("combat").error("miss");
    openLogTab(tools({ logger }));
    const text = query<HTMLInputElement>('input[aria-label="Filter the log by text"]');
    text.value = "ZONE";
    text.dispatchEvent(new Event("input"));
    expect(messages()).toEqual(["loaded zone"]);
    text.value = "combat";
    text.dispatchEvent(new Event("input"));
    expect(messages()).toEqual(["miss"]);
    text.value = "";
    text.dispatchEvent(new Event("input"));
    expect(messages()).toHaveLength(3);
  });

  it("redraws when the log changes, on the next tick", () => {
    const logger = quietRoot();
    openLogTab(tools({ logger }));
    expect(messages()).toHaveLength(0);
    logger.info("new");
    scheduler.tick(200);
    expect(messages()).toEqual(["new"]);
  });

  it("says that no logger was given when the options name none", () => {
    openLogTab(tools());
    expect(tabPage("Log").textContent).toContain("No logger was given");
    expect(document.querySelectorAll(".vdt-entry")).toHaveLength(0);
  });
});

describe("the error reports", () => {
  it("shows the reports above the entries, each closed until it is opened", () => {
    const logger = quietRoot();
    const target = new EventTarget();
    const reporter = installErrorReporter({ logger, target });
    reporters.push(reporter);
    target.dispatchEvent(errorEvent(new Error("first crash")));
    openLogTab(tools({ logger, errorReports: reporter }));
    const details = document.querySelectorAll<HTMLDetailsElement>(".vdt-report");
    expect(details).toHaveLength(1);
    expect(details[0]?.open).toBe(false);
    expect(details[0]?.textContent).toContain("first crash");
    expect(details[0]?.querySelector(".vdt-report-text")?.textContent).toContain("Message: first crash");
  });

  it("keeps an opened report open when a new report arrives", () => {
    const logger = quietRoot();
    const target = new EventTarget();
    const reporter = installErrorReporter({ logger, target });
    reporters.push(reporter);
    target.dispatchEvent(errorEvent(new Error("first crash")));
    openLogTab(tools({ logger, errorReports: reporter }));
    const first = query<HTMLDetailsElement>(".vdt-report");
    first.open = true;
    first.dispatchEvent(new Event("toggle"));
    target.dispatchEvent(errorEvent(new Error("second crash")));
    scheduler.tick(200);
    const details = [...document.querySelectorAll<HTMLDetailsElement>(".vdt-report")];
    expect(details).toHaveLength(2);
    // Each report's headline names its error; the full text may also quote the other error in its log lines.
    const headlineOf = (item: HTMLDetailsElement): string => item.querySelector("summary")?.textContent ?? "";
    expect(details.find((item) => headlineOf(item).includes("first crash"))?.open).toBe(true);
    expect(details.find((item) => headlineOf(item).includes("second crash"))?.open).toBe(false);
  });

  it("copies a report's full text with its button, and says so", async () => {
    const writeText = vi.fn(async () => undefined);
    vi.stubGlobal("navigator", { userAgent: "test", clipboard: { writeText } });
    const logger = quietRoot();
    const target = new EventTarget();
    const reporter = installErrorReporter({ logger, target });
    reporters.push(reporter);
    target.dispatchEvent(errorEvent(new Error("first crash")));
    openLogTab(tools({ logger, errorReports: reporter }));
    buttonLabelled("Copy report").click();
    await flush();
    expect(writeText).toHaveBeenCalledWith(expect.stringContaining("Message: first crash"));
    expect(tabPage("Log").textContent).toContain("Copied report");
  });

  it("says the clipboard is not there when the page has none", () => {
    vi.stubGlobal("navigator", { userAgent: "test" });
    const logger = quietRoot();
    const target = new EventTarget();
    const reporter = installErrorReporter({ logger, target });
    reporters.push(reporter);
    target.dispatchEvent(errorEvent(new Error("first crash")));
    openLogTab(tools({ logger, errorReports: reporter }));
    buttonLabelled("Copy report").click();
    expect(tabPage("Log").textContent).toContain("The clipboard is not available here.");
  });
});

describe("the log.level command", () => {
  it("sets the level of a logger by its full name, and the new level shows in the Log tab", () => {
    const logger = quietRoot();
    const combat = logger.child("combat");
    const devtools = tools({ logger });
    openConsole(devtools);
    runCommand("log.level game.combat warn");
    expect(combat.level).toBe("warn");
    expect(consoleText()).toContain("game.combat is now warn");
    devtools.show();
    tabButton("Log").click();
    scheduler.tick(0);
    expect(tabPage("Log").textContent).toContain("game.combat warn");
  });

  it("prints an error for a logger that does not exist, and for a level that is not one of the four", () => {
    const logger = quietRoot();
    openConsole(tools({ logger }));
    runCommand("log.level game.nope warn");
    expect(consoleText()).toContain('no logger named "game.nope": the loggers are game');
    runCommand("log.level game loud");
    expect(consoleText()).toContain("level must be one of debug, info, warn, error");
    expect(logger.level).toBe("info");
  });

  it("says that no logger was given when the options name none", () => {
    openConsole(tools());
    runCommand("log.level game debug");
    expect(consoleText()).toContain("no logger was given");
  });

  it("is listed by help, with its arguments", () => {
    openConsole(tools());
    runCommand("help log.level");
    expect(consoleText()).toContain("log.level <name:string> <level:debug|info|warn|error>");
  });
});

describe("the error count", () => {
  it("counts the error entries, so that a watch can show it, and the count moves as errors are logged", () => {
    const logger = quietRoot();
    const devtools = tools({ logger });
    devtools.watch("errors", () => devtools.errorCount());
    scheduler.tick(0);
    expect(devtools.errorCount()).toBe(0);
    logger.error("one");
    logger.warn("not an error");
    logger.error("two");
    scheduler.tick(200);
    expect(devtools.errorCount()).toBe(2);
    expect(query(".vdt-hud").textContent).toContain("errors");
    expect(query(".vdt-hud").textContent).toContain("2");
  });

  it("is zero when no logger was given", () => {
    expect(tools().errorCount()).toBe(0);
  });

  it("is zero, and does nothing, when the tools are disabled", () => {
    const devtools = createDevtools({ host, enabled: false });
    expect(devtools.errorCount()).toBe(0);
    expect(document.querySelector(".vdt-root")).toBeNull();
  });
});
