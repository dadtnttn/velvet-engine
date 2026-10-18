// Runs the stacks case files of a game (`kind: "case"`, docs/formats/STACKS.md, "Conformance cases").
// Each case is played step by step on a board of the game's catalog, through @velvet/cards, with the case's
// output placement (`placement`, the row when it names none), and the events and the table are compared
// with what the case expects. It follows the same steps and checks as
// the shared fixture runner in packages/cards/src/fixtures.test.ts, but it reports problems instead of
// failing a test.

import { Board, CardsError, type BoardEvent, type Catalog, type OutputPlacement } from "@velvet/cards";
import { makeIssue, type Issue } from "../issues";

/** A JSON object. */
type Json = Record<string, unknown>;

/**
 * The fields that a case document may hold at the top. Any other field is an error in a case. `placement`
 * is optional: without it the board uses the row (docs/formats/STACKS.md, "Output placement").
 */
const CASE_FIELDS: readonly string[] = ["format", "version", "kind", "name", "seed", "placement", "steps", "expect"];

/** The fields of a free placement, as a case writes it. */
const FREE_PLACEMENT_FIELDS: readonly string[] = ["kind", "footprint", "bounds"];

/** The operations a step may name. */
const OPERATIONS: readonly string[] = ["spawn", "drop", "split", "move", "remove", "tick", "check", "reload"];

