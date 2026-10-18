// The Log tab: the game's log entries, newest first, with a level filter and a text filter. Above them are the
// error reports of the reporter, each one opening to its full text, with a button that copies it. The tab redraws
// only when the log, the reports or a filter changes, so a report that is open stays open while entries arrive.

import { LOG_LEVELS, formatReport, type ErrorReport, type LogEntry, type LogLevel, type Logger } from "@velvet/core";
import type { OverlayApi } from "./api";
import { button, element } from "./dom";
import type { Tab } from "./tabs";
import { clockText, messageOf } from "./text";

/** The most entries that the tab lists, newest first. The log may keep more; this keeps the page short. */
export const LOG_ROWS = 200;

/** The most error reports that the tab lists, most recent first. */
export const REPORT_ROWS = 5;

/** True when an entry's logger, message or fields contain the text, ignoring case. An empty text matches every entry. */
function matchesText(entry: LogEntry, needle: string): boolean {
  if (needle === "") return true;
  const haystack = `${entry.logger} ${entry.message} ${JSON.stringify(entry.fields)}`.toLowerCase();
  return haystack.includes(needle);
}

/** The Log tab. It builds its DOM once, and redraws the parts whose data changed. */
export class LogTab implements Tab {
  readonly element: HTMLElement;
  private readonly api: OverlayApi;
  private readonly doc: Document;
  private readonly reportSection: HTMLElement;
  private readonly reportStatus: HTMLElement;
  /** The reports that are open, by id, so that a redraw keeps them open. */
  private readonly openReports = new Set<string>();
  private readonly level: HTMLSelectElement;
  private readonly filter: HTMLInputElement;
  private readonly note: HTMLElement;
  private readonly summary: HTMLElement;
  private readonly list: HTMLElement;
  private reportKey: string | null = null;
  private entryKey: string | null = null;

  constructor(doc: Document, api: OverlayApi) {
    this.api = api;
    this.doc = doc;
    this.element = element(doc, "div", "vdt-page");

    this.reportSection = element(doc, "section", "vdt-reports");
    this.reportStatus = element(doc, "p", "vdt-status");

    this.level = element(doc, "select", "vdt-input");
    this.level.setAttribute("aria-label", "Lowest level shown");
    for (const choice of LOG_LEVELS) {
      const option = element(doc, "option", undefined, `${choice} and up`);
      option.value = choice;
      this.level.append(option);
    }
    this.level.addEventListener("change", () => {
      this.update();
    });

    this.filter = element(doc, "input", "vdt-input");
    this.filter.type = "search";
    this.filter.placeholder = "Filter by text, logger or field";
    this.filter.setAttribute("aria-label", "Filter the log by text");
    this.filter.addEventListener("input", () => {
      this.update();
    });
    const filters = element(doc, "div", "vdt-filters");
    filters.append(this.level, this.filter);

    this.note = element(doc, "p", "vdt-note");
    this.summary = element(doc, "p", "vdt-status");
    this.list = element(doc, "ol", "vdt-entries");
    this.element.append(this.reportSection, filters, this.note, this.summary, this.list);
  }

  /** Redraws the reports when they changed, and the entries when the log or a filter changed. */
  update(): void {
    const reports = this.api.errorReports();
    const reportKey = reports.map((report) => `${report.id}#${report.count}`).join(",");
    if (reportKey !== this.reportKey) {
      this.reportKey = reportKey;
      this.renderReports(reports);
    }
    const logger = this.api.logger();
    const entryKey = `${logger === undefined ? "none" : logger.revision}|${this.level.value}|${this.filter.value}`;
    if (entryKey !== this.entryKey) {
      this.entryKey = entryKey;
      this.renderEntries(logger);
    }
  }

