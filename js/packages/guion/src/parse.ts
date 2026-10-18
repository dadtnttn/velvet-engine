// parseGuion: reads one guion file, given as text, and reports what it holds.

import { loadVelvetWasm } from "@velvet/wasm";
import type { CastSpec, GuionFile } from "./types";
import { decode } from "./wire";

/** Options for `parseGuion`. */
export interface ParseOptions {
  /** The file name: problems are reported under it, and it gives the file id (default `guion.md`). */
  readonly file?: string;
  /** The game's cast. Without one, `narrador` and `narrator` are the narrator and nobody else has a name. */
  readonly cast?: CastSpec;
}

/**
 * Reads one guion and returns its summary: its id, title, room, scenes and problems. A file with
 * problems still comes back, so read `problems` to decide what to do with it.
 *
 * @param text - the Markdown of the guion.
 * @param options - the file name and the cast.
 * @returns the summary of the file.
 * @throws {GuionError} with code `bad-input` when the cast is not valid.
 *
 * @example
 * const file = await parseGuion(text, { file: "posadera.md" });
 * for (const problem of file.problems) {
 *   console.warn(`${problem.file}:${problem.line}: ${problem.message}`);
 * }
 */
export async function parseGuion(text: string, options: ParseOptions = {}): Promise<GuionFile> {
  const wasm = await loadVelvetWasm();
  const cast = JSON.stringify(options.cast ?? {});
  return decode<GuionFile>(wasm.parseGuion(text, options.file ?? "guion.md", cast));
}
