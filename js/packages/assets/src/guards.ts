// Runtime checks for values that come from outside the program: the parsed JSON of a manifest.

/** True for an object that is neither null nor an array. */
export function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
