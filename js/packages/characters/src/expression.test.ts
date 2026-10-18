// Tests of the formula language beyond the shared fixtures: the shape of a parsed formula, the
// variables it reads, the errors of evaluation, and the refusal of anything that is not arithmetic.

import { describe, expect, it } from "vitest";
import { evaluateExpression, expressionVariables, parseExpression } from "./expression";
import { expectCode } from "./test-support";

describe("parseExpression", () => {
  it("builds the tree with the usual precedence and left associativity", () => {
    expect(parseExpression("1 + 2 * 3")).toEqual({
      kind: "binary",
      operator: "+",
      left: { kind: "number", value: 1 },
      right: {
        kind: "binary",
        operator: "*",
        left: { kind: "number", value: 2 },
        right: { kind: "number", value: 3 },
      },
    });
    expect(parseExpression("8 - 3 - 2")).toEqual({
      kind: "binary",
      operator: "-",
      left: { kind: "binary", operator: "-", left: { kind: "number", value: 8 }, right: { kind: "number", value: 3 } },
      right: { kind: "number", value: 2 },
    });
  });

  it("folds a minus in front of a number into the number", () => {
    expect(parseExpression("-4")).toEqual({ kind: "number", value: -4 });
    expect(parseExpression("-strength")).toEqual({ kind: "negate", operand: { kind: "variable", name: "strength" } });
  });

  it("reads a function call with its arguments", () => {
    expect(parseExpression("clamp(x, 0, 10)")).toEqual({
      kind: "call",
      name: "clamp",
      args: [
        { kind: "variable", name: "x" },
        { kind: "number", value: 0 },
        { kind: "number", value: 10 },
      ],
    });
  });

  it("refuses an empty formula, and a formula with a stray character", () => {
    expectCode(() => parseExpression("   "), "expression");
    expectCode(() => parseExpression("a % b"), "expression");
    expectCode(() => parseExpression("a.b"), "expression");
  });

  it("refuses a number too large to be a finite value", () => {
    expectCode(() => parseExpression(`1${"0".repeat(400)}`), "expression");
  });

  it("refuses a call to a JavaScript name, such as one that would reach the runtime", () => {
    expectCode(() => parseExpression("process.exit(1)"), "expression");
    expectCode(() => parseExpression("Math.max(1, 2)"), "expression");
  });

  it("refuses a call to a stat name, because stats are values and not functions", () => {
    expectCode(() => parseExpression("strength(2)"), "unknown-function");
  });

  it("refuses a divisor that is a literal zero, however it is written", () => {
    expectCode(() => parseExpression("1 / 0"), "division-by-zero");
    expectCode(() => parseExpression("1 / -0"), "division-by-zero");
    expectCode(() => parseExpression("1 / (0)"), "division-by-zero");
  });
});

describe("expressionVariables", () => {
  it("lists each variable once, in the order that it first appears", () => {
    const expression = parseExpression("wisdom + strength * wisdom - min(charm, strength)");
    expect(expressionVariables(expression)).toEqual(["wisdom", "strength", "charm"]);
  });

  it("lists nothing for a formula of numbers", () => {
    expect(expressionVariables(parseExpression("2 * (3 + 4)"))).toEqual([]);
  });
});

describe("evaluateExpression", () => {
  it("fails with unknown-stat when the lookup has no value for a variable", () => {
    expectCode(() => evaluateExpression(parseExpression("ghost + 1"), () => undefined), "unknown-stat");
  });

  it("does not read a value from the object prototype", () => {
    const values = new Map<string, number>([["strength", 2]]);
    expectCode(() => evaluateExpression(parseExpression("constructor"), (name) => values.get(name)), "unknown-stat");
  });

  it("fails with division-by-zero for a divisor that is zero when it is evaluated", () => {
    const values = new Map<string, number>([["wisdom", 6]]);
    const expression = parseExpression("strength / (wisdom - 6)");
    expectCode(() => evaluateExpression(expression, (name) => values.get(name) ?? 4), "division-by-zero");
  });

  it("fails with non-finite when a result overflows", () => {
    const big = `1${"0".repeat(200)}`;
    expectCode(() => evaluateExpression(parseExpression(`${big} * ${big}`), () => 0), "non-finite");
  });

  it("gives the same value for the same formula and values", () => {
    const expression = parseExpression("max(strength * 2, 10) - floor(wisdom / 4)");
    const values = new Map<string, number>([
      ["strength", 3],
      ["wisdom", 9],
    ]);
    const lookup = (name: string): number | undefined => values.get(name);
    expect(evaluateExpression(expression, lookup)).toBe(10 - 2);
    expect(evaluateExpression(expression, lookup)).toBe(10 - 2);
  });
});
