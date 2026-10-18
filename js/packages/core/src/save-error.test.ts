// Tests for the save error type: its code, its message and its cause.

import { describe, expect, it } from "vitest";
import { SaveError } from "./save-error";

describe("SaveError", () => {
  it("carries its code and message, and is an Error", () => {
    const error = new SaveError("newer-version", "written by a newer build");
    expect(error).toBeInstanceOf(Error);
    expect(error).toBeInstanceOf(SaveError);
    expect(error.code).toBe("newer-version");
    expect(error.message).toBe("written by a newer build");
    expect(error.name).toBe("SaveError");
  });

  it("keeps the cause when one is given", () => {
    const cause = new SyntaxError("Unexpected token");
    const error = new SaveError("corrupt", "not JSON", { cause });
    expect(error.cause).toBe(cause);
  });
});
