// The shared stacks fixtures (docs/formats/fixtures/stacks) through this package. The Rust test in
// crates/velvet-wasm reads the same files, so both runtimes must give the same results.

import { readdir, readFile } from "node:fs/promises";
import { describe, expect, it } from "vitest";
import {
  Board,
  CardsError,
  loadCatalog,
  type BoardEvent,
  type Catalog,
  type OutputPlacement,
} from "./index";
import { SHARED_CATALOG } from "./test-support";

/** The folder of the shared fixtures, found from this file (src, cards, packages, js, root). */
const FIXTURES = new URL("../../../../docs/formats/fixtures/stacks/", import.meta.url);

/**
 * One step of a case, as the case file writes it. `events`, when a step lists them, are the events
 * that the step must produce, in order.
 */
type Step =
  | {
      readonly op: "spawn";
      readonly def: string;
      readonly at: readonly [number, number];
      readonly events?: readonly BoardEvent[];
    }
  | {
      readonly op: "drop";
      readonly card: number;
      readonly onto: number;
      readonly error?: string;
      readonly events?: readonly BoardEvent[];
    }
  | { readonly op: "split"; readonly card: number; readonly events?: readonly BoardEvent[] }
  | { readonly op: "move"; readonly card: number; readonly to: readonly [number, number] }
  | { readonly op: "remove"; readonly card: number; readonly events?: readonly BoardEvent[] }
  | { readonly op: "tick"; readonly dt: number; readonly events?: readonly BoardEvent[] }
  | {
      readonly op: "check";
      readonly progress?: Readonly<Record<string, number | null>>;
      readonly running?: Readonly<Record<string, string | null>>;
      readonly position?: Readonly<Record<string, readonly [number, number]>>;
    }
  | { readonly op: "reload" };

/** The output placement of a case, as the case file writes it: sizes and rectangles as arrays. */
type CasePlacement =
  | { readonly kind: "row" }
  | {
      readonly kind: "free";
      readonly footprint: readonly [number, number];
      readonly bounds?: readonly [number, number, number, number];
    };

/** One case file: a seeded board, its steps, and what the table must hold at the end. */
interface Case {
  readonly format: string;
  readonly version: number;
  readonly kind: string;
  readonly name: string;
  readonly seed: string;
  /** Where outputs land. Absent means the row. */
  readonly placement?: CasePlacement;
  readonly steps: readonly Step[];
  readonly expect?: {
    readonly cards?: Readonly<Record<string, number>>;
    readonly stacks?: readonly (readonly number[])[];
  };
}

/** A case file, loaded: its file name and its text. */
interface LoadedCase {
  readonly file: string;
  readonly text: string;
}

/** Reads every case file of the fixtures, in name order. */
async function loadCases(): Promise<LoadedCase[]> {
  const names = (await readdir(new URL("cases/", FIXTURES)))
    .filter((name) => name.endsWith(".json"))
    .sort();
  return Promise.all(
    names.map(async (file) => ({
      file,
      text: await readFile(new URL(`cases/${file}`, FIXTURES), "utf8"),
    })),
  );
}

const cases = await loadCases();

/** The output placement that a case file names, in the shape the cards API takes. */
function placementOf(placement: CasePlacement | undefined): OutputPlacement {
  if (placement === undefined || placement.kind === "row") return { kind: "row" };
  const [width, height] = placement.footprint;
  const bounds = placement.bounds;
  if (bounds === undefined) return { kind: "free", footprint: { width, height } };
  const [x, y, w, h] = bounds;
  return { kind: "free", footprint: { width, height }, bounds: { x, y, width: w, height: h } };
}

/** The boundary code of an engine error name: `DropRefused` is `drop-refused`. */
function codeOf(name: string): string {
  return name.replace(/(?<!^)([A-Z])/g, "-$1").toLowerCase();
}

/** The error that `call` throws, or undefined when it returns. Other errors are rethrown. */
function failureOf(call: () => unknown): CardsError | undefined {
  try {
    call();
    return undefined;
  } catch (error) {
    if (error instanceof CardsError) return error;
    throw error;
  }
}

