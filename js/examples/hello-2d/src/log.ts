// Logging and error reports of hello-2d. The game logs with the core's logger, to the console and to the Velvet
// Desktop log. An uncaught error becomes a report: the report is logged, and the first report of a run shows the
// error overlay of @velvet/ui, with the report's text ready to copy.

import {
  createBridgeSink,
  createConsoleSink,
  createLogger,
  formatReport,
  installErrorReporter,
  type ErrorReport,
  type ErrorReporter,
  type Logger,
} from "@velvet/core";
import { createErrorOverlay, type ErrorOverlay } from "@velvet/ui";
import { GAME_ID } from "./config";
import type { TranslateFn } from "./view";

/** The game's own logger: the console in a browser, and the Velvet Desktop log where the host has one. */
export function createGameLogger(): Logger {
  return createLogger({
    name: GAME_ID,
    level: "info",
    sinks: [createConsoleSink(), createBridgeSink()],
  });
}

/**
 * Installs the error reporter: every uncaught error is logged, and a new report shows the error overlay over the page.
 * The overlay closes with its Continue button, and a later report replaces it.
 * @param logger - the logger that the reporter writes to.
 * @param host - the element that the overlay is added to. Default: the page's body.
 * @param translate - the game's text lookup, once the game has loaded its texts. Before that, or without it, the
 * overlay shows its English labels.
 * @returns the reporter; its `uninstall` stops it.
 */
export function reportErrors(
  logger: Logger,
  host: HTMLElement = document.body,
  translate: () => TranslateFn | undefined = () => undefined,
): ErrorReporter {
  let overlay: ErrorOverlay | null = null;
  return installErrorReporter({
    logger,
    game: { id: GAME_ID, title: "Hello 2D", version: "0.1.0" },
    onReport: (report: ErrorReport) => {
      overlay?.destroy();
      overlay = createErrorOverlay({
        host,
        report: { id: report.id, message: report.message, text: formatReport(report) },
        t: translate(),
        onContinue: () => {
          overlay?.close();
        },
        onReload: () => {
          window.location.reload();
        },
      });
    },
  });
}
