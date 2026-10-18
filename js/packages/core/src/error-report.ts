// Error reports: what a player sends when the game breaks. installErrorReporter listens for uncaught
// errors and unhandled promise rejections, and captureError takes an error that the game caught itself.
// Each error is written to the logger at the error level, and becomes a report with the game's identity,
// the browser's user agent, the caught context and the latest entries of the log. A repeat of the same
// error is counted rather than reported again, and only a few new reports reach the game each minute.
// formatReport turns a report into plain text that a player can paste into a bug report.

import { getBridge } from "./bridge";
import {
  errorParts,
  toJsonObject,
  toJsonValue,
  type JsonObject,
  type LogEntry,
  type LogFields,
  type Logger,
} from "./log";

/** Where a report came from: an uncaught error, an unhandled rejection, or an error the game caught. */
export type ErrorReportKind = "error" | "unhandledrejection" | "caught";

/** The game that a report is about. */
export interface ReportGame {
  /** The game's id, from its manifest. */
  readonly id: string;
  /** The game's title. */
  readonly title: string;
  /** The game's version, such as "0.2.0". */
  readonly version: string;
}

/** One error report. */
export interface ErrorReport {
  /** Unique for each report on the page, so a player can name it in a bug report. */
  readonly id: string;
  /** When the error first happened, in milliseconds since the Unix epoch. */
  readonly time: number;
  /** Where the error came from. */
  readonly kind: ErrorReportKind;
  /** The error's message. */
  readonly message: string;
  /** The stack trace, or null when the error had none. */
  readonly stack: string | null;
  /** How many times this same error has happened. `ErrorReporter.reports()` gives the current count. */
  readonly count: number;
  /** The game: from the options when they name one, else from the bridge, else null outside Velvet Desktop. */
  readonly game: ReportGame | null;
  /** The browser's user agent, which names the engine and the platform. */
  readonly userAgent: string;
  /** The context that `captureError` was given, as JSON. Empty for an error the reporter found by itself. */
  readonly context: Readonly<JsonObject>;
  /** The latest entries of the log when the error first happened, oldest first. */
  readonly recentLog: readonly LogEntry[];
}

/** Options for {@link installErrorReporter}. */
export interface ErrorReporterOptions {
  /** The logger that errors are written to, and whose latest entries go into each report. */
  readonly logger: Logger;
  /** Where the listeners go. Default: the window. */
  readonly target?: EventTarget;
  /** Called with each new report. A repeat of an error is counted, and is not reported again. */
  readonly onReport?: (report: ErrorReport) => void;
  /**
   * The game's identity. When given, it is used instead of the bridge's game info. Without it, the bridge's game
   * is used, or null outside Velvet Desktop.
   */
  readonly game?: ReportGame;
  /**
   * How many new reports may reach `onReport` in one minute. Default 5. Errors over the limit are still logged,
   * counted and listed by `reports()`, but they are not passed on.
   */
  readonly maxPerMinute?: number;
  /** The clock, in milliseconds since the Unix epoch. Default: Date.now. */
  readonly now?: () => number;
}

/** A reporter that is listening. Make one with {@link installErrorReporter}. */
export interface ErrorReporter {
  /** The errors seen so far, most recent first, each with its current count. Errors over the rate limit are included. */
  reports(): readonly ErrorReport[];
  /** Stops listening. Calling it again does nothing. */
  uninstall(): void;
}

/** How many of the latest log entries a report keeps. */
const RECENT_LOG_LIMIT = 50;

/** How many new reports may reach the game in one minute, when the options name no limit. */
const DEFAULT_MAX_PER_MINUTE = 5;

/** How many distinct errors are remembered for counting their repeats. The oldest are forgotten first. */
const TRACKED_LIMIT = 100;

/** One minute, in milliseconds. */
const MINUTE_MS = 60_000;

/** The message of an error that carries no text at all. */
const UNKNOWN_ERROR = "Unknown error";

/** The reporter that captureError sends to: the one installed last, until it is uninstalled. */
let active: Reporter | null = null;

/** The number of the last report made on this page. Each report takes the next number, so ids do not repeat. */
let lastReportNumber = 0;

/** A distinct error that the reporter has seen: its first report, and how often it has happened. */
interface Tracked {
  readonly report: ErrorReport;
  count: number;
}

/** The part of a reporter that does the work: it logs each error, makes or counts its report, and passes new ones on. */
class Reporter {
  private readonly logger: Logger;
  private readonly onReport: ((report: ErrorReport) => void) | undefined;
  private readonly game: ReportGame | undefined;
  private readonly maxPerMinute: number;
  private readonly now: () => number;
  private readonly tracked = new Map<string, Tracked>();
  /** The times of the reports that reached the game in the last minute, oldest first. */
  private readonly sentAt: number[] = [];