/** True for a JSON object that is neither null nor an array. */
function isRecord(value: unknown): value is Json {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

/**
 * The error code that a case names. A case writes the error's name in CamelCase (`DropRefused`), and the
 * code is the same name in lower case with dashes (`drop-refused`).
 * @param name - the name written in the case.
 */
export function caseErrorCode(name: string): string {
  return name.replace(/(?<!^)([A-Z])/gu, "-$1").toLowerCase();
}

/**
 * True when two JSON values are equal: the same keys with equal values, the same items in order.
 * @param a - one value.
 * @param b - the other value.
 */
export function sameJson(a: unknown, b: unknown): boolean {
  if (a === b) return true;
  if (Array.isArray(a) || Array.isArray(b)) {
    if (!Array.isArray(a) || !Array.isArray(b) || a.length !== b.length) return false;
    return a.every((item: unknown, index: number) => sameJson(item, b[index]));
  }
  if (!isRecord(a) || !isRecord(b)) return false;
  const keys = Object.keys(a);
  if (keys.length !== Object.keys(b).length) return false;
  return keys.every((key) => Object.hasOwn(b, key) && sameJson(a[key], b[key]));
}

/** A position written in a case: two numbers. */
function pairOf(value: unknown): [number, number] | undefined {
  if (!Array.isArray(value) || value.length !== 2) return undefined;
  const [x, y] = value as unknown[];
  return typeof x === "number" && typeof y === "number" ? [x, y] : undefined;
}

/** A whole number written in a case (a card id, for example). */
function integerOf(value: unknown): number | undefined {
  return typeof value === "number" && Number.isInteger(value) ? value : undefined;
}

/**
 * A number written in a case that the cards API takes: finite, and still finite as binary32, which is how the
 * engine holds every number of a placement (see `isBinary32` in @velvet/cards). JSON cannot hold NaN, but a
 * number such as 1e400 reads as infinity, and 1e39 is too large for binary32.
 */
function finiteNumber(value: unknown): number | undefined {
  return typeof value === "number" && Number.isFinite(Math.fround(value)) ? value : undefined;
}

/** Two finite numbers written in a case, such as a footprint: `[width, height]`. */
function finitePairOf(value: unknown): [number, number] | undefined {
  if (!Array.isArray(value) || value.length !== 2) return undefined;
  const [first, second] = value.map(finiteNumber);
  return first === undefined || second === undefined ? undefined : [first, second];
}

/** Four finite numbers written in a case, such as bounds: `[x, y, width, height]`. */
function finiteQuadOf(value: unknown): [number, number, number, number] | undefined {
  if (!Array.isArray(value) || value.length !== 4) return undefined;
  const [a, b, c, d] = value.map(finiteNumber);
  return a === undefined || b === undefined || c === undefined || d === undefined ? undefined : [a, b, c, d];
}

/**
 * The output placement that a case names, in the shape the cards API takes (docs/formats/STACKS.md, "Output
 * placement"). A case that names none has the row. The shape, the fields and the signs are checked here. The
 * engine checks the rest, such as the binary32 range: `Board.create` refuses that with `bad-placement`.
 * @param value - the case's `placement` field as written, or undefined.
 * @returns the placement, or the code and the sentence of what is wrong with it.
 */
export function casePlacement(
  value: unknown,
): { readonly placement: OutputPlacement } | { readonly code: string; readonly message: string } {
  if (value === undefined) return { placement: { kind: "row" } };
  if (!isRecord(value)) {
    return { code: "structure", message: 'a placement is an object: { "kind": "row" } or { "kind": "free", ... }' };
  }
  if (value["kind"] === "row") {
    const extra = Object.keys(value).find((key) => key !== "kind");
    if (extra !== undefined) return { code: "unknown-field", message: `"${extra}" is not a field of a row placement` };
    return { placement: { kind: "row" } };
  }
  if (value["kind"] !== "free") return { code: "structure", message: 'the placement kind is "row" or "free"' };
  const extra = Object.keys(value).find((key) => !FREE_PLACEMENT_FIELDS.includes(key));
  if (extra !== undefined) return { code: "unknown-field", message: `"${extra}" is not a field of a free placement` };
  const footprint = finitePairOf(value["footprint"]);
  if (footprint === undefined) {
    return { code: "structure", message: "a free placement needs footprint: two numbers that fit binary32, [width, height]" };
  }
  const [width, height] = footprint;
  if (!(width > 0 && height > 0)) {
    return { code: "bad-placement", message: "the footprint's width and height must be above zero" };
  }
  if (value["bounds"] === undefined) return { placement: { kind: "free", footprint: { width, height } } };
  const bounds = finiteQuadOf(value["bounds"]);
  if (bounds === undefined) {
    return { code: "structure", message: "bounds needs four numbers that fit binary32: [x, y, width, height]" };
  }
  const [x, y, boundsWidth, boundsHeight] = bounds;
  if (!(boundsWidth >= 0 && boundsHeight >= 0)) {
    return { code: "bad-placement", message: "the bounds' width and height must be zero or more" };
  }
  if (!Number.isFinite(x + boundsWidth) || !Number.isFinite(y + boundsHeight)) {
    return { code: "bad-placement", message: "the bounds must end at finite numbers" };
  }
  return {
    placement: {
      kind: "free",
      footprint: { width, height },
      bounds: { x, y, width: boundsWidth, height: boundsHeight },
    },
  };
}

/** The events a step lists, or undefined when it lists none. */
function eventsOf(step: Json): readonly BoardEvent[] | undefined {
  const events = step["events"];
  return events === undefined ? undefined : (events as readonly BoardEvent[]);
}

/**
 * Runs one case file on a board of the catalog.
 * @param file - the case's file, relative to the game folder.
 * @param data - the parsed case document.
 * @param catalog - the catalog to play it on.
 * @returns the problems found: an empty list when the case passes.
 */
export async function runStacksCase(file: string, data: unknown, catalog: Catalog): Promise<Issue[]> {
  const issue = (path: string, code: string, message: string): Issue => makeIssue("error", file, path, code, message);
  if (!isRecord(data)) return [issue("(file)", "structure", "a case must be a JSON object")];
  if (data["format"] !== "velvet-stacks" || data["version"] !== 1 || data["kind"] !== "case") {
    return [issue("(file)", "unsupported-document", "expected a velvet-stacks case of version 1")];
  }
  const unknown = Object.keys(data).filter((key) => !CASE_FIELDS.includes(key));
  if (unknown.length > 0) {
    return [issue(unknown[0] ?? "(file)", "unknown-field", `"${unknown[0]}" is not a field of a case`)];
  }
  const seed = data["seed"];
  const steps = data["steps"];
  if (typeof seed !== "string" || !Array.isArray(steps)) {
    return [issue("(file)", "structure", "a case needs a seed (a string of digits) and a list of steps")];
  }

  const named = casePlacement(data["placement"]);
  if (!("placement" in named)) return [issue("placement", named.code, named.message)];
  const outputPlacement = named.placement;

  let board: Board;
  try {
    board = await Board.create(catalog, seed, { outputPlacement });
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    // A free placement that the engine refuses is the placement's fault, not the seed's.
    if (outputPlacement.kind === "free" && error instanceof CardsError && error.code === "bad-placement") {
      return [issue("placement", "bad-placement", `the placement is not valid: ${message}`)];
    }
    return [issue("seed", "seed", `the seed is not valid: ${message}`)];
  }

  try {
    const expect = isRecord(data["expect"]) ? data["expect"] : undefined;
    return await playSteps(file, board, catalog, steps as readonly unknown[], outputPlacement, expect);
  } finally {
    board.free();
  }
}

/**
 * Plays the steps of a case and checks the final table.
 * @param file - the case's file.
 * @param first - the board the case starts with. A `reload` step replaces it.
 * @param catalog - the catalog, for the reloads.
 * @param steps - the steps, as written.
 * @param outputPlacement - the placement that the case names. A save does not hold it, so each reload gives it
 *   again to `Board.load`.
 * @param expect - what the table must hold at the end, or undefined.
 */
async function playSteps(
  file: string,
  first: Board,
  catalog: Catalog,
  steps: readonly unknown[],
  outputPlacement: OutputPlacement,
  expect: Json | undefined,
): Promise<Issue[]> {
  let board = first;
  const problemAt = (index: number, code: string, message: string): Issue[] => [
    makeIssue("error", file, `steps[${index}]`, code, message),
  ];
  for (const [index, raw] of steps.entries()) {
    if (!isRecord(raw) || typeof raw["op"] !== "string" || !OPERATIONS.includes(raw["op"])) {
      return problemAt(index, "case-step", "a step needs an op: spawn, drop, split, move, remove, tick, check or reload");
    }
    const step = raw;
    try {
      const mismatch = applyStep(board, step);
      if (mismatch !== undefined) return problemAt(index, "case-step", mismatch);
      if (step["op"] === "reload") {
        const saved = board.save();
        board.free();
        board = await Board.load(catalog, saved, { outputPlacement });
      }
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      const code = error instanceof CardsError ? error.code : "internal";
      return problemAt(index, code, `step ${String(step["op"])} failed: ${message}`);
    }
  }
  return expect === undefined ? [] : checkFinalTable(file, board, expect);
}

/**
 * Applies one step to the board and returns what did not match, or undefined when the step passed.
 * A step that the case expects to fail counts as passed when it fails with the named error.
 * @param board - the board to change.
 * @param step - the step, as written.
 */
function applyStep(board: Board, step: Json): string | undefined {
  const op = step["op"];
  switch (op) {
    case "spawn": {
      const def = step["def"];
      const at = pairOf(step["at"]);
      if (typeof def !== "string" || at === undefined) return "a spawn step needs def and at ([x, y])";
      board.spawn(def, at[0], at[1]);
      return eventsMismatch(board.takeEvents(), eventsOf(step));
    }
    case "drop": {
      const card = integerOf(step["card"]);
      const onto = integerOf(step["onto"]);
      if (card === undefined || onto === undefined) return "a drop step needs card and onto (card ids)";
      const expected = step["error"];
      let failure: string | undefined;
      try {
        board.dropOn(card, onto);
      } catch (error) {
        if (!(error instanceof CardsError)) throw error;
        failure = error.code;
      }
      if (typeof expected === "string") {
        if (failure !== caseErrorCode(expected)) return `the drop should fail with ${expected}, but it gave ${failure ?? "no error"}`;
      } else if (failure !== undefined) {
        return `the drop was refused (${failure})`;
      }
      return eventsMismatch(board.takeEvents(), eventsOf(step));
    }
    case "split": {
      const card = integerOf(step["card"]);
      if (card === undefined) return "a split step needs card";
      board.split(card);
      return eventsMismatch(board.takeEvents(), eventsOf(step));
    }
    case "move": {
      const card = integerOf(step["card"]);
      const to = pairOf(step["to"]);
      if (card === undefined || to === undefined) return "a move step needs card and to ([x, y])";
      board.moveStack(card, to[0], to[1]);
      return board.takeEvents().length === 0 ? undefined : "moving a stack should queue no events";
    }
    case "remove": {
      const card = integerOf(step["card"]);
      if (card === undefined) return "a remove step needs card";
      board.remove(card);
      return eventsMismatch(board.takeEvents(), eventsOf(step));
    }
    case "tick": {
      const dt = step["dt"];
      if (typeof dt !== "number") return "a tick step needs dt (seconds)";
      return eventsMismatch(board.tick(dt), eventsOf(step));
    }
    case "check":
      return checkStep(board, step);
    case "reload":
      return undefined;
    default:
      return `unknown op ${String(op)}`;
  }
}

/**
 * Compares the events of a step with the events the case lists, when it lists some.
 * @param got - the events the board produced.
 * @param want - the events the case lists, or undefined.
 */
function eventsMismatch(got: readonly BoardEvent[], want: readonly BoardEvent[] | undefined): string | undefined {
  if (want === undefined || sameJson(got, want)) return undefined;
  return `expected the events ${JSON.stringify(want)}, but got ${JSON.stringify(got)}`;
}

/**
 * The `check` step: progress, running recipe and position of cards, each keyed by card id.
 * @param board - the board to read.
 * @param step - the step, as written.
 */
function checkStep(board: Board, step: Json): string | undefined {
  const progress = step["progress"];
  if (isRecord(progress)) {
    for (const [card, want] of Object.entries(progress)) {
      const got = board.progress(Number(card));
      if (got !== want) return `progress of card ${card} should be ${String(want)}, but it is ${String(got)}`;
    }
  }
  const running = step["running"];
  if (isRecord(running)) {
    for (const [card, want] of Object.entries(running)) {
      const got = board.runningRecipe(Number(card));
      if (got !== want) return `running recipe of card ${card} should be ${String(want)}, but it is ${String(got)}`;
    }
  }
  const position = step["position"];
  if (isRecord(position)) {
    for (const [card, want] of Object.entries(position)) {
      const view = board.cards().find((candidate) => candidate.id === Number(card));
      const got = view === undefined ? undefined : [view.x, view.y];
      if (!sameJson(got, want)) return `position of card ${card} should be ${JSON.stringify(want)}, but it is ${JSON.stringify(got)}`;
    }
  }
  return undefined;
}

/**
 * Compares the table at the end of a case with its `expect` block: the cards by definition, and the
 * stacks (each a list of card ids, compared in any order).
 * @param file - the case's file.
 * @param board - the board at the end.
 * @param expect - the `expect` block.
 */
function checkFinalTable(file: string, board: Board, expect: Json): Issue[] {
  const counts: Record<string, number> = {};
  for (const card of board.cards()) counts[card.def] = (counts[card.def] ?? 0) + 1;
  const wantCards = isRecord(expect["cards"]) ? expect["cards"] : {};
  if (!sameJson(counts, wantCards)) {
    return [makeIssue("error", file, "expect.cards", "case-table", `the cards on the table are ${JSON.stringify(counts)}, expected ${JSON.stringify(wantCards)}`)];
  }
  const wantStacks = expect["stacks"];
  if (Array.isArray(wantStacks)) {
    // Each stack keeps its own order (bottom card first); only the list of stacks is unordered.
    const got = board.stacks().map((stack) => [...stack.cards]).sort(byText);
    const want = wantStacks.map((stack: unknown) => (Array.isArray(stack) ? [...stack] : stack)).sort(byText);
    if (!sameJson(got, want)) {
      return [makeIssue("error", file, "expect.stacks", "case-table", `the stacks are ${JSON.stringify(got)}, expected ${JSON.stringify(want)}`)];
    }
  }
  return [];
}

/**
 * Orders lists of card ids by their text, so that two lists of stacks can be compared in any order.
 * @param a - one list.
 * @param b - the other list.
 */
function byText(a: unknown, b: unknown): number {
  const left = JSON.stringify(a);
  const right = JSON.stringify(b);
  if (left === right) return 0;
  return left < right ? -1 : 1;
}
