// Helpers for the tests of hello-vn. The package build never imports this file. The tests that run the game
// in a page use happy-dom; the data files are read from the public folder, as the web server would serve them.

import { readFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { vi } from "vitest";
import type { SettingsBackend } from "@velvet/core";
import { GuionLibrary } from "@velvet/guion";
import { loadVelvetWasm } from "@velvet/wasm";
import { CAST, LANGUAGES, STORY_FILES, type Language } from "./config";

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
 * Starts the WebAssembly module from its bytes. The loader reads a file from its URL, but happy-dom replaces the
 * URL class, so the bytes are handed over directly. Later calls in the same run reuse the module.
 */
export async function loadWasmForTests(): Promise<void> {
  await loadVelvetWasm(await readFile(WASM_FILE));
}

/** Makes `fetch` read the game's files from the public folder, as the web server would. */
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

/** Settings kept in a plain object: the same shape as the bridge's settings, and nothing survives a test. */
export function memorySettings(): SettingsBackend & { readonly values: Map<string, unknown> } {
  const values = new Map<string, unknown>();
  return {
    values,
    async get(key: string): Promise<unknown> {
      return values.has(key) ? values.get(key) : null;
    },
    async set(key: string, value: unknown): Promise<void> {
      if (value === null) values.delete(key);
      else values.set(key, value);
    },
  };
}

/** Waits, in real time, until `check` is true. Fails when that takes longer than `timeoutMs`. */
export async function waitUntil(check: () => boolean, timeoutMs = 4000): Promise<void> {
  const deadline = Date.now() + timeoutMs;
  while (!check()) {
    if (Date.now() > deadline) throw new Error("timed out waiting for the page");
    await new Promise((done) => setTimeout(done, 20));
  }
}

/** The first element matching a selector. A test that asks for a missing element fails here. */
export function must<T extends Element>(root: ParentNode, selector: string): T {
  const found = root.querySelector<T>(selector);
  if (found === null) throw new Error(`no element matches ${selector}`);
  return found;
}

/** The guiones of the game, both languages added to one library, as the game adds them. */
export interface StoryFixture {
  readonly library: GuionLibrary;
  readonly fileIds: Readonly<Record<Language, string>>;
}

/** Starts WebAssembly, and makes the library of the story with both guiones added. */
export async function storyFixture(): Promise<StoryFixture> {
  await loadWasmForTests();
  const library = await GuionLibrary.create({ characters: CAST.characters });
  const fileIds: Record<Language, string> = { es: "", en: "" };
  for (const language of LANGUAGES) {
    const file = STORY_FILES[language];
    fileIds[language] = library.add(file.name, await readPublic(file.url)).id;
  }
  return { library, fileIds };
}