  /** Builds the newest reports, each one closed to its headline unless it was open before. */
  private renderReports(reports: readonly ErrorReport[]): void {
    const doc = this.doc;
    const count = reports.length;
    const heading = element(
      doc,
      "p",
      "vdt-note",
      count > REPORT_ROWS
        ? `Error reports: ${count}, the newest ${REPORT_ROWS} shown`
        : `Error reports: ${count}`,
    );
    const items = reports.slice(0, REPORT_ROWS).map((report) => this.reportItem(report));
    this.reportSection.hidden = count === 0;
    this.reportSection.replaceChildren(heading, ...items, this.reportStatus);
  }

  /** One report: its headline, which opens to the full text, and a button that copies that text. */
  private reportItem(report: ErrorReport): HTMLElement {
    const doc = this.doc;
    const details = element(doc, "details", "vdt-report");
    details.open = this.openReports.has(report.id);
    details.addEventListener("toggle", () => {
      if (details.open) this.openReports.add(report.id);
      else this.openReports.delete(report.id);
    });
    const headline = `${clockText(report.time)} · ${report.kind} · ×${report.count} · ${report.message}`;
    const copy = button(doc, "Copy report", () => {
      this.copy(report);
    });
    details.append(
      element(doc, "summary", "vdt-report-head", headline),
      copy,
      element(doc, "pre", "vdt-report-text", formatReport(report)),
    );
    return details;
  }

  /** Lists the entries that pass both filters, newest first, under a note about the loggers and the totals. */
  private renderEntries(logger: Logger | undefined): void {
    if (logger === undefined) {
      this.note.textContent = "No logger was given. Pass logger to createDevtools to list the game's log entries.";
      this.summary.textContent = "";
      this.list.replaceChildren();
      return;
    }
    const loggers = logger.loggers().map((item) => `${item.name} ${item.level}`);
    this.note.textContent = `Loggers: ${loggers.join(" · ")}`;

    const minimum = LOG_LEVELS.indexOf(this.level.value as LogLevel);
    const needle = this.filter.value.trim().toLowerCase();
    const all = logger.entries();
    const matching = all.filter((entry) => LOG_LEVELS.indexOf(entry.level) >= minimum && matchesText(entry, needle));
    const shown = matching.slice(-LOG_ROWS).reverse();
    this.summary.textContent = `${shown.length} of ${all.length} entries shown · ${this.api.errorCount()} errors in the log`;
    this.list.replaceChildren(...shown.map((entry) => this.entryItem(entry)));
  }

  /** One log entry as a row: its time, level, logger, message, and its fields when it has any. */
  private entryItem(entry: LogEntry): HTMLElement {
    const doc = this.doc;
    const item = element(doc, "li", `vdt-entry vdt-level-${entry.level}`);
    item.append(
      element(doc, "span", "vdt-entry-time", clockText(entry.time)),
      element(doc, "span", "vdt-entry-level", entry.level.toUpperCase()),
      element(doc, "span", "vdt-entry-logger", entry.logger),
      element(doc, "span", "vdt-entry-message", entry.message),
    );
    if (Object.keys(entry.fields).length > 0) {
      item.append(element(doc, "span", "vdt-entry-fields", JSON.stringify(entry.fields)));
    }
    return item;
  }

  /** Copies a report's full text to the clipboard, and says whether that worked. */
  private copy(report: ErrorReport): void {
    const text = formatReport(report);
    const clipboard = typeof navigator === "undefined" ? undefined : navigator.clipboard;
    if (clipboard === undefined || typeof clipboard.writeText !== "function") {
      this.setStatus("The clipboard is not available here.", true);
      return;
    }
    clipboard.writeText(text).then(
      () => {
        this.setStatus(`Copied report ${report.id}.`, false);
      },
      (error: unknown) => {
        this.setStatus(`Copy failed: ${messageOf(error)}`, true);
      },
    );
  }

  /** Sets the status line above the reports, in the error colour when `isError` is true. */
  private setStatus(text: string, isError: boolean): void {
    this.reportStatus.textContent = text;
    this.reportStatus.classList.toggle("is-error", isError);
  }
}
