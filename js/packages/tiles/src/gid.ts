// Global tile ids as Tiled writes them: a 32-bit number whose top four bits are flip flags (horizontal,
// vertical, diagonal and hexagonal rotation). The rest is the id itself. Ids are split here, once, so
// the rest of the package only sees clean ids and the three flip bits of `types.ts`.

import { TILE_FLIP_DIAGONAL, TILE_FLIP_HORIZONTAL, TILE_FLIP_VERTICAL } from "./types";

/** The largest value a tile id can have in a file: 32 bits. */
export const GID_LIMIT = 0xffffffff;

/** The value of the lowest bit of the flag field: 2^28. Everything at or above it is a flag. */
const FLAG_UNIT = 0x10000000;

/** A global id split into its id and its flags. */
export interface SplitGid {
  /** The id, with no flags. 0 is an empty cell. */
  readonly gid: number;
  /** The `TILE_FLIP_*` bits. */
  readonly flips: number;
  /** True when the hexagonal rotation bit is set. Only hexagonal maps use it, so this reader refuses it. */
  readonly hexRotation: boolean;
}

/**
 * Splits a 32-bit tile id from a file. The top four bits are, from the highest: horizontal (8), vertical (4),
 * diagonal (2) and hexagonal rotation (1).
 * @param raw - an integer from 0 to 2^32 - 1.
 */
export function splitGid(raw: number): SplitGid {
  const flagBits = Math.floor(raw / FLAG_UNIT);
  const gid = raw - flagBits * FLAG_UNIT;
  let flips = 0;
  if (flagBits & 8) flips |= TILE_FLIP_HORIZONTAL;
  if (flagBits & 4) flips |= TILE_FLIP_VERTICAL;
  if (flagBits & 2) flips |= TILE_FLIP_DIAGONAL;
  return { gid, flips, hexRotation: (flagBits & 1) === 1 };
}