/** Checks the events that a step produced, when the step lists them. */
function checkEvents(
  at: string,
  got: readonly BoardEvent[],
  want: readonly BoardEvent[] | undefined,
): void {
  if (want !== undefined) expect(got, `${at}: events`).toEqual(want);
}

/** Orders lists of card ids by their text, so that two lists of stacks compare in any order. */
function byIds(a: readonly number[], b: readonly number[]): number {
  const left = a.join(",");
  const right = b.join(",");
  if (left === right) return 0;
  return left < right ? -1 : 1;
}

/** Plays one case step by step, and checks each step and the table at the end. */
async function playCase(file: string, text: string, catalog: Catalog): Promise<void> {
  const data = JSON.parse(text) as Case;
  expect(data.format, file).toBe("velvet-stacks");
  expect(data.version, file).toBe(1);
  expect(data.kind, file).toBe("case");
  const name = `${file} (${data.name})`;

  // A save does not hold the placement, so the case's placement is given again at every load.
  const outputPlacement = placementOf(data.placement);
  let board = await Board.create(catalog, data.seed, { outputPlacement });
  for (const [index, step] of data.steps.entries()) {
    const at = `${name}, step ${index}`;
    switch (step.op) {
      case "spawn": {
        board.spawn(step.def, step.at[0], step.at[1]);
        checkEvents(at, board.takeEvents(), step.events);
        break;
      }
      case "drop": {
        const failure = failureOf(() => board.dropOn(step.card, step.onto));
        if (step.error === undefined) {
          expect(failure, at).toBeUndefined();
        } else {
          expect(failure?.code, `${at}: expected ${step.error}`).toBe(codeOf(step.error));
        }
        checkEvents(at, board.takeEvents(), step.events);
        break;
      }
      case "split": {
        board.split(step.card);
        checkEvents(at, board.takeEvents(), step.events);
        break;
      }
      case "move": {
        board.moveStack(step.card, step.to[0], step.to[1]);
        expect(board.takeEvents(), `${at}: moving queues no events`).toEqual([]);
        break;
      }
      case "remove": {
        board.remove(step.card);
        checkEvents(at, board.takeEvents(), step.events);
        break;
      }
      case "tick": {
        checkEvents(at, board.tick(step.dt), step.events);
        break;
      }
      case "check": {
        for (const [card, want] of Object.entries(step.progress ?? {})) {
          expect(board.progress(Number(card)), `${at}: progress of card ${card}`).toBe(want);
        }
        for (const [card, want] of Object.entries(step.running ?? {})) {
          expect(board.runningRecipe(Number(card)), `${at}: running recipe of card ${card}`).toBe(
            want,
          );
        }
        for (const [card, want] of Object.entries(step.position ?? {})) {
          const found = board.cards().find((view) => view.id === Number(card));
          const got = found === undefined ? undefined : [found.x, found.y];
          expect(got, `${at}: position of card ${card}`).toEqual(want);
        }
        break;
      }
      case "reload": {
        const saved = board.save();
        board.free();
        board = await Board.load(catalog, saved, { outputPlacement });
        break;
      }
    }
  }

  if (data.expect !== undefined) {
    const counts: Record<string, number> = {};
    for (const card of board.cards()) counts[card.def] = (counts[card.def] ?? 0) + 1;
    expect(counts, `${name}: cards on the table`).toEqual(data.expect.cards ?? {});
    if (data.expect.stacks !== undefined) {
      const got = board.stacks().map((stack) => stack.cards);
      const want = [...data.expect.stacks];
      expect([...got].sort(byIds), `${name}: stacks on the table`).toEqual(want.sort(byIds));
    }
  }
  board.free();
}

describe("the shared stacks fixtures", () => {
  it("the shared catalog loads, and its normal form reads back unchanged", async () => {
    const catalog = await loadCatalog(SHARED_CATALOG);
    expect(catalog.recipes.length).toBeGreaterThanOrEqual(12);
    expect(await loadCatalog(JSON.stringify(catalog))).toEqual(catalog);
  });

  it("finds the case files", () => {
    expect(cases.length).toBeGreaterThanOrEqual(8);
  });

  for (const { file, text } of cases) {
    it(`plays ${file} as written`, async () => {
      const catalog = await loadCatalog(SHARED_CATALOG);
      await playCase(file, text, catalog);
    });
  }
});
