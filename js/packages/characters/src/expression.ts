// The formula language of derived stats. A formula is numbers, stat ids, the operators + - * /,
// parentheses, unary minus, and the functions min, max, clamp and floor. It has no eval, no strings,
// no assignment and no property access, so a formula from a data file can only compute a number from
// the values it names. The rules are in docs/formats/CHARACTER.md.
//
// The grammar, lowest precedence first (operators are left-associative):
//   sum       := product (("+" | "-") product)*
//   product   := unary (("*" | "/") unary)*
//   unary     := "-" unary | atom
//   atom      := number | name | name "(" arguments ")" | "(" sum ")"
//   arguments := sum ("," sum)*
// A number is digits with an optional fraction ("12", "0.5"). A name is a stat id: a lowercase letter,
// then lowercase letters, digits or "_".

import { CharacterError } from "./error";

/** The four binary operators. */
type BinaryOperator = "+" | "-" | "*" | "/";

/** The functions that a formula may call. */
export type FunctionName = "min" | "max" | "clamp" | "floor";

/** A parsed formula. Parsing checks the syntax, the function names, the argument counts and literal zero divisors. */
export type Expression =
  | { readonly kind: "number"; readonly value: number }
  | { readonly kind: "variable"; readonly name: string }
  | { readonly kind: "negate"; readonly operand: Expression }
  | {
      readonly kind: "binary";
      readonly operator: BinaryOperator;
      readonly left: Expression;
      readonly right: Expression;
    }
  | { readonly kind: "call"; readonly name: FunctionName; readonly args: readonly Expression[] };

/** A token of the source text, with the position it starts at (0 for the first character). */
type Token =
  | { readonly kind: "number"; readonly value: number; readonly at: number }
  | { readonly kind: "name"; readonly text: string; readonly at: number }
  | { readonly kind: "symbol"; readonly text: string; readonly at: number }
  | { readonly kind: "end"; readonly at: number };

// The patterns of the tokens. They are sticky ("y"): each one matches only at the position it is set to.
const SPACE_PATTERN = /[ \t\r\n]+/y;
const NUMBER_PATTERN = /\d+(?:\.\d+)?/y;
const NAME_PATTERN = /[a-z][a-z0-9_]*/y;
const SYMBOL_PATTERN = /[-+*/(),]/y;

/** How many arguments each function takes: at least `least`, and at most `most`. */
const ARGUMENT_COUNTS: Readonly<Record<FunctionName, { readonly least: number; readonly most: number }>> = {
  min: { least: 1, most: Number.POSITIVE_INFINITY },
  max: { least: 1, most: Number.POSITIVE_INFINITY },
  clamp: { least: 3, most: 3 },
  floor: { least: 1, most: 1 },
};

/** Reads the match of a sticky pattern at a position, or undefined when it does not match there. */
function matchAt(pattern: RegExp, source: string, position: number): string | undefined {
  pattern.lastIndex = position;
  const match = pattern.exec(source);
  return match === null ? undefined : match[0];
}

/** The error for a formula that does not read, with the position in it (counted from 1 for people). */
function syntaxError(source: string, at: number, reason: string): CharacterError {
  return new CharacterError("expression", `formula "${source}": ${reason} at position ${at + 1}`);
}

/**
 * Splits a formula into tokens.
 * @throws {CharacterError} with code "expression" on a character that no token starts with.
 */
function tokenize(source: string): Token[] {
  const tokens: Token[] = [];
  let position = 0;
  while (position < source.length) {
    if (matchAt(SPACE_PATTERN, source, position) !== undefined) {
      position = SPACE_PATTERN.lastIndex;
      continue;
    }
    const number = matchAt(NUMBER_PATTERN, source, position);
    if (number !== undefined) {
      const value = Number(number);
      if (!Number.isFinite(value)) throw syntaxError(source, position, "a number is too large");
      tokens.push({ kind: "number", value, at: position });
      position += number.length;
      continue;
    }
    const name = matchAt(NAME_PATTERN, source, position);
    if (name !== undefined) {
      tokens.push({ kind: "name", text: name, at: position });
      position += name.length;
      continue;
    }
    const symbol = matchAt(SYMBOL_PATTERN, source, position);
    if (symbol !== undefined) {
      tokens.push({ kind: "symbol", text: symbol, at: position });
      position += symbol.length;
      continue;
    }
    throw syntaxError(source, position, `the character "${source.charAt(position)}" is not allowed`);
  }
  tokens.push({ kind: "end", at: source.length });
  return tokens;
}

