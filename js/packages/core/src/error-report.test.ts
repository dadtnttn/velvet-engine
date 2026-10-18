// Tests for the error reports: the listeners on a target, what a report holds, repeats that are counted, the limit
// of new reports per minute, the game's identity from the options or the bridge, captureError, formatReport, and
// uninstalling. The tests use a plain EventTarget, so they need no page.

import { afterEach, describe, expect, it, vi } from "vitest";
import {
  captureError,
  formatReport,
  installErrorReporter,
  type ErrorReport,
  type ErrorReporter,
  type ErrorReporterOptions,
} from "./error-report";
import { createLogger, type Logger } from "./log";
import { createFakeBridge } from "./test-support";

/** Every reporter that a test installed, so that each test leaves no listener behind. */
const installed: ErrorReporter[] = [];

afterEach(() => {
  for (const reporter of installed.splice(0)) reporter.uninstall();
  vi.unstubAllGlobals();
});

/** A logger that keeps its entries in memory, with no console output. */
function memoryLogger(): Logger {
  return createLogger({ name: "game", sinks: [] });
}

/**
 * A reporter on a fresh target, with a clock that the test moves. The logger is returned too, so that a test can
 * write entries before an error happens.
 */
function setup(extra: Omit<ErrorReporterOptions, "logger" | "target" | "now" | "onReport"> = {}): {
  logger: Logger;
  target: EventTarget;
  clock: { now: number };
  sent: ErrorReport[];
  reporter: ErrorReporter;
} {
  const logger = memoryLogger();
  const target = new EventTarget();
  const clock = { now: 1_000_000 };
  const sent: ErrorReport[] = [];
  const reporter = installErrorReporter({
    logger,
    target,
    now: () => clock.now,
    onReport: (report) => {
      sent.push(report);
    },
    ...extra,
  });
  installed.push(reporter);
  return { logger, target, clock, sent, reporter };
}

/** An error event as the browser sends it for an uncaught error: the error, and the message. */
function errorEvent(error: unknown, message = ""): Event {
  const event = new Event("error");
  Object.defineProperties(event, { error: { value: error }, message: { value: message } });
  return event;
}

/** An unhandled rejection event, with its reason. */
function rejectionEvent(reason: unknown): Event {
  const event = new Event("unhandledrejection");
  Object.defineProperty(event, "reason", { value: reason });
  return event;
}

/** An error whose stack is the text given, so that two errors with one message can still differ. */
function errorWithStack(message: string, stack: string): Error {
  const error = new Error(message);
  error.stack = stack;
  return error;
}

describe("the listeners", () => {
  it("makes a report from an uncaught error, with its message, stack and kind", () => {
    const { target, sent } = setup({ game: { id: "demon-queen", title: "Demon Queen", version: "0.2.0" } });
    const error = new TypeError("boom");
    target.dispatchEvent(errorEvent(error));
    expect(sent).toHaveLength(1);
    expect(sent[0]).toMatchObject({
      kind: "error",
      message: "boom",
      stack: error.stack,
      count: 1,
      context: {},
      game: { id: "demon-queen", title: "Demon Queen", version: "0.2.0" },
    });
    expect(sent[0]?.userAgent).toBeTypeOf("string");
    expect(sent[0]?.id).toBeTypeOf("string");
  });

  it("uses the event's message when the event carries no error", () => {
    const { target, sent } = setup();
    target.dispatchEvent(errorEvent(null, "Script error."));
    expect(sent[0]).toMatchObject({ kind: "error", message: "Script error.", stack: null });
  });

  it("reports an unhandled rejection with a text reason as its message", () => {
    const { target, sent } = setup();
    target.dispatchEvent(rejectionEvent("nope"));
    expect(sent[0]).toMatchObject({ kind: "unhandledrejection", message: "nope", stack: null });
  });

  it("reports an unhandled rejection with an error reason, with the error's stack", () => {
    const { target, sent } = setup();
    const error = new RangeError("too far");
    target.dispatchEvent(rejectionEvent(error));
    expect(sent[0]).toMatchObject({ kind: "unhandledrejection", message: "too far", stack: error.stack });
  });

  it("writes a reason that is not text as JSON", () => {
    const { target, sent } = setup();
    target.dispatchEvent(rejectionEvent({ code: 42 }));
    expect(sent[0]?.message).toBe('{"code":42}');
  });

  it("writes each error to the logger at the error level, with its kind", () => {
    const { logger, target } = setup();
    target.dispatchEvent(errorEvent(new Error("boom")));
    const last = logger.entries().at(-1);
    expect(last).toMatchObject({ level: "error", message: "boom", fields: { kind: "error" } });
  });

  it("gives the game from the bridge when the options name none", () => {
    const { bridge } = createFakeBridge();
    vi.stubGlobal("velvet", bridge);
    const { target, sent } = setup();
    target.dispatchEvent(errorEvent(new Error("boom")));
    expect(sent[0]?.game).toEqual({ id: "demo", title: "Demo", version: "0.1.0" });
  });

  it("has no game outside Velvet Desktop when the options name none", () => {
    const { target, sent } = setup();
    target.dispatchEvent(errorEvent(new Error("boom")));
    expect(sent[0]?.game).toBeNull();
  });

  it("gives each report its own id", () => {
    const { target, sent } = setup();
    target.dispatchEvent(errorEvent(new Error("one")));
    target.dispatchEvent(errorEvent(new Error("two")));
    expect(sent[0]?.id).not.toBe(sent[1]?.id);
  });
});

