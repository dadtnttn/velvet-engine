// Helpers for the tests that run the game's page parts in happy-dom. The package build never imports
// this file, and the tests that use it are the only ones that need vitest's mocks.
//
// happy-dom has no layout. `stubLayout` gives every element of the stage the rectangle that the
// browser would draw for it, from the inline positions that the view writes, so the pointer maths
// runs as it would on screen. The stage is at the window's origin and 1920 by 1080.

import { readFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { vi } from "vitest";
import { loadVelvetWasm } from "@velvet/wasm";
import { CARD, TABLE, TABLE_ORIGIN } from "./layout";

const HERE = dirname(fileURLToPath(import.meta.url));

/** The public folder of the game: its data files are served from here. */
export const PUBLIC_DIR = resolve(HERE, "..", "public");

/** The WebAssembly module, as the wasm package builds it. */
const WASM_FILE = resolve(HERE, "..", "..", "..", "packages", "wasm", "gen", "velvet_wasm_bg.wasm");

/** A file of the public folder, as text. */
export function readPublic(path: string): Promise<string> {
  return readFile(resolve(PUBLIC_DIR, path), "utf8");
}

/**
 * Starts the WebAssembly module from its bytes. The loader reads a file from its URL, but happy-dom
 * replaces the URL class, so the bytes are handed over directly.
 */
export async function loadWasmForTests(): Promise<void> {
  await loadVelvetWasm(await readFile(WASM_FILE));
}

/** Makes `fetch` read the game's data files from the public folder, as the web server would. */
export function fetchFromPublicFolder(): void {
  vi.stubGlobal("fetch", async (url: string) => {
    try {
      const text = await readFile(resolve(PUBLIC_DIR, url), "utf8");
      return { ok: true, status: 200, text: async () => text };
    } catch {
      return { ok: false, status: 404, text: async () => "" };
    }
  });
}

/** The rectangle the browser would draw for an element of the stage, in client pixels. */
function rectFor(element: HTMLElement): DOMRect {
  const style = element.style;
  const at = (value: string): number => {
    const number = Number.parseFloat(value);
    return Number.isFinite(number) ? number : 0;
  };
  let box = { left: 0, top: 0, width: 0, height: 0 };
  if (element.classList.contains("vu-viewport") || element.classList.contains("vu-root")) {
    box = { left: 0, top: 0, width: 1920, height: 1080 };
  } else if (element.classList.contains("hc-table")) {
    box = { left: TABLE_ORIGIN.left, top: TABLE_ORIGIN.top, width: TABLE.width, height: TABLE.height };
  } else if (element.classList.contains("hc-card")) {
    box = {
      left: TABLE_ORIGIN.left + at(style.left),
      top: TABLE_ORIGIN.top + at(style.top),
      width: CARD.width,
      height: CARD.height,
    };
  }
  return { ...box, right: box.left + box.width, bottom: box.top + box.height, x: box.left, y: box.top } as DOMRect;
}

/** Makes every element of the stage report the rectangle the browser would draw for it. Undo with `vi.restoreAllMocks`. */
export function stubLayout(): void {
  vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockImplementation(function (this: HTMLElement) {
    return rectFor(this);
  });
}

/** The client centre of an element. */
export function centreOf(element: Element): { x: number; y: number } {
  const rect = element.getBoundingClientRect();
  return { x: rect.left + rect.width / 2, y: rect.top + rect.height / 2 };
}

/** Dispatches a pointer event of the given type from the primary pointer, at a client position. */
export function firePointer(target: EventTarget, type: string, x: number, y: number): void {
  const event = new MouseEvent(type, { bubbles: true, cancelable: true, clientX: x, clientY: y, button: 0 });
  Object.defineProperty(event, "pointerId", { value: 1 });
  target.dispatchEvent(event);
}

/** A key press as a keyboard sends it: keydown, then keyup, on the window. */
export function pressKey(key: string, code: string): void {
  window.dispatchEvent(new KeyboardEvent("keydown", { key, code, bubbles: true }));
  window.dispatchEvent(new KeyboardEvent("keyup", { key, code, bubbles: true }));
}

/** The first element matching a selector. A test that asks for a missing element fails here. */
export function must<T extends Element>(root: ParentNode, selector: string): T {
  const found = root.querySelector<T>(selector);
  if (found === null) throw new Error(`no element matches ${selector}`);
  return found;
}

/** Waits, in real time, until `check` is true. Fails when that takes longer than `timeoutMs`. */
export async function waitUntil(check: () => boolean, timeoutMs = 4000): Promise<void> {
  const deadline = Date.now() + timeoutMs;
  while (!check()) {
    if (Date.now() > deadline) throw new Error("timed out waiting for the page");
    await new Promise((done) => setTimeout(done, 25));
  }
}
