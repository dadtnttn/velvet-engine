// @vitest-environment happy-dom
// Tests of the developer tools' commands, on a real run: `tp` puts the hero's feet where it is told and refuses what is
// outside the level, `coins` takes the next pickups and refuses a count that is not a whole number, and `god` switches
// the god mode. The overlay itself is mounted once, to check that it can be made and removed.

import { describe, expect, it } from "vitest";
import { EventEmitter, GameClock, GameLoop, createLogger, type Logger } from "@velvet/core";
import { createStage } from "@velvet/ui";
import type { GameEvents } from "./game";
import { mountDevtools, placeHero, setGod, takeCoins, type DevtoolsTarget } from "./devtools";
import type { Level } from "./level";
import { levelFromRows } from "./test-support";
import { Run } from "./world";

/** A target of the tools over a real run of a small level with two coins. */
async function targetOver(): Promise<{ target: DevtoolsTarget; run: Run; level: Level }> {
  const level = await levelFromRows(["S.o.oF", "######"]);
  const run = new Run(level);
  const host = document.createElement("div");
  document.body.append(host);
  const logger: Logger = createLogger({ name: "hello-2d", level: "error", sinks: [] });
  const target: DevtoolsTarget = {
    host,
    clock: new GameClock(),
    loop: new GameLoop({ update: () => undefined }),
    stage: createStage(host, { width: 1920, height: 1080 }),
    events: new EventEmitter<GameEvents>(),
    logger,
    level,
    run: () => run,
    teleport: (x, y) => run.teleport(x, y),
    giveCoins: (count) => {
      run.giveCoins(count);
    },
    setGod: (on) => run.setGod(on),
  };
  return { target, run, level };
}

describe("the tp command", () => {
  it("puts the hero's feet at the point, and the hero falls from there", async () => {
    const { target, run } = await targetOver();
    expect(placeHero(target, 50, 60)).toBe("hero at 50, 60");
    expect(run.hero.x).toBe(40);
    expect(run.hero.y).toBe(32);
  });

  it("refuses a point outside the level, and a point that is not a number", async () => {
    const { target } = await targetOver();
    expect(() => placeHero(target, -1, 10)).toThrow(RangeError);
    expect(() => placeHero(target, 10, 10_000)).toThrow(/inside the level/);
    expect(() => placeHero(target, Number.NaN, 10)).toThrow(/two numbers/);
  });
});

describe("the coins command", () => {
  it("takes the next coins of the level, and says how many there are", async () => {
    const { target, run } = await targetOver();
    expect(takeCoins(target, 1)).toBe("coins: 1 of 2");
    expect(run.coins).toBe(1);
    expect(takeCoins(target, 5)).toBe("coins: 2 of 2");
  });

  it("refuses a count that is not a whole number from 1 up", async () => {
    const { target } = await targetOver();
    expect(() => takeCoins(target, 0)).toThrow(RangeError);
    expect(() => takeCoins(target, 1.5)).toThrow(RangeError);
  });
});

describe("the god command", () => {
  it("turns the god mode on and off", async () => {
    const { target, run } = await targetOver();
    expect(setGod(target, true)).toBe("god on");
    expect(run.god).toBe(true);
    expect(setGod(target, false)).toBe("god off");
    expect(run.god).toBe(false);
  });
});

describe("the overlay", () => {
  it("can be mounted and removed", async () => {
    const { target } = await targetOver();
    const tools = mountDevtools(target);
    expect(tools.enabled).toBe(true);
    tools.destroy();
  });
});
