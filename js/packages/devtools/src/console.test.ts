// Tests for the console's parts: word splitting, typed arguments, completion, the history of lines and the
// output buffer. None of these touch the DOM.

import { describe, expect, it } from "vitest";
import {
  ConsoleHistory,
  ConsoleOutput,
  completeLine,
  parseArgs,
  tokenize,
  usageOf,
} from "./console";
import type { ArgSpec, CommandSpec } from "./types";

const noop = (): void => undefined;

describe("tokenize", () => {
  it("splits on any whitespace", () => {
    expect(tokenize("speed  2\t3")).toEqual(["speed", "2", "3"]);
  });

  it("keeps a quoted run of words together", () => {
    expect(tokenize('say "hello big world" now')).toEqual(["say", "hello big world", "now"]);
  });

  it("keeps an empty quoted word", () => {
    expect(tokenize('a "" b')).toEqual(["a", "", "b"]);
  });

  it("has no words in a blank line", () => {
    expect(tokenize("   ")).toEqual([]);
  });

  it("refuses a quote that is never closed", () => {
    expect(() => tokenize('a "b')).toThrow("missing closing quote");
  });
});

describe("parseArgs", () => {
  const specs: ArgSpec[] = [
    { name: "count", type: "number" },
    { name: "loud", type: "boolean", optional: true },
    { name: "mode", type: "string", optional: true, choices: ["a", "b"] },
  ];

  it("converts each word to the type of its argument", () => {
    expect(parseArgs(specs, ["3", "on", "b"])).toEqual({ count: 3, loud: true, mode: "b" });
    expect(parseArgs(specs, ["-1.5", "no"])).toEqual({ count: -1.5, loud: false });
  });

  it("leaves out an optional argument that was not given", () => {
    const values = parseArgs(specs, ["3"]);
    expect(values["count"]).toBe(3);
    expect(values["loud"]).toBeUndefined();
    expect(values["mode"]).toBeUndefined();
  });

  it("names the argument in each error", () => {
    expect(() => parseArgs(specs, ["x"])).toThrow('count must be a number, got "x"');
    expect(() => parseArgs(specs, [""])).toThrow("count must be a number");
    expect(() => parseArgs(specs, ["3", "maybe"])).toThrow('loud must be on or off, got "maybe"');
    expect(() => parseArgs(specs, ["3", "no", "c"])).toThrow("mode must be one of a, b, got \"c\"");
  });

  it("says which argument is missing, and that there are too many", () => {
    expect(() => parseArgs(specs, [])).toThrow("missing <count>");
    expect(() => parseArgs([], ["x"])).toThrow("this command takes no arguments");
    expect(() => parseArgs(specs, ["1", "on", "a", "x"])).toThrow("too many arguments: at most 3");
  });
});

describe("usageOf", () => {
  it("puts required arguments in angle brackets and optional ones in square brackets", () => {
    expect(usageOf("speed", [{ name: "value", type: "number", optional: true }])).toBe("speed [value:number]");
    expect(usageOf("pick", [{ name: "kind", type: "string", choices: ["a", "b"] }])).toBe("pick <kind:a|b>");
    expect(usageOf("pause", [])).toBe("pause");
  });
});

describe("completeLine", () => {
  const commands = new Map<string, CommandSpec>([
    ["speed", { run: noop }],
    ["pause", { run: noop }],
    ["clear-log", { run: noop }],
    ["clear-cache", { run: noop }],
    [
      "pick",
      { args: [{ name: "kind", type: "string", choices: ["bright", "bold"] }], run: noop },
    ],
  ]);

  it("completes a unique command name and adds a space", () => {
    expect(completeLine(commands, "spe")).toEqual({ text: "speed ", options: ["speed"] });
  });

  it("completes several names to their common start, and lists them", () => {
    expect(completeLine(commands, "cl")).toEqual({ text: "clear-", options: ["clear-cache", "clear-log"] });
  });

  it("lists every command for an empty line, and leaves the line as it is", () => {
    const result = completeLine(commands, "");
    expect(result.text).toBe("");
    expect(result.options).toHaveLength(5);
  });

  it("completes a choice argument that matches one choice", () => {
    expect(completeLine(commands, "pick bri")).toEqual({ text: "pick bright ", options: ["bright"] });
  });

  it("lists the choices that match, and completes to the common start they share", () => {
    expect(completeLine(commands, "pick b")).toEqual({ text: "pick b", options: ["bright", "bold"] });
    expect(completeLine(commands, "pick ")).toEqual({ text: "pick b", options: ["bright", "bold"] });
    expect(completeLine(commands, "pick br")).toEqual({ text: "pick bright ", options: ["bright"] });
  });

  it("does nothing for an unknown command, or for an argument with no choices", () => {
    expect(completeLine(commands, "nope x")).toEqual({ text: "nope x", options: [] });
    expect(completeLine(commands, "speed 5")).toEqual({ text: "speed 5", options: [] });
  });
});

describe("ConsoleHistory", () => {
  it("browses from the newest line, keeps the draft, and brings it back after the newest", () => {
    const history = new ConsoleHistory(10);
    history.add("a");
    history.add("b");
    history.add("c");
    expect(history.previous("dr")).toBe("c");
    expect(history.previous("")).toBe("b");
    expect(history.previous("")).toBe("a");
    expect(history.previous("")).toBe("a");
    expect(history.next()).toBe("b");
    expect(history.next()).toBe("c");
    expect(history.next()).toBe("dr");
    expect(history.next()).toBeNull();
  });

  it("does not keep a blank line, or a line that repeats the newest", () => {
    const history = new ConsoleHistory(10);
    history.add(" ");
    history.add("x");
    history.add("x");
    history.add("y");
    expect(history.size).toBe(2);
  });

  it("drops the oldest lines beyond its capacity", () => {
    const history = new ConsoleHistory(2);
    history.add("a");
    history.add("b");
    history.add("c");
    expect(history.previous("")).toBe("c");
    expect(history.previous("")).toBe("b");
    expect(history.previous("")).toBe("b");
  });

  it("has nothing to give when it is empty", () => {
    expect(new ConsoleHistory(5).previous("")).toBeNull();
  });
});

describe("ConsoleOutput", () => {
  it("keeps the newest lines, and changes its revision on every change", () => {
    const output = new ConsoleOutput(2);
    output.add("input", "> one");
    output.add("output", "two");
    output.add("error", "three");
    expect(output.lines.map((line) => line.text)).toEqual(["two", "three"]);
    expect(output.revision).toBe(3);
    output.clear();
    expect(output.lines).toEqual([]);
    expect(output.revision).toBe(4);
  });
});
