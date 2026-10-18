// Runtime type guards for values that come from outside the program: parsed JSON in
// save slots, and objects injected by the Velvet Desktop host.

/** True for an object that is neither null nor an array. */
export function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

/** True for a whole number from 1 upwards, the shape of a save version. */
export function isPositiveInteger(value: unknown): value is number {
  return typeof value === "number" && Number.isInteger(value) && value >= 1;
}
