// Soak runs: a game's systems run for many seeds and many frames, with a bot that decides the inputs. A run
// reports what the bot chose, the metrics that the test samples, the errors that were thrown, the time per
// frame, and the final state hash of each seed. The same seeds give the same numbers and hashes (only the
// times vary), so two builds can be compared seed by seed. A simulation that checks a game's economy for
// farming is one strategy with metrics; this runner is the generic form of it.

import { Driver } from "./driver";
import { TestingError } from "./errors";
import { createRandom, type Random } from "./random";
import { summarize, type Summary } from "./stats";

/** A run of consecutive seeds: `count` seeds from `from` (1 by default). */
export interface SoakRange {
  readonly count: number;
  readonly from?: number;
}

/** What a game is built from: the seed, its random source, and the driver that runs it. */
export interface SoakContext {
  readonly seed: number;
  readonly random: Random;
  readonly driver: Driver;
}

/**
 * A game built for one seed. `api` is whatever the bot and the metrics need from the game. `hash` is
 * optional, and gives the final state of the run for comparison. `dispose` runs when the run ends.
 */
export interface SoakGame<Api> {
  readonly api: Api;
  readonly hash?: () => string;
  readonly dispose?: () => void;
}

/** What a bot sees at each decision: the run's context, the frame, the game's api, and a way to record a choice. */
export interface BotContext<Api> extends SoakContext {
  /** The frame that is about to run, counted from 0. */
  readonly frame: number;
  /** The game's api, as `create` returned it. */
  readonly api: Api;
  /**
   * Records a value that the bot chose, under a name, and returns it. The report summarises every name.
   * @throws TestingError with code "bad-argument" when the value is not a finite number.
   */
  choose(name: string, value: number): number;
}

/**
 * A bot strategy: it looks at the game and decides the inputs of the next frame, on the driver it is given.
 * A strategy is a plain function, so a stateful one keeps its state in a closure.
 */
export type BotStrategy<Api> = (context: BotContext<Api>) => void;

/** A metric: reads a number from the game after a frame. Returning null or undefined leaves the sample out. */
export type MetricSampler<Api> = (api: Api, sample: { readonly seed: number; readonly frame: number }) => number | null | undefined;

/** Options for {@link runSoak}. */
export interface SoakOptions<Api> {
  /** The seeds, as a list or as a range. Default: 10 seeds from 1. */
  readonly seeds?: readonly number[] | SoakRange;
  /** The frames to run for each seed. */
  readonly frames: number;
  /** Update steps per second of the games. Default 60. */
  readonly rate?: number;
  /** Builds the game for one seed. It must create its loop with `driver.createLoop` and start it. */
  readonly create: (context: SoakContext) => SoakGame<Api> | Promise<SoakGame<Api>>;
  /** The bot. Without it the games run with no inputs. */
  readonly strategy?: BotStrategy<Api>;
  /** The bot decides every this many frames. Default 1. */
  readonly decideEvery?: number;
  /** The metrics, by name. */
  readonly metrics?: Readonly<Record<string, MetricSampler<Api>>>;
  /** The metrics are sampled every this many frames. Default 1. */
  readonly sampleEvery?: number;
  /** Ends a seed's run at its first error. Default true. With false, the run goes on with the next frame. */
  readonly stopOnError?: boolean;
  /** The most errors that the report lists. The count is always complete. Default 50. */
  readonly maxErrors?: number;
}

/** An error that a run raised, with the seed and the frame where it happened. */
export interface SoakError {
  readonly seed: number;
  readonly frame: number;
  readonly message: string;
}

/** The report of a soak run. */
export interface SoakReport {
  /** The seeds that ran, in the order they ran. */
  readonly seeds: readonly number[];
  /** The frames that each seed was asked to run. */
  readonly frames: number;
  /** The frames that completed, over all seeds. */
  readonly framesRun: number;
  /** Every error that was thrown, including the ones that the report does not list. */
  readonly errorCount: number;
  /** The first errors, up to `maxErrors`. */
  readonly errors: readonly SoakError[];
  /** The summary of each metric, over every sample. */
  readonly metrics: Readonly<Record<string, Summary>>;
  /** The summary of each name that the bot chose a value for. */
  readonly chosen: Readonly<Record<string, Summary>>;
  /** The time that each frame's step took, in milliseconds, over every frame. Varies between machines. */
  readonly frameTimeMs: Summary;
  /** The time that the whole run took, in milliseconds. Varies between machines. */
  readonly durationMs: number;
  /** The final hash of each seed, keyed by the seed as text, for the games that give one. */
  readonly finalHashes: Readonly<Record<string, string>>;
}

/** The seeds that a run uses when the options name none. */
const DEFAULT_SEED_COUNT = 10;

/**
 * Runs the games for every seed, with the bot and the metrics, and reports what happened.
 *
 * Each seed gets a new driver and a new game, so a seed's run does not depend on the seeds before it. The
 * driver is disposed at the end of each run.
 *
 * @throws TestingError with code "bad-argument" for a bad option. Errors that the game or the bot throw are
 * not thrown from here: they are listed in the report.
 */
