// Tests for the event log: the cap, the filter, the pause, the payload text and the revision.

import { describe, expect, it } from "vitest";
import { EventLog } from "./events";

describe("EventLog", () => {
  it("lists the newest entries first", () => {
    const log = new EventLog(10, () => 0);
    log.record("a", undefined);
    log.record("b", undefined);
    expect(log.list("").map((entry) => entry.name)).toEqual(["b", "a"]);
  });

  it("keeps only the newest entries up to its cap", () => {
    const log = new EventLog(2, () => 0);
    log.record("a", undefined);
    log.record("b", undefined);
    log.record("c", undefined);
    expect(log.size).toBe(2);
    expect(log.list("").map((entry) => entry.name)).toEqual(["c", "b"]);
  });

  it("filters by a part of the name, ignoring case", () => {
    const log = new EventLog(10, () => 0);
    log.record("Day", undefined);
    log.record("save", undefined);
    log.record("dayBreak", undefined);
    expect(log.list("DAY").map((entry) => entry.name)).toEqual(["dayBreak", "Day"]);
    expect(log.list("zzz")).toEqual([]);
  });

  it("counts the events that arrive while paused, and keeps none of them", () => {
    const log = new EventLog(10, () => 0);
    log.setPaused(true);
    log.record("a", undefined);
    log.record("b", undefined);
    expect(log.size).toBe(0);
    expect(log.dropped).toBe(2);
    log.setPaused(false);
    log.record("c", undefined);
    expect(log.list("").map((entry) => entry.name)).toEqual(["c"]);
  });

  it("writes the payload on one line, and nothing for an event without one", () => {
    const log = new EventLog(10, () => 0);
    log.record("gold", { amount: 5 });
    log.record("tick", undefined);
    const [tick, gold] = log.list("");
    expect(gold?.summary).toBe('{"amount":5}');
    expect(tick?.summary).toBe("");
  });

  it("never throws for a payload that cannot be read, since it runs inside the game's emit", () => {
    const log = new EventLog(10, () => 0);
    const hostile = new Proxy(
      {},
      {
        ownKeys() {
          throw new Error("no keys");
        },
      },
    );
    expect(() => log.record("odd", hostile)).not.toThrow();
    expect(log.list("")[0]?.summary).toContain("unreadable");
  });

  it("stamps each entry with the clock and a sequence number", () => {
    let now = 100;
    const log = new EventLog(10, () => now);
    log.record("a", undefined);
    now = 250;
    log.record("b", undefined);
    const [second, first] = log.list("");
    expect(first).toMatchObject({ seq: 1, atMs: 100 });
    expect(second).toMatchObject({ seq: 2, atMs: 250 });
  });

  it("changes its revision on every change, and clears its entries and dropped count", () => {
    const log = new EventLog(10, () => 0);
    const start = log.revision;
    log.record("a", undefined);
    log.setPaused(true);
    log.record("b", undefined);
    expect(log.revision).toBeGreaterThan(start + 1);
    log.setPaused(false);
    log.clear();
    expect(log.size).toBe(0);
    expect(log.dropped).toBe(0);
  });
});
