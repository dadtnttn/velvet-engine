// Tests for templates: parsing the holes and braces, and filling the holes.

import { describe, expect, it } from "vitest";
import { argumentValue, describeHoles, isHoleName, parseTemplate, renderTemplate, sameHoles, type Template } from "./template";

/** Parses a template that must be valid, for the tests that are about its parts. */
function parse(source: string): Template {
  const parsed = parseTemplate(source);
  if (!parsed.ok) {
    throw new Error(`expected a valid template: ${parsed.message}`);
  }
  return parsed.template;
}

describe("parseTemplate", () => {
  it("keeps plain text as one text part, with no holes", () => {
    const template = parse("Hello, world");
    expect(template.parts).toEqual([{ kind: "text", text: "Hello, world" }]);
    expect(template.holes.size).toBe(0);
  });

  it("finds positional and named holes, each name once", () => {
    const template = parse("{0} of {gold}, again {0}");
    expect([...template.holes].sort()).toEqual(["0", "gold"]);
    expect(template.parts.filter((part) => part.kind === "hole")).toHaveLength(3);
  });

  it("writes a doubled brace as a single literal brace", () => {
    const template = parse("{{0}} and {{name}}");
    expect(template.parts).toEqual([{ kind: "text", text: "{0} and {name}" }]);
    expect(template.holes.size).toBe(0);
  });

  it("reads a doubled closing brace as a literal one", () => {
    expect(parse("a }} b").parts).toEqual([{ kind: "text", text: "a } b" }]);
    expect(parse("{0}}}").parts).toEqual([
      { kind: "hole", name: "0" },
      { kind: "text", text: "}" },
    ]);
  });

  it("refuses syntax errors, each with a message, and never throws", () => {
    for (const source of ["a {0", "b }", "c {}", "d { 0 }", "e {01}", "f {a-b}", "g {-1}", "h {+1}", "{0}}", "{a{b}"]) {
      const parsed = parseTemplate(source);
      expect(parsed.ok, source).toBe(false);
      if (!parsed.ok) {
        expect(parsed.message.length, source).toBeGreaterThan(0);
      }
    }
  });
});

describe("isHoleName", () => {
  it("accepts canonical positions and names", () => {
    for (const name of ["0", "12", "gold", "_x", "item_2"]) {
      expect(isHoleName(name)).toBe(true);
    }
  });

  it("refuses leading zeros, signs, spaces, hyphens and names that start with a digit", () => {
    for (const name of ["", "01", "-1", "+1", " 0", "a-b", "1a"]) {
      expect(isHoleName(name)).toBe(false);
    }
  });
});

describe("argumentValue", () => {
  it("gives a list's items to the positional holes", () => {
    expect(argumentValue(["Ana", 3], "0")).toBe("Ana");
    expect(argumentValue(["Ana", 3], "1")).toBe(3);
    expect(argumentValue(["Ana"], "1")).toBeUndefined();
    expect(argumentValue(["Ana"], "gold")).toBeUndefined();
  });

  it("gives an object's keys to the holes of the same names, and never inherited properties", () => {
    expect(argumentValue({ gold: 5, 0: "first" }, "gold")).toBe(5);
    expect(argumentValue({ 0: "first" }, "0")).toBe("first");
    expect(argumentValue({ gold: 5 }, "toString")).toBeUndefined();
  });

  it("has no value without arguments", () => {
    expect(argumentValue(undefined, "0")).toBeUndefined();
  });
});

describe("renderTemplate", () => {
  it("fills each hole with its value, numbers included", () => {
    const template = parse("{0} has {count} coins, {count} more");
    const rendered = renderTemplate(template, (name) => argumentValue(["Ana", 5], name) ?? (name === "count" ? 2 : undefined));
    expect(rendered).toEqual({ text: "Ana has 2 coins, 2 more", missing: [] });
  });

  it("writes a hole with no value as it is written, and lists it once", () => {
    const rendered = renderTemplate(parse("{a} and {a} and {b}"), () => undefined);
    expect(rendered).toEqual({ text: "{a} and {a} and {b}", missing: ["a", "b"] });
  });

  it("does not parse the braces of an argument's value", () => {
    const rendered = renderTemplate(parse("<{0}>"), () => "{1}");
    expect(rendered.text).toBe("<{1}>");
  });
});

describe("sameHoles and describeHoles", () => {
  it("compares hole sets by their names, not their order", () => {
    expect(sameHoles(new Set(["0", "1"]), new Set(["1", "0"]))).toBe(true);
    expect(sameHoles(new Set(["0"]), new Set(["0", "x"]))).toBe(false);
    expect(sameHoles(new Set(["gold"]), new Set(["0"]))).toBe(false);
  });

  it("describes holes as a sorted list", () => {
    expect(describeHoles(new Set(["gold", "0"]))).toBe("{0, gold}");
    expect(describeHoles(new Set())).toBe("{}");
  });
});
