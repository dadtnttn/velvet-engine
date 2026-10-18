// FNV-1a, the 32-bit hash that picks the colour of a placeholder. It works on the UTF-8 bytes of a
// string, so every runtime that follows docs/formats/ART_MANIFEST.md gets the same number.

/** The FNV-1a offset basis for 32 bits. */
const OFFSET_BASIS = 0x811c9dc5;

/** The FNV-1a prime for 32 bits. */
const PRIME = 0x01000193;

const ENCODER = new TextEncoder();

/**
 * FNV-1a over the UTF-8 bytes of a string.
 * @param text - the string to hash.
 * @returns an unsigned 32-bit integer.
 */
export function fnv1a32(text: string): number {
  let hash = OFFSET_BASIS;
  for (const byte of ENCODER.encode(text)) {
    hash ^= byte;
    hash = Math.imul(hash, PRIME) >>> 0;
  }
  return hash >>> 0;
}