/** The state of the parser: the source for messages, the tokens, and where the parser is among them. */
interface ParseState {
  readonly source: string;
  readonly tokens: readonly Token[];
  position: number;
}

/** The token the parser is at. The token list ends with an "end" token that the parser never passes. */
function current(state: ParseState): Token {
  const token = state.tokens[state.position];
  if (token === undefined) throw new Error("internal: the formula parser ran past its end token");
  return token;
}

/** True when the current token is the symbol `text`. */
function atSymbol(state: ParseState, text: string): boolean {
  const token = current(state);
  return token.kind === "symbol" && token.text === text;
}

/** Describes a token for an error message. */
function describe(token: Token): string {
  switch (token.kind) {
    case "number":
      return `the number ${token.value}`;
    case "name":
      return `the name "${token.text}"`;
    case "symbol":
      return `"${token.text}"`;
    case "end":
      return "the end of the formula";
  }
}

/** Throws the error for the current token, which the grammar does not accept here. */
function unexpected(state: ParseState): never {
  const token = current(state);
  throw syntaxError(state.source, token.at, `unexpected ${describe(token)}`);
}

/** Moves past the symbol `text`, or throws when the current token is something else. */
function expectSymbol(state: ParseState, text: string): void {
  if (!atSymbol(state, text)) unexpected(state);
  state.position += 1;
}

/** True when `name` is one of the functions a formula may call. */
function isFunctionName(name: string): name is FunctionName {
  return name === "min" || name === "max" || name === "clamp" || name === "floor";
}

/** Throws an arity error unless a call to `name` gives the number of arguments that the function takes. */
function checkArity(source: string, name: FunctionName, count: number): void {
  const { least, most } = ARGUMENT_COUNTS[name];
  if (count >= least && count <= most) return;
  const wanted =
    most === Number.POSITIVE_INFINITY
      ? `at least ${least} argument${least === 1 ? "" : "s"}`
      : `exactly ${least} argument${least === 1 ? "" : "s"}`;
  throw new CharacterError("arity", `formula "${source}": ${name} takes ${wanted}, but it got ${count}`);
}

/** sum := product (("+" | "-") product)* */
function parseSum(state: ParseState): Expression {
  let left = parseProduct(state);
  while (atSymbol(state, "+") || atSymbol(state, "-")) {
    const operator: "+" | "-" = atSymbol(state, "+") ? "+" : "-";
    state.position += 1;
    const right = parseProduct(state);
    left = { kind: "binary", operator, left, right };
  }
  return left;
}

/** product := unary (("*" | "/") unary)* ; a divisor that is a literal zero is refused here. */
function parseProduct(state: ParseState): Expression {
  let left = parseUnary(state);
  while (atSymbol(state, "*") || atSymbol(state, "/")) {
    const operator: "*" | "/" = atSymbol(state, "*") ? "*" : "/";
    state.position += 1;
    const right = parseUnary(state);
    if (operator === "/" && right.kind === "number" && right.value === 0) {
      throw new CharacterError("division-by-zero", `formula "${state.source}": the divisor is a literal zero`);
    }
    left = { kind: "binary", operator, left, right };
  }
  return left;
}

/** unary := "-" unary | atom ; a minus in front of a number is folded into the number. */
function parseUnary(state: ParseState): Expression {
  if (atSymbol(state, "-")) {
    state.position += 1;
    const operand = parseUnary(state);
    if (operand.kind === "number") return { kind: "number", value: -operand.value };
    return { kind: "negate", operand };
  }
  return parseAtom(state);
}

/** atom := number | name | name "(" arguments ")" | "(" sum ")" */
function parseAtom(state: ParseState): Expression {
  const token = current(state);
  if (token.kind === "number") {
    state.position += 1;
    return { kind: "number", value: token.value };
  }
  if (token.kind === "name") {
    state.position += 1;
    if (!atSymbol(state, "(")) return { kind: "variable", name: token.text };
    // The name is checked before the arguments are read, so that an unknown function is reported as one.
    if (!isFunctionName(token.text)) {
      throw new CharacterError("unknown-function", `formula "${state.source}": "${token.text}" is not a function`);
    }
    state.position += 1;
    const args: Expression[] = [];
    if (!atSymbol(state, ")")) {
      args.push(parseSum(state));
      while (atSymbol(state, ",")) {
        state.position += 1;
        args.push(parseSum(state));
      }
    }
    expectSymbol(state, ")");
    checkArity(state.source, token.text, args.length);
    return { kind: "call", name: token.text, args };
  }
  if (atSymbol(state, "(")) {
    state.position += 1;
    const inner = parseSum(state);
    expectSymbol(state, ")");
    return inner;
  }
  return unexpected(state);
}

