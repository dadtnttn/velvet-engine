// Argument checks for the runtime. A call with a value that the API does not accept throws a
// RangeError that names the argument, as the core package does.

/** Throws a RangeError unless `value` is a finite number from `min` to `max`, both included. */
export function checkRange(value: number, min: number, max: number, name: string): void {
  if (!Number.isFinite(value) || value < min || value > max) {
    throw new RangeError(`${name} must be a number from ${min} to ${max}, got ${value}`);
  }
}

/** Throws a RangeError unless `value` is a finite number. */
export function checkFinite(value: number, name: string): void {
  if (!Number.isFinite(value)) throw new RangeError(`${name} must be a finite number, got ${value}`);
}

/** Throws a RangeError unless `value` is a finite number that is zero or more. */
export function checkNonNegative(value: number, name: string): void {
  checkFinite(value, name);
  if (value < 0) throw new RangeError(`${name} must not be negative, got ${value}`);
}

/** Throws a RangeError unless `value` is a finite number above zero. */
export function checkPositive(value: number, name: string): void {
  checkFinite(value, name);
  if (value <= 0) throw new RangeError(`${name} must be above zero, got ${value}`);
}
