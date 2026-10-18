// The native bridge: the object that Velvet Desktop injects as `window.velvet`.
//
// This file holds the contract as TypeScript types, the lookup that finds the bridge at
// runtime, and the slot-name rule that every save backend shares. The host implements the
// contract; the game only calls it.

import { isRecord } from "./guards";

/** Valid save slot names: 1 to 64 letters, digits, underscores or dashes. */
export const SLOT_PATTERN = /^[A-Za-z0-9_-]{1,64}$/;

/** The contract that Velvet Desktop implements. Every method returns a promise except `log`. */
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
  window: { setFullscreen(on: boolean): Promise<void>; isFullscreen(): Promise<boolean>; close(): Promise<void> };
  log(level: "debug" | "info" | "warn" | "error", message: string): void;
}

/** True when `slot` is a valid save slot name. */
export function isValidSlot(slot: string): boolean {
  return SLOT_PATTERN.test(slot);
}

/**
 * True when `value` has the shape of the bridge: the platform tag, the game description,
 * and every member that the contract lists.
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

/** True when every name in `names` is a property of `value` whose typeof matches `kind`. */
function hasMembers(value: unknown, names: readonly string[], kind: "string" | "function"): boolean {
  if (!isRecord(value)) return false;
  const record = value;
  return names.every((name) => typeof record[name] === kind);
}
