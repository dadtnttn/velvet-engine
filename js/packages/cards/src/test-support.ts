// Helpers shared by the tests of this package. Not part of the package's API.

import { readFile } from "node:fs/promises";
import { expect } from "vitest";
import { CardsError, type CardsErrorCode } from "./error";

/** The shared catalog of the stacks fixtures, as text (docs/formats/fixtures/stacks/catalog.json). */
export const SHARED_CATALOG: string = await readFile(
  new URL("../../../../docs/formats/fixtures/stacks/catalog.json", import.meta.url),
  "utf8",
);

/**
 * Asserts that `pending` rejects with a CardsError whose code is `code`.
 * @param pending - the promise that should reject.
 * @param code - the code the error must carry.
 */
export async function expectRejection(pending: Promise<unknown>, code: CardsErrorCode): Promise<void> {
  const error: unknown = await pending.then(
    () => undefined,
    (caught: unknown) => caught,
  );
  expect(error).toBeInstanceOf(CardsError);
  expect(error).toMatchObject({ code });
}

/**
 * Asserts that `call` throws a CardsError whose code is `code`.
 * @param call - the call that should throw.
 * @param code - the code the error must carry.
 */
export function expectFailure(call: () => unknown, code: CardsErrorCode): void {
  try {
    call();
  } catch (error) {
    expect(error).toBeInstanceOf(CardsError);
    expect(error).toMatchObject({ code });
    return;
  }
  expect.fail(`expected a ${code} failure, but the call returned`);
}
