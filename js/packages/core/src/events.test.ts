// Tests for the typed event emitter: subscriptions, removal, one-time listeners, the order of calls
// during an emit, and the tools: taps, the list of event names and the listener counts.

import { describe, expect, it } from "vitest";
import { EventEmitter, type TapEvent } from "./events";

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

describe("EventEmitter taps", () => {
  it("gives a tap every event, with its name and payload", () => {
    const events = new EventEmitter<TestEvents>();
    const seen: string[] = [];
    events.tap((event) => {
      seen.push(event.type === "day" ? `day ${event.payload.day}` : event.type === "note" ? `note ${event.payload}` : "save");
    });
    events.emit("day", { day: 4 });
    events.emit("note", "hello");
    events.emit("save");
    expect(seen).toEqual(["day 4", "note hello", "save"]);
  });

  it("runs a tap for an event that nobody listens to, and before the listeners of that event", () => {
    const events = new EventEmitter<TestEvents>();
    const order: string[] = [];
    events.on("note", () => {
      order.push("listener");
    });
    events.tap(() => {
      order.push("tap");
    });
    events.emit("note", "x");
    events.emit("day", { day: 1 });
    expect(order).toEqual(["tap", "listener", "tap"]);
  });

  it("runs the taps in the order they were added, and removes one with the function that tap returned", () => {
    const events = new EventEmitter<TestEvents>();
    const order: string[] = [];
    events.tap(() => {
      order.push("first");
    });
    const removeSecond = events.tap(() => {
      order.push("second");
    });
    events.emit("save");
    removeSecond();
    events.emit("save");
    expect(order).toEqual(["first", "second", "first"]);
  });

  it("types the payload of each event in the tap's union", () => {
    const events = new EventEmitter<TestEvents>();
    const kinds: Array<TapEvent<TestEvents>["type"]> = [];
    events.tap((event) => {
      kinds.push(event.type);
    });
    events.emit("note", "a");
    expect(kinds).toEqual(["note"]);
  });

  it("does not run a tap that an earlier tap removed in the same emit", () => {
    const events = new EventEmitter<TestEvents>();
    const calls: string[] = [];
    let removeSecond: () => void = () => undefined;
    events.tap(() => {
      calls.push("first");
      removeSecond();
    });
    removeSecond = events.tap(() => {
      calls.push("second");
    });
    events.emit("save");
    expect(calls).toEqual(["first"]);
  });

  it("lets an error thrown by a tap reach the caller of emit, as a listener's error does", () => {
    const events = new EventEmitter<TestEvents>();
    events.tap(() => {
      throw new Error("tap failed");
    });
    expect(() => events.emit("save")).toThrow("tap failed");
  });
});

describe("EventEmitter names", () => {
  it("lists the events with a listener, and the events that were emitted, in first-seen order", () => {
    const events = new EventEmitter<TestEvents>();
    expect(events.names()).toEqual([]);
    events.on("note", () => undefined);
    events.emit("save");
    events.on("day", () => undefined);
    expect(events.names()).toEqual(["note", "save", "day"]);
  });

  it("keeps an event that was emitted, and drops one that was only listened to and then removed", () => {
    const events = new EventEmitter<TestEvents>();
    const removeNote = events.on("note", () => undefined);
    const removeDay = events.on("day", () => undefined);
    events.emit("day", { day: 1 });
    removeNote();
    removeDay();
    expect(events.names()).toEqual(["day"]);
  });

  it("does not change when a tap is added, and does not list a tap", () => {
    const events = new EventEmitter<TestEvents>();
    events.tap(() => undefined);
    expect(events.names()).toEqual([]);
  });
});

describe("EventEmitter listener counts", () => {
  it("counts the listeners of one event, and counts none for an event nobody listens to", () => {
    const events = new EventEmitter<TestEvents>();
    expect(events.listenerCount("note")).toBe(0);
    const listener = (): void => undefined;
    events.on("note", listener);
    events.once("note", () => undefined);
    events.on("note", listener);
    expect(events.listenerCount("note")).toBe(2);
    events.off("note", listener);
    expect(events.listenerCount("note")).toBe(1);
    expect(events.listenerCount("day")).toBe(0);
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
