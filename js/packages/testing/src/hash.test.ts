import { describe, expect, it } from "vitest";
import { TestingError } from "./errors";
import { hashState, hashText, stableStringify } from "./hash";

describe("stableStringify", () => {
  it("sorts object keys, and keeps the order of arrays", () => {
    expect(stableStringify({ b: 1, a: { d: 2, c: [3, 1] } })).toBe('{"a":{"c":[3,1],"d":2},"b":1}');
  });

  it("rounds numbers to six digits by default, and to the chosen digits", () => {
    expect(stableStringify(1.23456789)).toBe("1.234568");
    expect(stableStringify(1.23456789, { decimals: 2 })).toBe("1.23");
    expect(stableStringify(-0)).toBe("0");
  });

  it("writes NaN and the infinities as strings, since JSON has no such numbers", () => {
    expect(stableStringify([Number.NaN, Infinity, -Infinity])).toBe('["NaN","Infinity","-Infinity"]');
  });

  it("leaves out undefined, functions and symbols in objects, and writes them as null in arrays", () => {
    const value = { keep: 1, gone: undefined, fn: () => 1, sym: Symbol("x") };
    expect(stableStringify(value)).toBe('{"keep":1}');
    expect(stableStringify([undefined, () => 1])).toBe("[null,null]");
  });

  it("writes a bigint with an n, a Map as an object, a Set as a sorted array, and a Date as ISO text", () => {
    expect(stableStringify(12n)).toBe('"12n"');
    expect(stableStringify(new Map([["z", 1], ["a", 2]]))).toBe('{"a":2,"z":1}');
    expect(stableStringify(new Set(["b", "a"]))).toBe('["a","b"]');
    expect(stableStringify(new Date(0))).toBe('"1970-01-01T00:00:00.000Z"');
  });

  it("writes a typed array as an array", () => {
    expect(stableStringify(new Uint8Array([3, 4]))).toBe("[3,4]");
  });

  it("uses a value's own toJSON", () => {
    expect(stableStringify({ inner: { toJSON: () => "text" } })).toBe('{"inner":"text"}');
  });

  it("indents when asked, and keeps one line by default", () => {
    expect(stableStringify({ a: [1] }, { indent: 2 })).toBe('{\n  "a": [\n    1\n  ]\n}');
    expect(stableStringify({ a: [1] })).toBe('{"a":[1]}');
  });

  it("refuses a value that refers to itself", () => {
    const cyclic: Record<string, unknown> = { name: "loop" };
    cyclic.self = cyclic;
    expect(() => stableStringify(cyclic)).toThrow(TestingError);
    expect(() => stableStringify(cyclic)).toThrow(/refers to itself, at \$\.self/);
  });

  it("refuses a bad number of decimals or indent", () => {
    expect(() => stableStringify(1, { decimals: 16 })).toThrow(/decimals/);
    expect(() => stableStringify(1, { indent: -1 })).toThrow(/indent/);
  });
});

describe("hashState", () => {
  it("gives the same hash for equal states built in a different order", () => {
    const first = { coins: 4, pointer: { x: 1, y: 2 } };
    const second = { pointer: { y: 2, x: 1 }, coins: 4 };
    expect(hashState(first)).toBe(hashState(second));
  });

  it("tells states apart, and writes sixteen hex digits", () => {
    expect(hashState({ coins: 4 })).not.toBe(hashState({ coins: 5 }));
    expect(hashState({ coins: 4 })).toMatch(/^[0-9a-f]{16}$/);
  });

  it("hashes text the same way every time", () => {
    expect(hashText("abc")).toBe(hashText("abc"));
    expect(hashText("")).toMatch(/^[0-9a-f]{16}$/);
    expect(hashText("abc")).not.toBe(hashText("abd"));
  });
});
