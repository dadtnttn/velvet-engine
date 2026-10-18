import { describe, expect, it } from "vitest";
import type { Driver } from "./driver";
import { runSoak, type BotStrategy, type MetricSampler, type SoakGame } from "./soak";
import { startFakeGame, type FakeGame } from "./test-support";

/** Builds the fake game for a seed, with its loop started on the run's driver. */
function create({ seed, driver }: { seed: number; driver: Driver }): SoakGame<FakeGame> {
  const game = startFakeGame(driver, { seed });
  return { api: game, hash: () => game.hash(), dispose: () => game.dispose() };
}

/**
 * A bot that taps the jump key every five frames, clicks every seven frames (the fake game draws its random
 * numbers while the button is held), and chooses a bet on every frame.
 */
const tapping: BotStrategy<FakeGame> = ({ driver, frame, random, choose }) => {
  if (frame % 5 === 0) driver.keyDown("Space");
  if (frame % 5 === 2) driver.keyUp("Space");
  if (frame % 7 === 0) driver.pointerDown("left");
  if (frame % 7 === 3) driver.pointerUp("left");
  choose("bet", random.int(1, 10));
};

/** The metrics that the tests sample: the coins and the shots of the fake game. */
const metrics: Record<string, MetricSampler<FakeGame>> = {
  coins: (game) => game.state.coins,
  shots: (game) => game.state.fired,
};

describe("runSoak", () => {
  it("runs every seed for every frame, and summarises the metrics, the choices and the frame times", async () => {
    const report = await runSoak({ seeds: { count: 4, from: 1 }, frames: 100, create, strategy: tapping, metrics });
    expect(report.seeds).toEqual([1, 2, 3, 4]);
    expect(report.framesRun).toBe(400);
    expect(report.errorCount).toBe(0);
    expect(report.metrics["coins"]?.count).toBe(400);
    expect(report.chosen["bet"]?.count).toBe(400);
    expect(report.chosen["bet"]?.min).toBeGreaterThanOrEqual(1);
    expect(report.chosen["bet"]?.max).toBeLessThanOrEqual(10);
    expect(report.frameTimeMs.count).toBe(400);
    expect(report.durationMs).toBeGreaterThanOrEqual(0);
    expect(Object.keys(report.finalHashes)).toEqual(["1", "2", "3", "4"]);
  });

  it("gives the same numbers and the same hashes for the same seeds", async () => {
    const options = { seeds: [5, 9, 12], frames: 80, create, strategy: tapping, metrics };
    const first = await runSoak(options);
    const second = await runSoak(options);
    expect(second.metrics).toEqual(first.metrics);
    expect(second.chosen).toEqual(first.chosen);
    expect(second.finalHashes).toEqual(first.finalHashes);
    expect(second.framesRun).toBe(first.framesRun);
  });

  it("gives a different hash for a seed that is not the same, since the game draws from it", async () => {
    const report = await runSoak({ seeds: [1, 2], frames: 60, create, strategy: tapping });
    expect(report.finalHashes["1"]).not.toBe(report.finalHashes["2"]);
  });

  it("ends a seed's run at its first error, and lists the seed and the frame", async () => {
    const failing: BotStrategy<FakeGame> = ({ seed, frame }) => {
      if (seed === 2 && frame === 5) throw new Error("bot failed");
    };
    const report = await runSoak({ seeds: [1, 2, 3], frames: 20, create, strategy: failing });
    expect(report.errorCount).toBe(1);
    expect(report.errors).toEqual([{ seed: 2, frame: 5, message: "bot failed" }]);
    // Seed 1 runs 20 frames, seed 2 runs 5 before its error, and seed 3 runs 20.
    expect(report.framesRun).toBe(45);
  });

  it("goes on to the next frame after an error when stopOnError is off", async () => {
    const failing: BotStrategy<FakeGame> = ({ seed }) => {
      if (seed === 1) throw new Error("always");
    };
    const report = await runSoak({ seeds: [1, 2], frames: 5, create, strategy: failing, stopOnError: false });
    expect(report.errorCount).toBe(5);
    expect(report.framesRun).toBe(5);
  });

  it("lists a game that fails to start at frame 0, and runs the other seeds", async () => {
    const report = await runSoak({
      seeds: [1, 2],
      frames: 10,
      create: (context) => {
        if (context.seed === 2) throw new Error("no start");
        return create(context);
      },
    });
    expect(report.errors).toEqual([{ seed: 2, frame: 0, message: "no start" }]);
    expect(report.framesRun).toBe(10);
  });

  it("reports a chosen value that is not a finite number as an error", async () => {
    const report = await runSoak({
      seeds: [1],
      frames: 3,
      create,
      strategy: ({ choose }) => {
        choose("bad", Number.NaN);
      },
    });
    expect(report.errorCount).toBe(1);
    expect(report.errors[0]?.message).toMatch(/finite number/);
  });

  it("decides and samples at the intervals that it is given", async () => {
    const decisions: number[] = [];
    const report = await runSoak({
      seeds: [1],
      frames: 20,
      create,
      strategy: ({ frame }) => {
        decisions.push(frame);
      },
      decideEvery: 4,
      metrics: { coins: metrics["coins"] ?? (() => 0) },
      sampleEvery: 10,
    });
    expect(decisions).toEqual([0, 4, 8, 12, 16]);
    expect(report.metrics["coins"]?.count).toBe(2);
  });

  it("leaves out a metric sample that the sampler does not return", async () => {
    const report = await runSoak({
      seeds: [1],
      frames: 10,
      create,
      metrics: { evenFrames: (game, sample) => (sample.frame % 2 === 0 ? game.state.coins : null) },
    });
    expect(report.metrics["evenFrames"]?.count).toBe(5);
  });

  it("uses ten seeds from 1 when the options name none", async () => {
    const report = await runSoak({ frames: 1, create });
    expect(report.seeds).toEqual([1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
  });

  it("refuses a seed that is not a safe whole number, a seed range without seeds, and a bad frame count", async () => {
    await expect(runSoak({ seeds: [-1], frames: 1, create })).rejects.toThrow(/safe whole number/);
    await expect(runSoak({ seeds: { count: 0 }, frames: 1, create })).rejects.toThrow(/count of at least 1/);
    await expect(runSoak({ frames: -1, create })).rejects.toThrow(/frames is a whole number/);
  });
});
