// A sample test: the rules of the game, with no browser. Run it with `pnpm test`.

import { describe, expect, it } from "vitest";
import { fromSave, newGame, takeTurn, toSave } from "./game";

describe("the game rules", () => {
  it("starts with no turns, and each turn adds one", () => {
    expect(newGame().turns).toBe(0);
    expect(takeTurn(takeTurn(newGame())).turns).toBe(2);
  });

  it("gives back the same state after a save is written and read as JSON", () => {
    const state = takeTurn(newGame());
    expect(fromSave(JSON.parse(JSON.stringify(toSave(state))))).toEqual(state);
  });

  it("refuses a save that is not a state of this game", () => {
    expect(fromSave(null)).toBeNull();
    expect(fromSave("turns")).toBeNull();
    expect(fromSave({ turns: -1 })).toBeNull();
    expect(fromSave({ turns: 1.5 })).toBeNull();
  });
});