describe("repeats", () => {
  it("counts a repeat of the same error, and does not report it again", () => {
    const { logger, target, sent, reporter } = setup();
    const error = errorWithStack("again", "stack line");
    target.dispatchEvent(errorEvent(error));
    target.dispatchEvent(errorEvent(error));
    expect(sent).toHaveLength(1);
    expect(reporter.reports()[0]?.count).toBe(2);
    expect(logger.entries().filter((entry) => entry.level === "error")).toHaveLength(2);
  });

  it("takes the same message with another stack as another error", () => {
    const { target, sent } = setup();
    target.dispatchEvent(errorEvent(errorWithStack("same", "A")));
    target.dispatchEvent(errorEvent(errorWithStack("same", "B")));
    expect(sent).toHaveLength(2);
  });

  it("lists the errors most recent first, each with its current count", () => {
    const { target, reporter } = setup();
    const first = errorWithStack("first", "F");
    target.dispatchEvent(errorEvent(first));
    target.dispatchEvent(errorEvent(first));
    target.dispatchEvent(errorEvent(errorWithStack("second", "S")));
    expect(reporter.reports().map((report) => [report.message, report.count])).toEqual([
      ["second", 1],
      ["first", 2],
    ]);
  });
});

describe("the limit of new reports per minute", () => {
  it("passes at most maxPerMinute new reports in a minute, and still counts and lists the rest", () => {
    const { target, sent, reporter } = setup({ maxPerMinute: 2 });
    for (const name of ["one", "two", "three"]) target.dispatchEvent(errorEvent(errorWithStack(name, name)));
    expect(sent.map((report) => report.message)).toEqual(["one", "two"]);
    expect(reporter.reports()).toHaveLength(3);
  });

  it("passes new reports again once a minute has gone by", () => {
    const { target, clock, sent } = setup({ maxPerMinute: 1 });
    target.dispatchEvent(errorEvent(errorWithStack("first", "1")));
    target.dispatchEvent(errorEvent(errorWithStack("second", "2")));
    expect(sent).toHaveLength(1);
    clock.now += 60_000;
    target.dispatchEvent(errorEvent(errorWithStack("third", "3")));
    expect(sent.map((report) => report.message)).toEqual(["first", "third"]);
  });

  it("refuses a limit that is not a whole number from 1 up, and adds no listener", () => {
    const logger = memoryLogger();
    const target = new EventTarget();
    expect(() => installErrorReporter({ logger, target, maxPerMinute: 0 })).toThrow(RangeError);
    target.dispatchEvent(errorEvent(new Error("unheard")));
    expect(logger.entries()).toHaveLength(0);
  });
});

