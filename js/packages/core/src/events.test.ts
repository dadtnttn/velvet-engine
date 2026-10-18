// Tests for the typed event emitter: subscriptions, removal, one-time listeners and the
// order of calls during an emit.

import { describe, expect, it } from "vitest";
import { EventEmitter } from "./events";

interface TestEvents {
  day: { day: number };
  save: void;
  note: string;
}

describe("EventEmitter subscriptions", () => {
  it("calls the listeners of an event with its payload", () => {
    const events = new EventEmitter<TestEvents>();
    const seen: number[] = [];
    events.on("day", (payload) => {
      seen.push(payload.day);
    });
    events.emit("day", { day: 2 });
    expect(seen).toEqual([2]);
  });

  it("returns a function from on that removes the listener", () => {
    const events = new EventEmitter<TestEvents>();
    let calls = 0;
    const unsubscribe = events.on("note", () => {
      calls += 1;
    });
    events.emit("note", "first");
    unsubscribe();
    events.emit("note", "second");
    expect(calls).toBe(1);
  });

  it("removes a listener with off", () => {
    const events = new EventEmitter<TestEvents>();
    let calls = 0;
    const listener = (): void => {
      calls += 1;
    };
    events.on("note", listener);
    events.off("note", listener);
    events.off("note", listener); // removing it again is harmless
    events.emit("note", "x");
    expect(calls).toBe(0);
  });

  it("runs a once-listener only at the first emit", () => {
    const events = new EventEmitter<TestEvents>();
    const payloads: string[] = [];
    events.once("note", (payload) => {
      payloads.push(payload);
    });
    events.emit("note", "first");
    events.emit("note", "second");
    expect(payloads).toEqual(["first"]);
  });

  it("removes a once-listener before it runs, through the function that once returned", () => {
    const events = new EventEmitter<TestEvents>();
    let calls = 0;
    const unsubscribe = events.once("note", () => {
      calls += 1;
    });
    unsubscribe();
    events.emit("note", "x");
    expect(calls).toBe(0);
  });

  it("calls listeners in the order they were added", () => {
    const events = new EventEmitter<TestEvents>();
    const order: string[] = [];
    events.on("note", () => {
      order.push("a");
    });
    events.on("note", () => {
      order.push("b");
    });
    events.emit("note", "x");
    expect(order).toEqual(["a", "b"]);
  });

  it("runs a listener that is added twice only once", () => {
    const events = new EventEmitter<TestEvents>();
    let calls = 0;
    const listener = (): void => {
      calls += 1;
    };
    events.on("note", listener);
    events.on("note", listener);
    events.emit("note", "x");
    expect(calls).toBe(1);
  });

  it("keeps the listeners of different events apart", () => {
    const events = new EventEmitter<TestEvents>();
    const notes: string[] = [];
    let days = 0;
    events.on("note", (payload) => {
      notes.push(payload);
    });
    events.on("day", () => {
      days += 1;
    });
    events.emit("day", { day: 1 });
    expect(notes).toEqual([]);
    expect(days).toBe(1);
  });
});

describe("EventEmitter during an emit", () => {
  it("skips a listener that an earlier listener removed in the same emit", () => {
    const events = new EventEmitter<TestEvents>();
    const calls: string[] = [];
    let removeSecond: () => void = () => undefined;
    events.on("note", () => {
      calls.push("first");
      removeSecond();
    });
    removeSecond = events.on("note", () => {
      calls.push("second");
    });
    events.emit("note", "x");
    expect(calls).toEqual(["first"]);
  });

  it("leaves a listener that is added during an emit for the next emit", () => {
    const events = new EventEmitter<TestEvents>();
    const calls: string[] = [];
    const inner = (): void => {
      calls.push("inner");
    };
    events.on("note", () => {
      calls.push("outer");
      events.on("note", inner);
    });
    events.emit("note", "x");
    expect(calls).toEqual(["outer"]);
    events.emit("note", "y");
    expect(calls).toEqual(["outer", "outer", "inner"]);
  });

  it("lets an error thrown by a listener reach the caller", () => {
    const events = new EventEmitter<TestEvents>();
    events.on("note", () => {
      throw new Error("boom");
    });
    expect(() => events.emit("note", "x")).toThrow("boom");
  });
});

describe("EventEmitter payload types", () => {
  it("emits an event that has no payload", () => {
    const events = new EventEmitter<TestEvents>();
    let saved = 0;
    events.on("save", () => {
      saved += 1;
    });
    events.emit("save");
    expect(saved).toBe(1);
  });

  it("does nothing when nobody listens", () => {
    const events = new EventEmitter<TestEvents>();
    expect(() => events.emit("note", "unheard")).not.toThrow();
  });

  it("rejects a payload of the wrong type at compile time", () => {
    const events = new EventEmitter<TestEvents>();
    let calls = 0;
    events.on("day", () => {
      calls += 1;
    });
    // @ts-expect-error the "day" payload is an object, not a string
    events.emit("day", "tuesday");
    // @ts-expect-error the "save" event carries no payload
    events.emit("save", 1);
    expect(calls).toBe(1);
  });
});
