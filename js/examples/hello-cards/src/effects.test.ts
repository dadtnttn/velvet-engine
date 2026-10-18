// Tests of the effects that board events turn into: pops, fades, settles and notices.

import { describe, expect, it } from "vitest";
import type { BoardEvent } from "@velvet/cards";
import { effectsOf } from "./effects";

describe("effectsOf", () => {
  it("makes a new card pop in and gives it a place to settle at", () => {
    const event: BoardEvent = { event: "card_spawned", card: 4, def: "wood", x: 1.5, y: 0.5 };
    expect(effectsOf(event)).toEqual([
      { kind: "pop", card: 4 },
      { kind: "settle", card: 4, x: 1.5, y: 0.5 },
    ]);
  });

  it("fades a card that the table consumed", () => {
    expect(effectsOf({ event: "card_consumed", card: 9, def: "wood" })).toEqual([{ kind: "fade", card: 9 }]);
  });

  it("announces a finished crafting job, and keeps quiet about gathering", () => {
    expect(effectsOf({ event: "job_finished", stack: 1, recipe: "make_planks" })).toEqual([
      { kind: "toast", toast: { key: "done.make_planks", tone: "info" } },
    ]);
    expect(effectsOf({ event: "job_finished", stack: 1, recipe: "gather_wood" })).toEqual([]);
  });

  it("warns when a job stops", () => {
    expect(effectsOf({ event: "job_cancelled", stack: 1, recipe: "gather_wood" })).toEqual([
      { kind: "toast", toast: { key: "toast.cancelled", tone: "warn" } },
    ]);
  });

  it("does nothing for a job that starts or a stack that changes", () => {
    expect(effectsOf({ event: "job_started", stack: 1, recipe: "gather_wood" })).toEqual([]);
    expect(effectsOf({ event: "stack_changed", stack: 1, cards: [1, 2] })).toEqual([]);
  });
});