describe("captureError", () => {
  it("reports a caught error with its context, and returns the report", () => {
    const { sent } = setup();
    const report = captureError(new Error("save failed"), { slot: "3", scene: "mine" });
    expect(report).toMatchObject({ kind: "caught", message: "save failed", context: { slot: "3", scene: "mine" } });
    expect(sent).toHaveLength(1);
  });

  it("counts a repeat of a caught error, and returns its current count", () => {
    setup();
    const error = errorWithStack("again", "same");
    captureError(error);
    expect(captureError(error)?.count).toBe(2);
  });

  it("gives the context as JSON, even when it holds an error, a function or a cycle", () => {
    setup();
    const cyclic: Record<string, unknown> = {};
    cyclic["self"] = cyclic;
    const report = captureError("odd", { error: new Error("inner"), fn: () => 1, cyclic });
    expect(report?.context).toMatchObject({
      error: { name: "Error", message: "inner" },
      fn: "[Function fn]",
      cyclic: { self: "[Circular]" },
    });
  });

  it("returns null when no reporter is installed, and after the reporter is uninstalled", () => {
    expect(captureError("nobody listens")).toBeNull();
    const { reporter } = setup();
    reporter.uninstall();
    expect(captureError("not any more")).toBeNull();
  });
});

describe("uninstall", () => {
  it("stops listening, and calling it twice does nothing", () => {
    const { logger, target, sent, reporter } = setup();
    reporter.uninstall();
    reporter.uninstall();
    target.dispatchEvent(errorEvent(new Error("after")));
    target.dispatchEvent(rejectionEvent("after too"));
    expect(sent).toHaveLength(0);
    expect(logger.entries()).toHaveLength(0);
  });
});

describe("a handler that fails", () => {
  it("is logged, and the reporter carries on", () => {
    const logger = memoryLogger();
    const target = new EventTarget();
    const reporter = installErrorReporter({
      logger,
      target,
      onReport: () => {
        throw new Error("handler broke");
      },
    });
    installed.push(reporter);
    expect(() => target.dispatchEvent(errorEvent(new Error("first")))).not.toThrow();
    target.dispatchEvent(errorEvent(new Error("second")));
    expect(reporter.reports()).toHaveLength(2);
    expect(logger.entries().map((entry) => entry.message)).toContain("the error report handler failed");
  });
});

describe("formatReport", () => {
  it("writes the report as plain text: the id, the game, the error, the context, the agent and the log", () => {
    const { logger } = setup({ game: { id: "demo", title: "Demo", version: "0.1.0" } });
    logger.info("loading zone", { zone: 3 });
    logger.warn("low health", { hp: 2 });
    const report = captureError(new Error("save failed"), { slot: "3", scene: "mine" });
    if (report === null) throw new Error("no reporter");
    const text = formatReport(report);
    const lines = text.split("\n");
    expect(lines[0]).toBe(`Error report ${report.id}`);
    expect(text).toContain("Game: Demo (demo) 0.1.0");
    expect(text).toContain("Kind: caught");
    expect(text).toContain("Message: save failed");
    expect(text).toContain("Count: 1");
    expect(text).toContain('"scene": "mine"');
    expect(text).toContain("User agent: ");
    expect(text).toContain("Recent log (3 entries, oldest first):");
    const logStart = lines.findIndex((line) => line.startsWith("Recent log"));
    const logLines = lines.slice(logStart + 1);
    expect(logLines[0]).toMatch(/ INFO game: loading zone \{"zone":3\}$/);
    expect(logLines[1]).toMatch(/ WARN game: low health \{"hp":2\}$/);
    expect(logLines[2]).toMatch(/ ERROR game: save failed /);
  });

  it("says the game is unknown when there is none", () => {
    const { target, sent } = setup();
    target.dispatchEvent(errorEvent(new Error("boom")));
    expect(formatReport(sent[0] as ErrorReport)).toContain("Game: unknown");
  });

  it("leaves out the context section when there is no context", () => {
    const { target, sent } = setup();
    target.dispatchEvent(errorEvent(new Error("boom")));
    const text = formatReport(sent[0] as ErrorReport);
    expect(text).not.toContain("Context:");
    expect(text).toContain("Recent log (1 entries, oldest first):");
  });

  it("says so when the report holds no log entries", () => {
    const { target, sent } = setup();
    target.dispatchEvent(errorEvent(new Error("boom")));
    const report = sent[0] as ErrorReport;
    expect(formatReport({ ...report, recentLog: [] })).toContain("(none)");
  });
});
