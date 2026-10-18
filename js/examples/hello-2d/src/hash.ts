// Fingerprints of the game's state. A replay compares them at each checkpoint, so two equal states must give the same
// text whatever order their objects were built in.

/**
 * Writes a value as JSON with its object keys sorted, so that two equal states give the same text.
 * @param value - the value to write.
 */
export function sortedJson(value: unknown): string {
  if (Array.isArray(value)) return `[${value.map(sortedJson).join(",")}]`;
  if (typeof value === "object" && value !== null) {
    const record = value as Record<string, unknown>;
    const parts = Object.keys(record)
      .sort()
      .map((key) => `${JSON.stringify(key)}:${sortedJson(record[key])}`);
    return `{${parts.join(",")}}`;
  }
  const text: string | undefined = JSON.stringify(value);
  return text ?? "null";
}

/**
 * A 32-bit FNV-1a fingerprint of a text, as eight hex digits.
 * @param text - the text to fingerprint.
 */
export function fingerprint(text: string): string {
  let hash = 0x811c9dc5;
  for (let index = 0; index < text.length; index += 1) {
    hash ^= text.charCodeAt(index);
    hash = Math.imul(hash, 0x01000193) >>> 0;
  }
  return hash.toString(16).padStart(8, "0");
}
