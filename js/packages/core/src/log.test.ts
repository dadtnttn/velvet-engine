// Tests for the logger: the levels and their filtering, child loggers that share one buffer, the ring buffer and
// its revision, the fields as JSON, the console and bridge sinks, and the checks on names, levels and capacity.

import { afterEach, describe, expect, it, vi } from "vitest";
import {
  LOG_LEVELS,
  createBridgeSink,
  createConsoleSink,
  createLogger,
  isLogLevel,
  type LogEntry,
  type LogFields,
  type LogSink,
  type LogTarget,
} from "./log";
import { createFakeBridge } from "./test-support";

afterEach(() => {
  vi.unstubAllGlobals();
  vi.useRealTimers();
});

/** A sink that keeps the entries it is given, in arrival order. */
function recordingSink(): { sink: LogSink; entries: LogEntry[] } {
  const entries: LogEntry[] = [];
  return {
    sink: {
      write(entry: LogEntry): void {
        entries.push(entry);
      },
    },
    entries,
  };
}

/** A root logger named "game" that writes nowhere but to the list it returns. */
function quietLogger(options: { level?: "debug" | "info" | "warn" | "error"; capacity?: number } = {}): {
  log: ReturnType<typeof createLogger>;
  entries: LogEntry[];
} {
  const { sink, entries } = recordingSink();
  return { log: createLogger({ name: "game", sinks: [sink], ...options }), entries };
}

describe("levels", () => {
  it("keeps the entries at its level and above, and drops the rest", () => {
    const { log, entries } = quietLogger();
    log.debug("hidden");
    log.info("shown");
    log.warn("shown too");
    log.error("and this");
    expect(entries.map((entry) => entry.level)).toEqual(["info", "warn", "error"]);
  });

  it("keeps every level when it starts at debug", () => {
    const { log, entries } = quietLogger({ level: "debug" });
    log.debug("a");
    log.info("b");
    log.warn("c");
    log.error("d");
    expect(entries).toHaveLength(4);
  });

  it("changes the level of one logger, and leaves the others alone", () => {
    const { log, entries } = quietLogger({ level: "debug" });
    const combat = log.child("combat");
    combat.setLevel("error");
    log.info("root keeps info");
    combat.info("combat drops info");
    combat.error("combat keeps errors");
    expect(entries.map((entry) => `${entry.logger}:${entry.level}`)).toEqual(["game:info", "game.combat:error"]);
    expect(log.level).toBe("debug");
    expect(combat.level).toBe("error");
  });

  it("starts a child at its parent's level, and keeps its own level after that", () => {
    const { log } = quietLogger({ level: "warn" });
    const child = log.child("combat");
    expect(child.level).toBe("warn");
    log.setLevel("debug");
    expect(child.level).toBe("warn");
  });

  it("says whether a level would be kept, without writing anything", () => {
    const { log, entries } = quietLogger();
    expect(log.isEnabled("info")).toBe(true);
    expect(log.isEnabled("debug")).toBe(false);
    expect(entries).toHaveLength(0);
  });

  it("does not read the fields of an entry that its level filters out", () => {
    const { log } = quietLogger({ level: "warn" });
    let reads = 0;
    const fields = {
      get value(): number {
        reads += 1;
        return 1;
      },
    };
    log.debug("hidden", fields);
    expect(reads).toBe(0);
    log.error("shown", fields);
    expect(reads).toBe(1);
  });

  it("rejects a level that is not one of the four", () => {
    expect(() => createLogger({ name: "game", level: "loud" as never, sinks: [] })).toThrow(RangeError);
    const { log } = quietLogger();
    expect(() => log.setLevel("loud" as never)).toThrow(RangeError);
    expect(log.level).toBe("info");
  });
});

describe("names and children", () => {
  it("names a child with a dot after its parent's name, at any depth", () => {
    const { log } = quietLogger();
    const combat = log.child("combat");
    expect(combat.name).toBe("game.combat");
    expect(combat.child("ai").name).toBe("game.combat.ai");
    expect(log.child("combat.ai").name).toBe("game.combat.ai");
  });

  it("gives the same logger for the same name", () => {
    const { log } = quietLogger();
    expect(log.child("combat")).toBe(log.child("combat"));
  });

  it("lists the family with the root first, in the order the loggers were made", () => {
    const { log } = quietLogger();
    log.child("a").child("b");
    log.child("c");
    expect(log.loggers().map((item) => item.name)).toEqual(["game", "game.a", "game.a.b", "game.c"]);
  });

  it("accepts letters, digits, dashes and underscores, with dots between the parts", () => {
    expect(createLogger({ name: "demon-queen_2.combat", sinks: [] }).name).toBe("demon-queen_2.combat");
  });

  it("refuses names that the console could not target", () => {
    for (const name of ["", "bad name", "game..x", ".game", "game.", "niño"]) {
      expect(() => createLogger({ name, sinks: [] }), name).toThrow(RangeError);
    }
    const { log } = quietLogger();
    expect(() => log.child("")).toThrow(RangeError);
    expect(() => log.child("two words")).toThrow(RangeError);
  });
});

