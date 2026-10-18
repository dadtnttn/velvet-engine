// Platform access: the game's log lines and its uncaught errors. Each call has a browser fallback, so the
// same build runs in a plain page during development.

import { getBridge } from "@velvet/core";

/** The levels that the bridge's `log` takes. */
export type LogLevel = "debug" | "info" | "warn" | "error";

/** A message for any thrown value. */
function describe(value: unknown): string {
  if (value instanceof Error) return value.message;
  return typeof value === "string" ? value : String(value);
}

/**
 * Writes one line to the game's log. In Velvet Desktop the line goes to `velvet-desktop.log` (and to the
 * terminal in a debug build); in a browser it goes to the console.
 */
export function logLine(level: LogLevel, message: string): void {
  const bridge = getBridge();
  if (bridge !== null) {
    bridge.log(level, message);
    return;
  }
  if (level === "error") console.error(message);
  else if (level === "warn") console.warn(message);
  else console.log(message);
}

/**
 * Sends every uncaught error and rejected promise to the log. The window's own handler returns false, so the
 * browser still reports the error in its console.
 */
export function installErrorLog(): void {
  window.onerror = (message: string | Event, source?: string, line?: number, column?: number, error?: Error): boolean => {
    const where = source === undefined ? "" : ` at ${source}:${line ?? 0}:${column ?? 0}`;
    logLine("error", `window.onerror: ${describe(error ?? message)}${where}`);
    return false;
  };
  window.addEventListener("unhandledrejection", (event) => {
    logLine("error", `unhandled rejection: ${describe(event.reason)}`);
  });
}

/** A readable line for any thrown value, for the log. */
export function describeError(error: unknown): string {
  return describe(error);
}
