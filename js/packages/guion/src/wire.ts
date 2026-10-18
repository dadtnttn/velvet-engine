// Reading what velvet-wasm replies. Every reply is JSON text. A failed call replies with
// `{"error": message, "code": code}`, and a call that cannot reply that way (a constructor, or
// `start`) throws the same text as a string. Both become a GuionError here.

import { GuionError, type GuionErrorCode } from "./error";

/** Every code that the module can send, so that an unknown one is reported as "internal". */
const CODES: readonly GuionErrorCode[] = [
  "bad-input",
  "unknown-scene",
  "not-a-choice",
  "no-such-option",
  "unavailable",
  "internal",
];

/** The failure object of the module. */
interface FailureReply {
  readonly error: string;
  readonly code: GuionErrorCode;
}

/** True when `value` is a failure object: a message and a known code. */
function isFailureReply(value: unknown): value is FailureReply {
  if (typeof value !== "object" || value === null) return false;
  const error: unknown = Reflect.get(value, "error");
  const code: unknown = Reflect.get(value, "code");
  return typeof error === "string" && typeof code === "string" && isCode(code);
}

/** True when `code` is one of the codes of the module. */
function isCode(code: string): code is GuionErrorCode {
  return (CODES as readonly string[]).includes(code);
}

/**
 * Reads a reply of the module: the value it carries, or a GuionError when the reply is a failure.
 * @param reply - the JSON text that the module returned.
 * @throws {GuionError} when the reply is a failure object.
 */
export function decode<T>(reply: string): T {
  const value: unknown = JSON.parse(reply);
  if (isFailureReply(value)) throw new GuionError(value.code, value.error);
  return value as T;
}

/**
 * Runs a call that may throw the module's failure text, and throws it again as a GuionError.
 * @param call - the call into the module.
 * @returns what the call returns.
 * @throws {GuionError} whatever the call throws, as a GuionError.
 */
export function attempt<T>(call: () => T): T {
  try {
    return call();
  } catch (thrown) {
    throw asGuionError(thrown);
  }
}

/** The GuionError for something the module threw: its failure text, or any other error. */
function asGuionError(thrown: unknown): GuionError {
  if (thrown instanceof GuionError) return thrown;
  if (typeof thrown === "string") {
    try {
      const value: unknown = JSON.parse(thrown);
      if (isFailureReply(value)) return new GuionError(value.code, value.error, { cause: thrown });
    } catch {
      // Not JSON: an error of some other kind, reported below as "internal".
    }
  }
  const message = thrown instanceof Error ? thrown.message : String(thrown);
  return new GuionError("internal", message, { cause: thrown });
}