describe("the buffer", () => {
  it("keeps the newest entries, oldest first, up to its capacity", () => {
    const { log } = quietLogger({ capacity: 3 });
    for (const message of ["m1", "m2", "m3", "m4", "m5"]) log.info(message);
    expect(log.entries().map((entry) => entry.message)).toEqual(["m3", "m4", "m5"]);
  });

  it("is shared by the family, and names the logger that made each entry", () => {
    const { log } = quietLogger();
    const combat = log.child("combat");
    log.info("from the root");
    combat.warn("from the child");
    const entries = log.entries();
    expect(entries.map((entry) => entry.logger)).toEqual(["game", "game.combat"]);
    expect(combat.entries()).toEqual(entries);
  });

  it("gives a copy, so that changing it changes nothing", () => {
    const { log } = quietLogger();
    log.info("kept");
    (log.entries() as LogEntry[]).pop();
    expect(log.entries()).toHaveLength(1);
  });

  it("empties on clear, for the whole family", () => {
    const { log } = quietLogger();
    const combat = log.child("combat");
    log.info("one");
    combat.info("two");
    combat.clear();
    expect(log.entries()).toHaveLength(0);
  });

  it("moves its revision on each kept entry, each level change and each clear, and not on a filtered call", () => {
    const { log } = quietLogger();
    const start = log.revision;
    log.debug("filtered");
    expect(log.revision).toBe(start);
    log.info("kept");
    expect(log.revision).toBe(start + 1);
    log.setLevel("warn");
    expect(log.revision).toBe(start + 2);
    log.clear();
    expect(log.revision).toBe(start + 3);
  });

  it("stamps each entry with the time it was made, in milliseconds since the epoch", () => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date("2026-10-18T10:00:00.000Z"));
    const { log } = quietLogger();
    log.info("stamped");
    expect(log.entries()[0]?.time).toBe(Date.parse("2026-10-18T10:00:00.000Z"));
  });

  it("refuses a capacity that is not a whole number from 1 up", () => {
    for (const capacity of [0, -1, 1.5, Number.NaN]) {
      expect(() => createLogger({ name: "game", capacity, sinks: [] }), String(capacity)).toThrow(RangeError);
    }
  });
});

describe("fields", () => {
  /** The fields of the one entry that a logger wrote with the given fields. */
  function fieldsOf(fields: LogFields): Record<string, unknown> {
    const { log, entries } = quietLogger();
    log.info("m", fields);
    return entries[0]?.fields ?? {};
  }

  it("keeps plain values as they are", () => {
    expect(fieldsOf({ n: 1, s: "x", b: true, nothing: null })).toEqual({ n: 1, s: "x", b: true, nothing: null });
  });

  it("turns an error into its name, message and stack", () => {
    const error = new TypeError("bad");
    expect(fieldsOf({ error })).toEqual({ error: { name: "TypeError", message: "bad", stack: error.stack } });
  });

  it("cuts a cycle, so that every entry can be written", () => {
    const node: Record<string, unknown> = { name: "a" };
    node["self"] = node;
    const fields = fieldsOf({ node });
    expect(fields).toEqual({ node: { name: "a", self: "[Circular]" } });
  });

  it("writes functions, big integers, NaN, infinity and dates as text", () => {
    function named(): void {
      // The function is only here to have a name.
    }
    expect(
      fieldsOf({ fn: named, big: 12n, nan: Number.NaN, inf: Number.POSITIVE_INFINITY, when: new Date(0) }),
    ).toEqual({ fn: "[Function named]", big: "12", nan: "NaN", inf: "Infinity", when: "1970-01-01T00:00:00.000Z" });
  });

  it("drops undefined fields, and writes undefined list items as null", () => {
    const fields = fieldsOf({ gone: undefined, list: [1, undefined] });
    expect(Object.keys(fields)).toEqual(["list"]);
    expect(fields).toEqual({ list: [1, null] });
  });

  it("marks a value that is too deep, and a list that is too long", () => {
    let deep: unknown = "end";
    for (let level = 0; level < 8; level += 1) deep = { next: deep };
    expect(JSON.stringify(fieldsOf({ deep }))).toContain("[Depth limit]");
    const long = fieldsOf({ long: Array.from({ length: 150 }, (_, index) => index) });
    const list = long["long"] as unknown[];
    expect(list).toHaveLength(101);
    expect(list[100]).toBe("[more than 100 items]");
  });

  it("marks a getter that throws, and keeps the other fields", () => {
    const source: Record<string, unknown> = { ok: 1 };
    Object.defineProperty(source, "bad", {
      enumerable: true,
      get(): never {
        throw new Error("nope");
      },
    });
    expect(fieldsOf(source)).toEqual({ ok: 1, bad: "[Unreadable]" });
  });

  it("keeps a key named __proto__ as a plain field", () => {
    const fields = JSON.parse('{"__proto__": 1}') as Record<string, unknown>;
    const kept = fieldsOf(fields);
    expect(Object.getOwnPropertyDescriptor(kept, "__proto__")?.value).toBe(1);
  });

  it("marks a typed array by its size, and does not list its bytes", () => {
    expect(fieldsOf({ bytes: new Uint8Array(1000) })).toEqual({ bytes: "[Uint8Array 1000 bytes]" });
  });

  it("writes a Map as its pairs and a Set as its items", () => {
    expect(fieldsOf({ map: new Map([["a", 1]]), set: new Set([1, 2]) })).toEqual({
      map: [["a", 1]],
      set: [1, 2],
    });
  });

  it("treats null as no fields", () => {
    const { log, entries } = quietLogger();
    log.info("m", null as unknown as LogFields);
    expect(entries[0]?.fields).toEqual({});
  });
});

