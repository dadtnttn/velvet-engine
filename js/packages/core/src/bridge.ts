// The native bridge: the object that Velvet Desktop injects as `window.velvet`.
//
// This file holds the contract as TypeScript types, the lookup that finds the bridge at
// runtime, and the slot-name rule that every save backend shares. The host implements the
// contract; the game only calls it.

import { isRecord } from "./guards";

/** Valid save slot names: 1 to 64 letters, digits, underscores or dashes. */
export const SLOT_PATTERN = /^[A-Za-z0-9_-]{1,64}$/;

/**
 * The contract that Velvet Desktop implements. Every method returns a promise except `log`.
 *
 * `window.capture` and `shell` came after the first hosts. They are optional here, so a host
 * without them still counts as a bridge, and the helpers below say what is missing.
 */
export interface VelvetBridge {
  platform: "desktop";
  game: { id: string; title: string; version: string };
  saves: {
    list(): Promise<string[]>;
    read(slot: string): Promise<string | null>;
    write(slot: string, text: string): Promise<void>;
    remove(slot: string): Promise<void>;
  };
  settings: { get(key: string): Promise<unknown>; set(key: string, value: unknown): Promise<void> };
  window: {
    setFullscreen(on: boolean): Promise<void>;
    isFullscreen(): Promise<boolean>;
    close(): Promise<void>;
    /**
     * A PNG of the game's page as a data URL (`data:image/png;base64,...`). A picture over the
     * host's size limit is refused with the code "too_large".
     */
    capture?(): Promise<string>;
  };
  /** Opens a link in the player's own programs: only https, http and mailto links are allowed. */
  shell?: { openExternal(url: string): Promise<void> };
  log(level: "debug" | "info" | "warn" | "error", message: string): void;
}

/** True when `slot` is a valid save slot name. */
export function isValidSlot(slot: string): boolean {
  return SLOT_PATTERN.test(slot);
}

/**
 * True when `value` has the shape of the bridge: the platform tag, the game description, and every
 * member that the contract requires. The optional members (`window.capture`, `shell`) are not
 * needed, so a host from before them still counts.
 */
export function isVelvetBridge(value: unknown): value is VelvetBridge {
  if (!isRecord(value) || value.platform !== "desktop") return false;
  return (
    hasMembers(value.game, ["id", "title", "version"], "string") &&
    hasMembers(value.saves, ["list", "read", "write", "remove"], "function") &&
    hasMembers(value.settings, ["get", "set"], "function") &&
    hasMembers(value.window, ["setFullscreen", "isFullscreen", "close"], "function") &&
    typeof value.log === "function"
  );
}

/** The object at `window.velvet` when it looks like the bridge; otherwise null. */
export function getBridge(): VelvetBridge | null {
  const candidate: unknown = Reflect.get(globalThis, "velvet");
  return isVelvetBridge(candidate) ? candidate : null;
}

/** Why a bridge helper could not make its call: no Velvet Desktop, or a host that is too old. */
export type BridgeUnavailableCode = "no-desktop" | "unsupported";

/**
 * Raised by the helpers below before the host is asked. A refusal by the host itself arrives as
 * the host sent it, with the host's own `code` (for example "invalid_url").
 */
export class BridgeUnavailableError extends Error {
  /** "no-desktop" when the page runs in a browser, "unsupported" when the host predates the call. */
  readonly code: BridgeUnavailableCode;

  /**
   * @param code - why the call could not be made.
   * @param message - a sentence for logs and developers.
   */
  constructor(code: BridgeUnavailableCode, message: string) {
    super(message);
    this.name = "BridgeUnavailableError";
    this.code = code;
  }
}

/**
 * Opens an https, http or mailto link in the player's browser or mail program. The host checks the
 * address first and refuses anything else with the code "invalid_url".
 */
export async function openExternalLink(url: string): Promise<void> {
  const bridge = getBridge();
  if (bridge === null) {
    throw new BridgeUnavailableError(
      "no-desktop",
      "openExternalLink needs Velvet Desktop: this page is running in a browser.",
    );
  }
  if (bridge.shell === undefined) {
    throw new BridgeUnavailableError(
      "unsupported",
      "This Velvet Desktop is too old to open links: update it to open them.",
    );
  }
  await bridge.shell.openExternal(url);
}

/**
 * A PNG of the game's page as a data URL, for photo modes and bug reports. A picture over the
 * host's size limit is refused with the code "too_large".
 */
export async function captureScreenshot(): Promise<string> {
  const bridge = getBridge();
  if (bridge === null) {
    throw new BridgeUnavailableError(
      "no-desktop",
      "captureScreenshot needs Velvet Desktop: this page is running in a browser.",
    );
  }
  if (bridge.window.capture === undefined) {
    throw new BridgeUnavailableError(
      "unsupported",
      "This Velvet Desktop is too old to take pictures: update it to take them.",
    );
  }
  return bridge.window.capture();
}

/** True when every name in `names` is a property of `value` whose typeof matches `kind`. */
function hasMembers(value: unknown, names: readonly string[], kind: "string" | "function"): boolean {
  if (!isRecord(value)) return false;
  const record = value;
  return names.every((name) => typeof record[name] === kind);
}
