// Portraits: the faces of the dialogue box, read from a retratos.json.

import { loadVelvetWasm, type GuionPortraits as NativePortraits } from "@velvet/wasm";
import { attempt, decode } from "./wire";

/** A face of the dialogue box. */
export interface Portrait {
  /** The image, as an asset key without its extension (`retratos/ana`). */
  readonly image: string;
  /** The square of the image that holds the head and shoulders: `[x, y, width, height]` in its pixels. */
  readonly crop: readonly [number, number, number, number];
}

/**
 * The portraits of a game: for each character, one face per mood (`normal` is the default). Make one
 * with `Portraits.create`, from the text of the game's `retratos.json`.
 *
 * @example
 * const faces = await Portraits.create(await readText("retratos.json"));
 * const face = faces.find("greta", shown.mood ?? undefined);
 */
export class Portraits {
  readonly #native: NativePortraits;

  /** @param native - the module's portraits object. Use `Portraits.create` to make one. */
  private constructor(native: NativePortraits) {
    this.#native = native;
  }

  /**
   * Reads the portraits listed in `json`.
   *
   * @param json - the text of a `retratos.json`.
   * @returns the portraits.
   * @throws {GuionError} with code `bad-input` when the text is not a valid listing.
   */
  static async create(json: string): Promise<Portraits> {
    const wasm = await loadVelvetWasm();
    return new Portraits(attempt(() => new wasm.GuionPortraits(json)));
  }

  /**
   * The face of `who` looking `mood`, or their normal face when that mood is not listed. A thought
   * shows its thinker's face.
   *
   * @param who - the character id, as a `Shown` line gives it.
   * @param mood - how they look, as a `Shown` line gives it (null or undefined for the normal face).
   * @returns the face, or null when `who` has none.
   */
  find(who: string, mood?: string | null): Portrait | null {
    return decode<Portrait | null>(this.#native.find(who, mood ?? undefined));
  }

  /** Every face listed, as `[who, mood]` pairs. */
  entries(): Array<readonly [string, string]> {
    return decode<Array<readonly [string, string]>>(this.#native.entries());
  }
}