export async function runSoak<Api>(options: SoakOptions<Api>): Promise<SoakReport> {
  const seeds = resolveSeeds(options.seeds);
  const frames = wholeOption(options.frames, "frames", 0);
  const rate = options.rate ?? 60;
  const decideEvery = wholeOption(options.decideEvery ?? 1, "decideEvery", 1);
  const sampleEvery = wholeOption(options.sampleEvery ?? 1, "sampleEvery", 1);
  const maxErrors = wholeOption(options.maxErrors ?? 50, "maxErrors", 0);
  const stopOnError = options.stopOnError ?? true;
  const metricNames = Object.keys(options.metrics ?? {});
  const metricSamples = new Map<string, number[]>(metricNames.map((name) => [name, []]));
  const chosenSamples = new Map<string, number[]>();
  const frameTimes: number[] = [];
  const errors: SoakError[] = [];
  const finalHashes: Record<string, string> = {};
  let errorCount = 0;
  let framesRun = 0;
  const startedAt = performance.now();

  const recordError = (seed: number, frame: number, error: unknown): void => {
    errorCount += 1;
    if (errors.length < maxErrors) errors.push({ seed, frame, message: messageOf(error) });
  };

  for (const seed of seeds) {
    const driver = new Driver({ rate });
    const random = createRandom(seed);
    let game: SoakGame<Api>;
    try {
      game = await options.create({ seed, random, driver });
    } catch (error) {
      recordError(seed, 0, error);
      driver.dispose();
      continue;
    }

    const sample = (frame: number): void => {
      for (const name of metricNames) {
        const sampler = options.metrics?.[name];
        if (sampler === undefined) continue;
        try {
          const value = sampler(game.api, { seed, frame });
          if (typeof value === "number" && Number.isFinite(value)) metricSamples.get(name)?.push(value);
        } catch (error) {
          recordError(seed, frame, error);
        }
      }
    };

    let frame = 0;
    while (frame < frames) {
      try {
        if (options.strategy !== undefined && frame % decideEvery === 0) {
          options.strategy({
            seed,
            random,
            driver,
            frame,
            api: game.api,
            choose: (name, value) => {
              if (!Number.isFinite(value)) {
                throw new TestingError("bad-argument", `a chosen value must be a finite number, got ${value} for ${name}`);
              }
              const values = chosenSamples.get(name) ?? [];
              values.push(value);
              chosenSamples.set(name, values);
              return value;
            },
          });
        }
        const before = performance.now();
        driver.step(1);
        frameTimes.push(performance.now() - before);
      } catch (error) {
        recordError(seed, frame, error);
        frame += 1;
        if (stopOnError || isNotRunning(error)) break;
        continue;
      }
      frame += 1;
      framesRun += 1;
      if (frame % sampleEvery === 0) sample(frame);
    }

    try {
      if (game.hash !== undefined) finalHashes[String(seed)] = game.hash();
    } catch (error) {
      recordError(seed, frame, error);
    }
    game.dispose?.();
    driver.dispose();
  }

  const metrics: Record<string, Summary> = {};
  for (const name of metricNames) metrics[name] = summarize(metricSamples.get(name) ?? []);
  const chosen: Record<string, Summary> = {};
  for (const name of [...chosenSamples.keys()].sort()) chosen[name] = summarize(chosenSamples.get(name) ?? []);
  return {
    seeds,
    frames,
    framesRun,
    errorCount,
    errors,
    metrics,
    chosen,
    frameTimeMs: summarize(frameTimes),
    durationMs: performance.now() - startedAt,
    finalHashes,
  };
}

/** The seeds of a run, from the list or the range in the options. */
function resolveSeeds(seeds: readonly number[] | SoakRange | undefined): number[] {
  if (seeds === undefined) return rangeOf({ count: DEFAULT_SEED_COUNT, from: 1 });
  if (Array.isArray(seeds)) {
    for (const seed of seeds) {
      if (!Number.isSafeInteger(seed) || seed < 0) {
        throw new TestingError("bad-argument", `a soak seed is a safe whole number from 0 up, got ${seed}`);
      }
    }
    return [...seeds];
  }
  return rangeOf(seeds as SoakRange);
}

/** The consecutive seeds of a range. */
function rangeOf(range: SoakRange): number[] {
  const from = range.from ?? 1;
  if (!Number.isInteger(range.count) || range.count < 1) {
    throw new TestingError("bad-argument", `a seed range needs a count of at least 1, got ${range.count}`);
  }
  if (!Number.isSafeInteger(from) || from < 0) {
    throw new TestingError("bad-argument", `a seed range starts at a safe whole number from 0 up, got ${from}`);
  }
  return Array.from({ length: range.count }, (_, index) => from + index);
}

/** A whole number option, from `min` up. */
function wholeOption(value: number, name: string, min: number): number {
  if (!Number.isInteger(value) || value < min) {
    throw new TestingError("bad-argument", `${name} is a whole number from ${min} up, got ${value}`);
  }
  return value;
}

/** True for the error that means the game's loop has stopped, which no later frame can recover from. */
function isNotRunning(error: unknown): boolean {
  return error instanceof TestingError && error.code === "not-running";
}

/** The text of an error, or of any other thrown value. */
function messageOf(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}