  /** @param options - the reporter's options. Throws a RangeError when the rate limit is not a whole number from 1 up. */
  constructor(options: ErrorReporterOptions) {
    const maxPerMinute = options.maxPerMinute ?? DEFAULT_MAX_PER_MINUTE;
    if (!Number.isInteger(maxPerMinute) || maxPerMinute < 1) {
      throw new RangeError(`maxPerMinute must be a whole number from 1 up, not ${String(maxPerMinute)}`);
    }
    this.logger = options.logger;
    this.onReport = options.onReport;
    this.game = options.game;
    this.maxPerMinute = maxPerMinute;
    this.now = options.now ?? Date.now;
  }

  /**
   * Handles one error: writes it to the log at the error level, then makes a report, or counts a repeat of an
   * error that was already reported. Its own errors are caught by the callers, so it never throws into a page.
   */
  capture(kind: ErrorReportKind, value: unknown, context: Readonly<JsonObject>): ErrorReport {
    const { message, stack } = describeValue(value);
    this.logger.error(message, { ...context, kind });
    const key = `${kind}\n${message}\n${stack ?? ""}`;
    const known = this.tracked.get(key);
    if (known !== undefined) {
      known.count += 1;
      return this.snapshot(known);
    }
    const now = this.now();
    const report: ErrorReport = {
      id: nextReportId(now),
      time: now,
      kind,
      message,
      stack,
      count: 1,
      game: this.gameInfo(),
      userAgent: userAgentText(),
      context,
      recentLog: this.logger.entries().slice(-RECENT_LOG_LIMIT),
    };
    const entry: Tracked = { report, count: 1 };
    this.remember(key, entry);
    if (this.takeSlot(now)) this.deliver(report);
    return this.snapshot(entry);
  }

  /** The errors seen so far, most recent first, with their current counts. */
  reports(): ErrorReport[] {
    return [...this.tracked.values()].reverse().map((entry) => this.snapshot(entry));
  }

  /** The game that a report is about: the options' game, else the bridge's, else null. */
  private gameInfo(): ReportGame | null {
    if (this.game !== undefined) return { ...this.game };
    const bridge = getBridge();
    if (bridge === null) return null;
    return { id: bridge.game.id, title: bridge.game.title, version: bridge.game.version };
  }

  /** Keeps a distinct error so that its repeats can be counted. When the list is full, the oldest one goes. */
  private remember(key: string, entry: Tracked): void {
    if (this.tracked.size >= TRACKED_LIMIT) {
      const oldest = this.tracked.keys().next();
      if (!oldest.done) this.tracked.delete(oldest.value);
    }
    this.tracked.set(key, entry);
  }

  /** Takes one of the reports that may reach the game in this minute. False when the minute's allowance is used up. */
  private takeSlot(now: number): boolean {
    let oldest = this.sentAt[0];
    while (oldest !== undefined && now - oldest >= MINUTE_MS) {
      this.sentAt.shift();
      oldest = this.sentAt[0];
    }
    if (this.sentAt.length >= this.maxPerMinute) return false;
    this.sentAt.push(now);
    return true;
  }

  /** Passes a new report to the game's handler. A handler that throws is logged, and the reporter carries on. */
  private deliver(report: ErrorReport): void {
    if (this.onReport === undefined) return;
    try {
      this.onReport(report);
    } catch (error) {
      this.logger.error("the error report handler failed", { message: describeValue(error).message });
    }
  }

  /** A copy of a tracked report, with its current count. */
  private snapshot(entry: Tracked): ErrorReport {
    return { ...entry.report, count: entry.count };
  }
}

/**
 * Starts reporting the errors of the page: uncaught errors and unhandled promise rejections. Each one is written
 * to the logger, and becomes a report, or counts as a repeat of a report that was already made. The reporter
 * installed last also receives the errors that `captureError` is given.
 * @param options - the logger, where the listeners go, the handler for new reports, and the game's identity.
 * @returns the reporter, whose `reports()` lists the errors and whose `uninstall()` stops it.
 * @throws RangeError when `maxPerMinute` is not a whole number from 1 up.
 * @example
 * const reporter = installErrorReporter({ logger: log, onReport: (report) => showOverlay(formatReport(report)) });
 */
export function installErrorReporter(options: ErrorReporterOptions): ErrorReporter {
  const reporter = new Reporter(options);
  const target: EventTarget = options.target ?? window;
  const onError = (event: Event): void => {
    guard(() => {
      reporter.capture("error", errorValueOf(event), {});
    });
  };
  const onRejection = (event: Event): void => {
    guard(() => {
      reporter.capture("unhandledrejection", propertyOf(event, "reason"), {});
    });
  };
  target.addEventListener("error", onError);
  target.addEventListener("unhandledrejection", onRejection);
  active = reporter;
  return {
    reports(): ErrorReport[] {
      return reporter.reports();
    },
    uninstall(): void {
      target.removeEventListener("error", onError);
      target.removeEventListener("unhandledrejection", onRejection);
      if (active === reporter) active = null;
    },
  };
}