/**
 * Reads a formula into an expression tree.
 *
 * @param source - the formula text.
 * @returns the parsed formula.
 * @throws {CharacterError} with code "expression" when the text is not a formula (the message gives the
 *   position), "unknown-function" for a call to a name that is not a function, "arity" for a call with
 *   the wrong number of arguments, and "division-by-zero" for a divisor that is a literal zero.
 *
 * @example
 * const expression = parseExpression("strength * 2 + min(wisdom, 5)");
 */
export function parseExpression(source: string): Expression {
  const state: ParseState = { source, tokens: tokenize(source), position: 0 };
  if (current(state).kind === "end") throw new CharacterError("expression", `formula "${source}" is empty`);
  const expression = parseSum(state);
  if (current(state).kind !== "end") unexpected(state);
  return expression;
}

/** Returns the value when it is a finite number, and throws the "non-finite" error otherwise. */
function finite(value: number): number {
  if (!Number.isFinite(value)) {
    throw new CharacterError("non-finite", "a formula produced a value that is infinite or not a number");
  }
  return value;
}

/** Applies one of the four operators. A zero divisor is an error, not an infinite value. */
function applyOperator(operator: BinaryOperator, left: number, right: number): number {
  switch (operator) {
    case "+":
      return finite(left + right);
    case "-":
      return finite(left - right);
    case "*":
      return finite(left * right);
    case "/":
      if (right === 0) throw new CharacterError("division-by-zero", "a formula divides by zero");
      return finite(left / right);
  }
}

/** Applies one of the functions to its argument values. The argument counts were checked when the formula was read. */
function applyFunction(name: FunctionName, values: readonly number[]): number {
  switch (name) {
    case "min":
      return Math.min(...values);
    case "max":
      return Math.max(...values);
    case "clamp": {
      const [value = 0, low = 0, high = 0] = values;
      return Math.min(Math.max(value, low), high);
    }
    case "floor":
      return Math.floor(values[0] ?? 0);
  }
}

/**
 * Computes a formula's value.
 *
 * @param expression - the parsed formula.
 * @param lookup - the value of each variable (a stat id), or undefined when there is none.
 * @returns the value of the formula.
 * @throws {CharacterError} with code "unknown-stat" when `lookup` gives no value for a variable,
 *   "division-by-zero" when a divisor is zero, and "non-finite" when a result is infinite or not a number.
 *
 * @example
 * evaluateExpression(parseExpression("(charm + wisdom) / 2"), (name) => ({ charm: 9, wisdom: 5 })[name]);
 */
export function evaluateExpression(
  expression: Expression,
  lookup: (name: string) => number | undefined,
): number {
  switch (expression.kind) {
    case "number":
      return expression.value;
    case "variable": {
      const value = lookup(expression.name);
      if (value === undefined) throw new CharacterError("unknown-stat", `no value for "${expression.name}"`);
      return value;
    }
    case "negate":
      return -evaluateExpression(expression.operand, lookup);
    case "binary": {
      const left = evaluateExpression(expression.left, lookup);
      const right = evaluateExpression(expression.right, lookup);
      return applyOperator(expression.operator, left, right);
    }
    case "call": {
      const values = expression.args.map((arg) => evaluateExpression(arg, lookup));
      return finite(applyFunction(expression.name, values));
    }
  }
}

/**
 * The variables that a formula reads, each once, in the order they first appear.
 *
 * @param expression - the parsed formula.
 * @returns the names of the variables (stat ids).
 */
export function expressionVariables(expression: Expression): string[] {
  const names: string[] = [];
  const visit = (node: Expression): void => {
    switch (node.kind) {
      case "number":
        return;
      case "variable":
        if (!names.includes(node.name)) names.push(node.name);
        return;
      case "negate":
        visit(node.operand);
        return;
      case "binary":
        visit(node.left);
        visit(node.right);
        return;
      case "call":
        node.args.forEach(visit);
        return;
    }
  };
  visit(expression);
  return names;
}
