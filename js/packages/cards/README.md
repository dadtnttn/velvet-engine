# @velvet/cards

Cards on a board, in the style of Stacklands: cards lie on a table and stack on each other, and a
stack whose cards match a recipe starts a timed job that uses up some cards and makes others. The
rules are velvet-stacks' own (`crates/velvet-stacks`, format in `docs/formats/STACKS.md`), compiled to
WebAssembly through `@velvet/wasm`, so the Rust and the JavaScript runtimes give the same results.

## A game loop

```ts
import { Board, loadCatalog, type BoardSave } from "@velvet/cards";

const catalog = await loadCatalog(await (await fetch("catalog.json")).text());
const board = await Board.create(catalog, 42);

const forest = board.spawn("forest", 2, 3);
const worker = board.spawn("worker", 0, 0);
board.dropOn(worker, forest); // the worker starts gathering wood

let last = performance.now();

function frame(now: number): void {
  const dt = (now - last) / 1000; // seconds since the last frame
  last = now;

  // `tick` advances the jobs and returns every event since the last call, in order.
  for (const event of board.tick(dt)) {
    if (event.event === "card_spawned") playSound("pop");
  }

  // Draw from the snapshots. A card's position is its stack's bottom card, and a stack's job is
  // null when it runs none.
  for (const card of board.cards()) drawCard(card.def, card.x, card.y);
  for (const stack of board.stacks()) {
    if (stack.job !== null) drawProgress(stack.x, stack.y, stack.job.progress);
  }
  requestAnimationFrame(frame);
}

requestAnimationFrame(frame);
```

`playSound`, `drawCard` and `drawProgress` are the game's own functions.

Saving takes the events first, because a save does not hold queued events (`tick` and `takeEvents`
both return them):

```ts
board.takeEvents();
localStorage.setItem("slot-1", JSON.stringify(board.save()));

const saved = JSON.parse(localStorage.getItem("slot-1") ?? "null") as BoardSave;
const restored = await Board.load(catalog, saved);
```

## The API

| Call | What it does |
|---|---|
| `loadCatalog(json)` | Checks a catalog file and returns it in its normal form (recipes in precedence order). |
| `Board.create(catalog, seed, options?)` | An empty board. Equal seeds and equal actions give equal events. `options.outputPlacement` sets where outputs land (see below). |
| `Board.load(catalog, save, options?)` | The board of a save, with the same job and random state. A save does not hold the output placement, so pass it again here. |
| `spawn(def, x, y)` | A new card in its own stack. Returns its id. |
| `moveStack(card, x, y)` | Moves the whole stack of `card`. Queues no events. |
| `dropOn(card, onto)` | Drops the stack from `card` upward onto the stack of `onto`, if the rules allow it. |
| `split(card, at?)` | Takes `card` and the cards above it into a new stack. Returns the stack's id. The new stack stays where the old one is, on top of it, unless `at` (`{ x, y }`) moves it there in the same call. |
| `remove(card)` | Takes `card` off the table. The cards above stay in place. |
| `tick(dt)` | Advances time and returns the events, in order. |
| `takeEvents()` | Returns the events that actions queued, without advancing time. |
| `progress(card)`, `runningRecipe(card)` | The progress (0 to 1) and recipe of the job on the card's stack, or null. |
| `cards()`, `stacks()` | Snapshots of the table, sorted by id, for drawing. |
| `freeSpot(near, size, options?)` | The nearest spot to `near` where a new stack of `size` does not overlap the stacks on the table, or null. Options: `bounds` (a rectangle the spot must lie in) and `except` (a card whose stack is not counted). Queues no events. |
| `setOutputPlacement(placement)` | Sets where the outputs of finished jobs land: `{ kind: "row" }` (the default) or `{ kind: "free", footprint, bounds? }`. Queues no events and changes no card or job. |
| `outputPlacement()` | The placement in use. |
| `save()` | The board's state as a save document. |
| `free()` | Frees the board in WebAssembly. Any call afterwards throws `disposed`. |

## Conventions

- **Ids** are numbers that count from 1 in creation order. A number holds an id exactly up to
  2^53 - 1; any other id fails with `bad-id`.
- **Seeds** make the chance rolls. A seed is a safe whole number (a `number`), any 64-bit value as a
  `bigint`, or any 64-bit value as a string of decimal digits. A `number` above 2^53 - 1 is refused,
  because its digits would not be the seed that was meant.
- **Positions and times** are binary32 numbers. JSON writes them in their shortest decimal form, so
  `Math.fround` gives back the exact value when arithmetic on them matters.
- **Split** keeps the new stack where the old one is: its bottom card takes the old position, so the
  new stack lies on top of the old one. A UI that shows the two apart passes `at` to `split`, or
  calls `moveStack(card, x, y)` on the new stack's bottom card (`card`) right after.
- **Free spots** are found in JavaScript, from the table's snapshot. Every stack covers the same `size`
  from its position, so two stacks overlap when their positions differ by less than `size` on both
  axes; stacks that only touch do not overlap. The search tries the grid of `size` steps around `near`,
  ring by ring outward, and returns the nearest spot it tries; ties go to the smaller y, then x. It is
  the nearest grid spot, not the nearest position in the plane. Like the module, it computes in
  binary32, rounding every result with `Math.fround`.
- **Output placement** decides where the copies of a finished job appear. `row` (the default) puts
  them in a row to the right of the stack, one step of 1.5 apart. `free` puts each copy on the nearest
  free spot, found by the same search as `freeSpot`, from one footprint width to the right of the
  stack. `footprint` is a `BoardSize` and `bounds` (optional) a `BoardRect`; when no spot inside the
  bounds is free, the search runs again without bounds, and the copy takes the nearest free spot anywhere
  on the table. Only when there is none does it land on its row position, even over a stack. The
  placement is set on the board, not saved: pass it to `Board.create` or `Board.load` as
  `options.outputPlacement`, or call
  `setOutputPlacement` after loading. The module's search and `freeSpot` agree, spot for spot, on the
  same table (the tests check it).
- **Events** are tagged by `event` (`job_started`, `job_cancelled`, `job_finished`, `card_spawned`,
  `card_consumed`, `stack_changed`). A refused action changes nothing and queues no events.
- **Failures** are `CardsError`s with a `code` (see `src/error.ts`): `bad-input`, `bad-id`,
  `unknown-card`, `drop-refused`, `stack-full`, `same-stack`, `no-such-card`, `bad-placement`,
  `invalid-save`, and the catalog codes such as `duplicate-card`. `bad-placement` means a footprint side
  that is not above zero, or a negative bounds side; `bad-input` means a number that is not finite.

## Tests

`pnpm test` (from `js/`) runs the shared stacks fixtures in `docs/formats/fixtures/stacks`, the same
cases that `crates/velvet-stacks` and `crates/velvet-wasm` run, and the unit tests next to the code. The
output placement is tested through the real WebAssembly module, and against `findFreeSpot` on random
tables: the position the module picks for each copy must equal the one the JavaScript search picks.