/**
 * Reports an error that the game caught itself, through the reporter installed last. The error is written to
 * that reporter's logger at the error level, and becomes a report of kind "caught", or counts as a repeat. Use it
 * for an error that the game recovers from, but that a player should still be able to report.
 * @param error - the error, or any value that was thrown.
 * @param context - facts about what the game was doing, such as a scene name. They are kept as JSON.
 * @returns the report with its current count, or null when no reporter is installed.
 */
export function captureError(error: unknown, context: LogFields = {}): ErrorReport | null {
  const reporter = active;
  if (reporter === null) return null;
  try {
    return reporter.capture("caught", error, toJsonObject(context));
  } catch {
    return null;
  }
}

/**
 * The report as plain text, for a player to paste into a bug report: the report id, the game, the time, the
 * error with its stack, the caught context, the user agent and the recent log lines, oldest first. Nothing
 * else is added, so the text holds only what the report already holds.
 * @param report - a report from a reporter, or one that the game built with the same fields.
 */
export function formatReport(report: ErrorReport): string {
  const lines: string[] = [
    `Error report ${report.id}`,
    `Game: ${gameText(report.game)}`,
    `Time: ${isoText(report.time)}`,
    `Kind: ${report.kind}`,
    `Count: ${report.count}`,
    `Message: ${report.message}`,
    "",
    "Stack:",
    report.stack ?? "(no stack)",
  ];
  if (Object.keys(report.context).length > 0) {
    lines.push("", "Context:", JSON.stringify(report.context, null, 2));
  }
  lines.push("", `User agent: ${report.userAgent}`, "", `Recent log (${report.recentLog.length} entries, oldest first):`);
  if (report.recentLog.length === 0) lines.push("(none)");
  for (const entry of report.recentLog) lines.push(entryText(entry));
  return lines.join("\n");
}

/** The message and stack of a value: an error's own, or the text of anything else. */
function describeValue(value: unknown): { message: string; stack: string | null } {
  const parts = errorParts(value);
  if (parts !== null) {
    return { message: parts.message === "" ? parts.name : parts.message, stack: parts.stack };
  }
  const text = textOf(value);
  return { message: text === "" ? UNKNOWN_ERROR : text, stack: null };
}

/** The text of a value that was thrown or rejected: a string as it is, anything else as JSON. */
function textOf(value: unknown): string {
  if (typeof value === "string") return value;
  const plain = toJsonValue(value);
  if (plain === undefined) return "undefined";
  return typeof plain === "string" ? plain : JSON.stringify(plain);
}

/** The value of an error event: the error itself, or the event's message when it carries no error. */
function errorValueOf(event: Event): unknown {
  const error = propertyOf(event, "error");
  if (error !== undefined && error !== null) return error;
  const message = propertyOf(event, "message");
  return typeof message === "string" && message !== "" ? message : UNKNOWN_ERROR;
}

/** A property of an event or another object, or undefined when reading it throws. */
function propertyOf(target: object, key: string): unknown {
  try {
    return Reflect.get(target, key);
  } catch {
    return undefined;
  }
}

/** The browser's user agent, or "unknown" where there is no navigator. */
function userAgentText(): string {
  return typeof navigator === "undefined" ? "unknown" : navigator.userAgent;
}

/** A report id: the time and a number, both in base 36. The number counts the reports made on this page. */
function nextReportId(now: number): string {
  lastReportNumber += 1;
  return `${now.toString(36)}-${lastReportNumber.toString(36)}`;
}

/** Runs one listener's work. The reporter must never throw into the page's error handlers, which would report again. */
function guard(action: () => void): void {
  try {
    action();
  } catch {
    // Nothing more can be done here: the reporter is the last place that an error can go.
  }
}

/** The game's text in a report: its title, id and version, or "unknown". */
function gameText(game: ReportGame | null): string {
  return game === null ? "unknown" : `${game.title} (${game.id}) ${game.version}`;
}

/** One log entry as a line: its time, its level, its logger, its message and its fields. */
function entryText(entry: LogEntry): string {
  const head = `${isoText(entry.time)} ${entry.level.toUpperCase()} ${entry.logger}: ${entry.message}`;
  return Object.keys(entry.fields).length === 0 ? head : `${head} ${JSON.stringify(entry.fields)}`;
}

/** A time as an ISO string, or the number itself when it is not a valid time. */
function isoText(time: number): string {
  const date = new Date(time);
  return Number.isNaN(date.getTime()) ? String(time) : date.toISOString();
}