describe("sinks", () => {
  it("passes each kept entry to every sink, and a sink that throws does not stop the others", () => {
    const { sink, entries } = recordingSink();
    const broken: LogSink = {
      write(): void {
        throw new Error("broken sink");
      },
    };
    const log = createLogger({ name: "game", sinks: [broken, sink] });
    log.info("still delivered");
    expect(entries.map((entry) => entry.message)).toEqual(["still delivered"]);
    expect(log.entries()).toHaveLength(1);
  });

  it("writes the console sink's line at the console method of the level, with the fields when there are any", () => {
    const output = { debug: vi.fn(), info: vi.fn(), warn: vi.fn(), error: vi.fn() };
    const log = createLogger({ name: "game", level: "debug", sinks: [createConsoleSink(output)] });
    log.info("plain");
    log.warn("low", { hp: 3 });
    expect(output.info).toHaveBeenCalledWith("game: plain");
    expect(output.warn).toHaveBeenCalledWith("game: low", { hp: 3 });
  });

  it("writes the bridge sink's line to the bridge log, with the fields as JSON", () => {
    const calls: Array<[string, string]> = [];
    const target: LogTarget = {
      log(level, message): void {
        calls.push([level, message]);
      },
    };
    const log = createLogger({ name: "game", sinks: [createBridgeSink(target)] });
    log.child("save").info("saved", { slot: 2 });
    log.error("failed");
    expect(calls).toEqual([
      ["info", 'game.save: saved {"slot":2}'],
      ["error", "game: failed"],
    ]);
  });

  it("finds the bridge when each entry is written, when it is given a function", () => {
    const calls: string[] = [];
    let current: LogTarget | null = null;
    const log = createLogger({ name: "game", sinks: [createBridgeSink(() => current)] });
    log.info("before the bridge");
    current = {
      log(_level, message): void {
        calls.push(message);
      },
    };
    log.info("after the bridge");
    expect(calls).toEqual(["game: after the bridge"]);
  });

  it("writes to the Velvet Desktop bridge by default, when the page runs in it", () => {
    const { bridge } = createFakeBridge();
    const lines: string[] = [];
    bridge.log = (level, message): void => {
      lines.push(`${level} ${message}`);
    };
    vi.stubGlobal("velvet", bridge);
    const log = createLogger({ name: "game", sinks: [createBridgeSink()] });
    log.info("hello");
    expect(lines).toEqual(["info game: hello"]);
  });

  it("writes nothing by default when the page is not in Velvet Desktop", () => {
    const log = createLogger({ name: "game", sinks: [createBridgeSink()] });
    expect(() => log.info("nowhere to go")).not.toThrow();
    expect(log.entries()).toHaveLength(1);
  });
});

describe("the level words", () => {
  it("lists the levels from debug to error, and checks a level word", () => {
    expect([...LOG_LEVELS]).toEqual(["debug", "info", "warn", "error"]);
    expect(isLogLevel("warn")).toBe(true);
    expect(isLogLevel("loud")).toBe(false);
    expect(isLogLevel(3)).toBe(false);
  });
});
