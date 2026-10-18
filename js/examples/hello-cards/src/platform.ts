// Platform access: what the game asks of Velvet Desktop (logs, settings, fullscreen, quit), with a
// browser fallback for each call, so the same build runs in a plain page during development.

import { createSettingsStore, getBridge, type SettingsStore } from "@velvet/core";
import { GAME_ID, type Language } from "./config";

/** The levels that the bridge's `log` takes. */
export type LogLevel = "debug" | "info" | "warn" | "error";

/** What the game keeps between runs. A saved value replaces a default only when it has the right type. */
export interface GameSettings {
  /** The language of the texts, or null when none was chosen yet (the game then starts in Spanish). */
  readonly language: Language | null;
}

/** A message for any thrown value. */
function describe(value: unknown): string {
  if (value instanceof Error) return value.message;
  return typeof value === "string" ? value : String(value);
}

/**
 * Writes one line to the game's log. In Velvet Desktop the line goes to `velvet-desktop.log` (and to
 * the terminal in a debug build); in a browser it goes to the console.
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
 * Sends every uncaught error and rejected promise to the log. Installs `window.onerror`, which
 * returns false, so the browser still reports the error in its own console.
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

/**
 * The store of the game's settings: the bridge's settings in Velvet Desktop, localStorage in a browser, and
 * memory when neither is there. Call `load` before reading them.
 */
export function createGameSettings(): SettingsStore<GameSettings> {
  return createSettingsStore<GameSettings>({ namespace: GAME_ID, version: 1, defaults: { language: null } });
}

/** True while the window is full screen. */
export async function isFullscreen(): Promise<boolean> {
  const bridge = getBridge();
  if (bridge !== null) return bridge.window.isFullscreen();
  return document.fullscreenElement !== null;
}

/**
 * Turns full screen on or off. A browser refuses the request without a user gesture, and then the
 * promise rejects; the caller decides what to show.
 */
export async function setFullscreen(on: boolean): Promise<void> {
  const bridge = getBridge();
  if (bridge !== null) {
    await bridge.window.setFullscreen(on);
    return;
  }
  if (on) await document.documentElement.requestFullscreen();
  else await document.exitFullscreen();
}

/**
 * Closes the game. In Velvet Desktop the program ends. In a browser a page cannot close itself, so
 * the function returns false and the caller says so.
 */
export async function quitApp(): Promise<boolean> {
  const bridge = getBridge();
  if (bridge === null) return false;
  await bridge.window.close();
  return true;
}
