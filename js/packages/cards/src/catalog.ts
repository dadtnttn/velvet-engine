// loadCatalog: reads a catalog document through velvet-stacks, which checks every rule.

import { loadVelvetWasm } from "@velvet/wasm";
import type { Catalog } from "./types";
import { decode } from "./wire";

/**
 * Reads and checks a catalog document (the format of docs/formats/STACKS.md, `kind: "catalog"`).
 * The engine checks every rule (unique ids, known cards in selectors and outputs, counts and
 * chances in range, a duration that is a number) and returns the catalog in its normal form: every
 * default written out, and the recipes in precedence order. Give the result to `Board.create`.
 *
 * @param json - the text of the catalog file.
 * @returns the checked catalog.
 * @throws {CardsError} with code `bad-input` when the text is not a catalog of the expected shape,
 *   `unsupported-document` when it is not a velvet-stacks catalog of version 1, and the code of the
 *   rule that broke otherwise (`duplicate-card`, `unknown-card`, `zero-count`, and so on).
 *
 * @example
 * const catalog = await loadCatalog(await readText("catalog.json"));
 */
export async function loadCatalog(json: string): Promise<Catalog> {
  const wasm = await loadVelvetWasm();
  return decode<Catalog>(wasm.parseCatalog(json));
}
