// @vitest-environment happy-dom
// Tests of the error reports: an uncaught error is logged with the game's logger, and the first report shows the error
// overlay over the page, with its text ready to copy.

import { afterEach, describe, expect, it } from "vitest";
import { createLogger, type ErrorReporter, type LogEntry } from "@velvet/core";
import { reportErrors } from "./log";

describe("error reports", () => {
  let reporter: ErrorReporter | null = null;

  afterEach(() => {
    reporter?.uninstall();
    reporter = null;
    document.body.innerHTML = "";
  });

  it("logs an uncaught error, and shows the overlay with the report", () => {
    const lines: string[] = [];
    const logger = createLogger({
      name: "hello-2d",
      level: "info",
      sinks: [{ write: (entry: LogEntry) => lines.push(`${entry.logger}: ${entry.message}`) }],
    });
    reporter = reportErrors(logger, document.body);
    window.dispatchEvent(new ErrorEvent("error", { error: new Error("boom"), message: "boom" }));

    expect(lines.some((line) => line.startsWith("hello-2d") && line.includes("boom"))).toBe(true);
    expect(document.body.textContent).toContain("Something went wrong");
  });
});
