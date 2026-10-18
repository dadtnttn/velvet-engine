// Reading what velvet-wasm replies. Every reply is JSON text. A failed call replies with
// `{"error": message, "code": code}`, and a call that cannot reply that way (a constructor, or
// `load`) throws the same text as a string. Both become a CardsError here.

import { CardsError, type CardsErrorCode } from "./error";

/** Every code that the module or this package can send, so that an unknown one reads as "internal". */
const CODES: readonly CardsErrorCode[] = [
  "bad-input",
  "bad-id",
  "unsupported-document",
  "empty-card-id",
  "duplicate-card",
  "zero-max",
  "unknown-card",
  "duplicate-recipe",
  "unknown-recipe",
  "no-ingredients",
  "zero-count",
  "bad-seconds",
  "bad-chance",
  "no-such-card",
  "same-stack",
  "drop-refused",
  "stack-full",
  "bad-placement",
  "invalid-save",
  "disposed",
  "internal",
];

/** The failure object of the module. */
interface FailureReply {
  readonly error: string;
  readonly code: CardsErrorCode;
}

/** True when `value` is a failure object: a message and a known code. */
function isFailureReply(value: unknown): value is FailureReply {
  if (typeof value !== "object" || value === null) return false;
  const error: unknown = Reflect.get(value, "error");
  const code: unknown = Reflect.get(value, "code");
  return typeof error === "string" && typeof code === "string" && isCode(code);
}

/** True when `code` is one of the codes of this API. */
function isCode(code: string): code is CardsErrorCode {
  return (CODES as readonly string[]).includes(code);
}

/**
 * Reads a reply of the module: the value it carries, or a CardsError when the reply is a failure.
 * @param reply - the JSON text that the module returned.
 * @throws {CardsError} when the reply is a failure object.
 */
export function decode<T>(reply: string): T {
  const value: unknown = JSON.parse(reply);
  if (isFailureReply(value)) throw new CardsError(value.code, value.error);
  return value as T;
}

/**
 * Runs a call that may throw the module's failure text, and throws it again as a CardsError.
 * @param call - the call into the module.
 * @returns what the call returns.
 * @throws {CardsError} whatever the call throws, as a CardsError.
 */
export function attempt<T>(call: () => T): T {
  try {
    return call();
  } catch (thrown) {
    throw asCardsError(thrown);
  }
}

/** The CardsError for something the module threw: its failure text, or any other error. */
function asCardsError(thrown: unknown): CardsError {
  if (thrown instanceof CardsError) return thrown;
  if (typeof thrown === "string") {
    try {
      const value: unknown = JSON.parse(thrown);
      if (isFailureReply(value)) {
        return new CardsError(value.code, value.error, { cause: thrown });
      }
    } catch {
      // Not JSON: an error of some other kind, reported below as "internal".
    }
  }
  const message = thrown instanceof Error ? thrown.message : String(thrown);
  return new CardsError("internal", message, { cause: thrown });
}
