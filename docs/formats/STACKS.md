# Stacks v1 — cards on a board: formats and rules

Stacks is the rules engine for card games played on a board in the style of Stacklands. Cards lie
on a table and stack on each other. A stack whose cards match a recipe starts a timed job, which
uses up some of its cards and makes others.

This document is normative for version 1. The reference implementation is `crates/velvet-stacks`
(`src/` holds the rules, `tests/fixtures.rs` runs the conformance cases). The conformance fixtures
live in `docs/formats/fixtures/stacks/`: `catalog.json` and `cases/*.json`. A JavaScript runtime
passes the same fixtures, either through the WebAssembly build of the crate or through a port.
Where this text and the code disagree, the fixtures decide.

## Conventions

- JSON is UTF-8. Field names are `snake_case`. Readers ignore fields they do not know.
- Definition ids and recipe ids are non-empty strings. Card and stack ids on a board are unsigned
  integers counted from 1 in creation order (see [Determinism](#determinism)). They stay below 2^53,
  so JavaScript numbers hold them exactly.
- Positions, durations, elapsed time, chances and `dt` are IEEE 754 binary32 numbers. Every
  arithmetic result is rounded to binary32 (`f32` in Rust, `Math.fround` in JavaScript). Counts and
  priorities are integers.
- Random state and seeds are 64-bit unsigned integers, written as decimal strings.

## Documents

Every document has the same envelope:

| Field | Value |
|---|---|
| `format` | `"velvet-stacks"` |
| `version` | `1` |
| `kind` | `"catalog"`, `"board"` or `"case"` |

A reader refuses a document whose `format`, `version` or `kind` it does not read
(`UnsupportedDocument`). The three kinds are the catalog (definitions and recipes), the board (a
save), and a conformance case (a fixture).

## Catalog document (`kind: "catalog"`)

The catalog holds two arrays: `cards` (required) and `recipes` (optional, default empty).

### Card definition

| Field | Type | Default | Meaning |
|---|---|---|---|
| `id` | string | required | Unique in the catalog. Not empty. |
| `name` | string | required | An i18n key. The engine does not read it. |
| `kind` | string | required | Free text: `character`, `resource`, `building`, `zone`, `currency`, and so on. |
| `tags` | string array | `[]` | Free tags. A selector can match one. |
| `stack` | stack rule | see below | What may lie on the card, and how high the stack grows. |
| `value` | integer | `0` | Game currency. The engine does not use it. |

### Stack rule

| Field | Type | Default | Meaning |
|---|---|---|---|
| `accepts` | selector array | `[]` | Cards that may be dropped onto this card. Empty means any card. |
| `max` | integer, at least 1 | `30` | The most cards a stack may hold after a drop onto this card. |

### Selector

A selector sets any of `id` (the card's id is exactly this), `kind` (the card's kind is exactly
this) and `tag` (the card carries this tag). A selector matches when every field it sets matches.
A selector with no fields matches every card.

### Recipe

| Field | Type | Default | Meaning |
|---|---|---|---|
| `id` | string | required | Unique among recipes. Its byte order is the last tie-break. |
| `ingredients` | ingredient array | required | Not empty. |
| `seconds` | number, finite, 0 or more | required | How long the job takes. Zero completes on the next tick. |
| `outputs` | output array | `[]` | What the job makes. May be empty: the ingredients are then just used up. |
| `priority` | integer | `0` | Higher runs first when several recipes match a stack. |

### Ingredient

| Field | Type | Default | Meaning |
|---|---|---|---|
| `select` | selector | required | Which cards qualify. |
| `count` | integer, at least 1 | `1` | How many cards this ingredient needs. |
| `consume` | boolean | `true` | Whether those cards are used up when the job finishes. Workers and zones are usually kept. |

### Output

| Field | Type | Default | Meaning |
|---|---|---|---|
| `card` | id | required | A card definition of the catalog. |
| `count` | integer, at least 1 | `1` | How many copies the job makes. |
| `chance` | number from 0 to 1 | `1` | The probability of each copy appearing. See [Random numbers](#random-numbers). |

Example (from `catalog.json`):

```json
{
  "id": "gather_wood",
  "ingredients": [
    { "select": { "tag": "gather" }, "count": 1, "consume": false },
    { "select": { "id": "forest" }, "count": 1, "consume": false }
  ],
  "seconds": 2.0,
  "outputs": [{ "card": "wood", "count": 1, "chance": 1.0 }],
  "priority": 0
}
```

### Validation

`Catalog` checks everything when it is read. A failure names the rule that broke:

| Error | When |
|---|---|
| `EmptyCardId` | A card id is empty. |
| `DuplicateCard` | Two cards share an id. |
| `ZeroMax` | A stack rule's `max` is 0. |
| `UnknownCard` | A selector by id, or an output, names no card of the catalog. |
| `DuplicateRecipe` | Two recipes share an id. |
| `NoIngredients` | A recipe has no ingredients. |
| `ZeroCount` | An ingredient or an output has a count of 0. |
| `BadSeconds` | A recipe's `seconds` is negative or not a finite number. |
| `BadChance` | An output's `chance` is outside 0 to 1, or not a finite number. |

Recipes are kept in precedence order, whatever order the file lists them in. Writing a catalog
writes them in that order.

## Matching

A stack's cards, bottom to top, match a recipe when they can be assigned **exactly** to the
ingredients:

1. every card fills one unit of one ingredient, and only if the card's definition matches that
   ingredient's selector;
2. every ingredient gets exactly its `count` of cards, and no card is left over.

Selectors may overlap: a plank matches both `tag: wood` and `id: plank`. Matching is therefore an
assignment problem, not a greedy walk. The reference solves it with augmenting paths (Kuhn's
algorithm): cards are tried bottom to top, and units in ingredient order. The first assignment found
decides which cards a finished job consumes. A card is consumed when the ingredient it fills has
`consume: true`.

Because every card must be used, a recipe matches a stack only if its ingredient counts add up to
the stack's height. A stack with an extra card that no recipe uses matches nothing.

**Precedence.** When several recipes match the same stack, the first in this order runs:

1. higher `priority` first;
2. more ingredient cards first (the sum of the counts). Two recipes that match the same stack always
   have the same sum, so this rule only orders the list; it never decides between them;
3. smaller `id` first, compared as UTF-8 bytes.

## Drops and stack rules

Dragging a stack means taking a card and every card above it. The dragged stack is dropped onto a
target card, and it lands on top of that card's stack. The **landing card** is the top card of the
target stack. Its stack rule decides, and the drop is refused when either check fails:

- `accepts`: the dragged stack's bottom card must match the landing card's `accepts` (an empty list
  accepts any card). Refused with `DropRefused`.
- `max`: the stack's height after the drop must not exceed the landing card's `max`. Refused with
  `StackFull`.

A stack cannot be dropped onto itself (`SameStack`).

## Positions

A stack's position is the position of its bottom card. Every card of a stack carries that same
position, so a card's own `x` and `y` always equal its stack's.

## Board save document (`kind: "board"`)

| Field | Type | Meaning |
|---|---|---|
| `rng` | decimal string | The random generator's state. |
| `next_card` | integer | The id the next card gets. |
| `next_stack` | integer | The id the next stack gets. |
| `cards` | array of `{ id, def, x, y }` | Every card on the table. |
| `stacks` | array of `{ id, cards, job? }` | Every stack. `cards` lists the card ids bottom to top. `job`, when present, is `{ recipe, elapsed }`. |

A save holds the state only. The catalog is not in it: a board is loaded with a catalog. Loading
fails (`InvalidSave`, `UnknownCard` or `UnknownRecipe`) when:

- a card id or a stack id is 0 or not below its counter;
- a card's `def`, or a job's `recipe`, is not in the catalog;
- a card is listed in two stacks, or in none;
- a stack is empty, or a job's `elapsed` is negative or not a finite number.

After loading, every stack is matched once, so a save edited by hand cannot keep a job that no
longer matches. Events of that check are not reported. Events not yet read are not saved: read them
(take them) before saving.

## Board actions and events

Every action appends its events to a queue. `tick` returns the queue, and so does an explicit take.
Within one action the order is: the action's own events, then the job events of the stacks it
touched, in ascending stack id.

| Action | Effect | Events, in order |
|---|---|---|
| `spawn(def, pos)` | A new card in a new stack. Refused with `UnknownCard` if `def` is not in the catalog. | `card_spawned`, `stack_changed` of the new stack. |
| `drop(card, onto)` | The stack from `card` upward moves onto the stack of `onto`, if the rules allow it. Refused errors leave everything unchanged and queue no events. | `stack_changed` of the source (the cards left, or `[]` when the source is gone), `stack_changed` of the target. |
| `split(card)` | The card and the cards above it move to a new stack, which gets the next stack id. Splitting the bottom card changes nothing and returns the stack it is in. | `stack_changed` of the source (the cards left), `stack_changed` of the new stack. |
| `move(card, pos)` | The whole stack of `card` moves so that its bottom sits at `pos`. Every card takes the new position. | None. Moving never starts or cancels a job. |
| `remove(card)` | The card leaves the table. The cards above it stay in the same stack. | If the stack empties while it runs a job: `job_cancelled`, then `stack_changed` with `[]`. Otherwise `stack_changed` with the cards left. |

A drop onto a card of the dragged stack's own stack is refused with `SameStack`. `drop` does not
check the height of a stack that was already too tall before the drop; it only checks the result.

## Jobs

### Matching

Only the stacks an action or a finished job changed are matched again (they are *dirty*). The work
for one change is therefore the size of that stack times the number of recipes. The board is never
rescanned for matching.

A matched stack runs the first recipe, in precedence order, that matches it. When a stack is
matched:

| Before | Match result | Events | Job afterwards |
|---|---|---|---|
| no job | none | none | none |
| no job | recipe `R` | `job_started(R)` | `R`, elapsed 0 |
| job `R` | none | `job_cancelled(R)` | none |
| job `R` | `R` again | none | unchanged, progress kept |
| job `R` | another recipe `S` | `job_cancelled(R)`, `job_started(S)` | `S`, elapsed 0 |

### Ticks

`tick(dt)` advances time:

1. A `dt` that is negative or not a finite number counts as 0, and one over 3600 (an hour) counts
   as 3600, which keeps every elapsed time finite.
2. Every running job adds `dt` to its elapsed time.
3. The jobs whose elapsed time is at least their recipe's `seconds` finish, in ascending stack id.
   Each job finishes at most once per tick.
4. The dirty stacks are matched in ascending id, until none is left.
5. The queued events are returned.

A job that finishes keeps the time beyond its duration: its elapsed time becomes `elapsed - seconds`,
and the next run of the same recipe starts with it. A long tick therefore never loses production.
A job started by matching begins at elapsed 0 and first advances on the next tick.

### Finishing a job

Events come in this order:

1. `job_finished { stack, recipe }`.
2. `card_consumed { card, def }` for every card whose ingredient has `consume: true`, bottom to top.
   Those cards leave the table.
3. If any card was consumed: `stack_changed` of the stack, with the cards left (`[]` when it is gone).
4. The outputs, in the order the recipe lists them. Each output makes `count` copies. For each copy
   the chance rule decides whether it appears (see [Random numbers](#random-numbers)). A copy that
   appears is a new card in a new stack: `card_spawned`, then `stack_changed` of its stack.
5. The stack the job ran on is matched again, as in the matching table above. A recipe that matches
   starts at once with `job_started`. The same recipe starts with the carried time as its elapsed;
   another recipe starts at zero.

Positions of the copies, with the row placement (the default; see [Output placement](#output-placement)): the origin is the position the stack had before its cards were consumed.
The copies **that appear** are numbered `k` from 0, across all outputs of this finish, in the order they
appear. Copy `k` lands at `x = origin.x + 1.5 * (k + 1)`, and `y = origin.y`.

A copy that does not appear takes no number and no position, so it does not move the copies after it.
Only the copies that appear count. With three copies of which the first does not appear, the two that
appear land at `origin.x + 1.5` and `origin.x + 3.0`. So the expected position of a repeated card in a case
(a `position` in a `check`) is worked out from the copies that appear, never from the copies that were
rolled. `cases/12_copies_that_skip.json` checks this rule: a skipped copy sits between two copies that
appear, and the second of those lands at `3.0`, not at `4.5`.

### Progress

`progress(card)` is the elapsed time of the job on the card's stack divided by its recipe's
`seconds`, clamped to `0..1`. A recipe with `seconds` 0 reports 1. A stack without a job reports
nothing (`null` in a case). `running_recipe(card)` names the job's recipe, or nothing.

## Outputs and positions

Where the copies of a finished job land is the board's output placement. The row (`1.5` board units
per copy, to the right of the origin) is the default. The other form, a search for free spots, is
described in [Output placement](#output-placement). Callers that want any other layout move the new
stacks with `move`.

## Output placement

Where the copies of a finished job appear is the board's **output placement**. It takes one of two
forms, written as JSON:

| Form | JSON | Meaning |
|---|---|---|
| Row (the default) | `{ "kind": "row" }` | The copies go in a row to the right of the origin, as in [Finishing a job](#finishing-a-job). |
| Free | `{ "kind": "free", "footprint": [width, height], "bounds": [x, y, width, height] }` | Each copy lands on the nearest free spot inside `bounds`, or on the nearest free spot anywhere when none is free inside them. `bounds` may be left out, which means the whole table. |

**Configuration, not state.** The placement is not part of a board save. A board that is created or
loaded starts with the row, and a game sets its placement again after it loads a board, as it loads
its catalog. The reference runner does the same after each `reload` step. Setting a placement changes
nothing on the table: it affects only the copies that finish afterwards.

**Checks.** `footprint` holds two binary32 numbers above zero. `bounds`, when given, holds four binary32
numbers, with a width and a height of zero or more, and `x + width` and `y + height` must be finite
too. A placement that breaks a check fails with `BadPlacement`, and the placement in use stays.

**Free placement.** The copies of a finish are placed one at a time, in the order that
[Finishing a job](#finishing-a-job) gives. Number the copies that appear `k` from 0, as the row does.
For copy `k`:

1. The search starts at `near = (origin.x + width, origin.y)`. `origin` is the position of the job's
   stack before its cards were consumed, and `width` is the footprint's width.
2. The obstacles are the positions of the bottom cards of every stack on the table at that moment:
   the job's own stack, if cards are left on it, and the copies that this finish has already placed.
3. The copy lands on the spot that the search below finds. When bounds are given and the search finds
   none inside them, the search runs again without bounds, and the copy lands on the nearest free spot
   anywhere on the table. Only when no free spot is found at all does the copy land on its row position,
   `(origin.x + 1.5 * (k + 1), origin.y)`, even when a stack is there already.

Each copy is a `card_spawned` and a `stack_changed` event, as in the row.

**The search.** Let `w` and `h` be the footprint, and `(bx, by, bw, bh)` the bounds. Every number is
binary32, and every operation below rounds its result to binary32, in the order written.

- *Overlap.* Two stacks overlap when `|x1 - x2| < w - t` and `|y1 - y2| < h - t`, where
  `t = 1e-5 * max(w, h)`. Stacks that only touch do not overlap.
- *Spots.* The spots are `near + (i * w, j * h)` for whole numbers `i` and `j`. A spot's *distance* is
  `d = dx * dx + dy * dy`, where `dx = i * w` and `dy = j * h`. Its *ring* is `max(|i|, |j|)`.
- *Candidates.* A spot is a candidate when it lies inside the bounds, edges included (without bounds,
  every spot does), and no obstacle overlaps it.
- *Choice.* The result is the candidate with the least `(d, j, i)`, compared in that order: the
  nearest spot, then the smaller `j` (the offset in y), then the smaller `i` (the offset in x). When no
  candidate is found inside bounds, step 3 above applies: the search runs again without bounds, and only
  if that finds no candidate either does the copy take its row position.
- *Rings searched.* Only the rings from `first` to `last` are searched. Let `s(x, y)` be
  `max(|x - near.x| / w, |y - near.y| / h)`, and `F` the largest `s` over the obstacles (without bounds)
  or over the four corners of the bounds (with bounds); `F` is 0 when there are none.
  - Without bounds: `first = 0` and `last = min(ceil(F) + 2, 1000)`, where 1000 is the constant
    `MAX_FREE_RING` (see the ring cap below).
  - With bounds: `(cx, cy)` is the point of the bounds nearest to `near`, `g = s(cx, cy)`,
    `first = max(0, floor(g) - 1)` and `last = ceil(F) + 1`.
- *Stopping.* Once a candidate is known, the search stops at the first ring `r` where
  `(r * min(w, h))^2` is greater than the candidate's `d`: no spot of that ring or a later one can win.

**The ring cap.** `MAX_FREE_RING` is 1000. Without bounds the search never looks past that ring. The cap
keeps the search finite: for a footprint so small that the squares in the stopping rule underflow (below
about 3e-23 board units), the search would otherwise run on for ever. A stack overlaps only spots within one
footprint of it, and blocks at most four of them, so the rings up to 1000 hold over four million spots, and
a table of fewer than about a million stacks always leaves a free spot among them. The cap changes nothing
for an ordinary table, whose nearest free spot lies well inside it. Rounding to binary32 can spoil the
search only where the table lies so far from the origin, in footprints, that neighbouring spots cannot be
told apart (about 16 million footprints or more). There the search may find none, and the copy takes its
row position, which is the only case in which a free placement uses it.

The search tries every spot of the rings it visits, so its cost is the number of spots tried times the
number of stacks. Large bounds that hold no free spot, far from `near`, make it slow: a board should
keep its bounds near its stacks.

The cards package (`findFreeSpot` in `js/packages/cards/src/spot.ts`) runs this search with `Math.fround`
after every operation, so that both runtimes pick the same spot for the same table.

## Random numbers

The generator is xorshift64 with the shifts 13, 7 and 17, on a 64-bit state. It is seeded like this:

1. `z = seed + 0x9E3779B97F4A7C15`, then `z = (z ^ (z >> 30)) * 0xBF58476D1CE4E5B9`, then
   `z = (z ^ (z >> 27)) * 0x94D049BB133111EB`, all modulo 2^64. The state is `z ^ (z >> 31)`.
2. If the state is 0, it becomes `0x9E3779B97F4A7C15`.

`next_u64`: `x ^= x << 13; x ^= x >> 7; x ^= x << 17` (modulo 2^64), and `x` is the result.
`next_unit`: `(next_u64() >> 40) / 2^24`, a value from 0 up to but not including 1. Every such value is
exact in binary32 and in a double.

Draws happen in exactly one place: an output copy whose `chance` is below 1 draws one `next_unit()`
value, and the copy appears when that value is less than the chance. A chance of 1 draws nothing.
Draws follow the order in which jobs finish and their outputs are made. No other code draws.

Reference values for implementers:

| Seed | SplitMix state | First `next_u64` | First `next_unit` | Second `next_unit` |
|---|---|---|---|---|
| 1 | `0x910a2dec89025cc1` | `0x7274658bcb6f4838` | 0.4470885396003723 | 0.8223768472671509 |
| 42 | `0xbdd732262feb6e95` | `0xfb4d394f8eadbd08` | 0.9816470742225647 | 0.37027138471603394 |

## Determinism

The same catalog, seed, actions and ticks give the same events, ids and state, on every platform:

- Ids: card ids and stack ids each count from 1, in creation order. Spawns, copies and splits take
  ids; drops, removes and consumption never reuse one.
- Iteration: every collection is visited in id order. The reference uses sorted maps and sets, never
  hash maps.
- No clock, no threads, and no randomness except the generator above.
- Arithmetic is binary32, performed in the order given above: elapsed `+= dt`; the test
  `elapsed >= seconds`; the carry `elapsed - seconds`; progress `elapsed / seconds` clamped.
  A JavaScript runtime rounds every result with `Math.fround`.
- The fixtures use only values that binary32 represents exactly (multiples of 1/4 here), so a
  decimal reading and a binary reading agree.
- The free placement's search ([Output placement](#output-placement)) is binary32 in the same way.

## Versioning

This document is version 1. A reader refuses any other version. A new optional field whose absence
means the old behaviour may be added within version 1, because readers ignore fields they do not
know. Anything that changes what a document means needs version 2.

## Errors

The engine's refusals have these names. Conformance cases name them too:

`Parse`, `UnsupportedDocument`, `EmptyCardId`, `DuplicateCard`, `ZeroMax`, `UnknownCard`,
`DuplicateRecipe`, `UnknownRecipe`, `NoIngredients`, `ZeroCount`, `BadSeconds`, `BadChance`,
`NoSuchCard`, `SameStack`, `DropRefused`, `StackFull`, `BadPlacement`, `InvalidSave`.

## Conformance cases (`fixtures/stacks/cases/*.json`)

A case is a document with `kind: "case"`, a `name` for humans, a `seed` (a decimal string), `steps`,
and an optional `expect`. A case may also have an optional `placement`, an output placement as in
[Output placement](#output-placement). Without it the board uses the row, the default. Runners read
the cases in file name order. Unknown fields are an error in cases.

| Step | Fields | Meaning |
|---|---|---|
| `spawn` | `def`, `at` (`[x, y]`), `events`? | Spawn a card. |
| `drop` | `card`, `onto`, `error`?, `events`? | Drop a stack. With `error`, the drop must fail, and the error's name must start with that text. |
| `split` | `card`, `events`? | Split a stack. |
| `move` | `card`, `to` (`[x, y]`) | Move a stack. Queues no events. |
| `remove` | `card`, `events`? | Remove a card. |
| `tick` | `dt`, `events`? | Advance time. `events` are those `tick` returns. |
| `check` | `progress`?, `running`?, `position`? | Maps from card id: the expected progress (number or `null`), the expected running recipe (name or `null`), the expected position (`[x, y]`). The copies of an output that do not appear have no position: see "Positions of the copies" above. |
| `reload` | none | Save the board to JSON and load it again with the same catalog. The case's placement is set again afterwards, because a save does not hold it. |

When a step lists `events`, they must equal the events the step produced, in order, in their JSON
form (the `event` tag names the kind, as in `{ "event": "job_finished", "stack": 1, "recipe": "build_house" }`).
A step without `events` ignores them, but the events are always taken, so they never leak into the
next step.

`expect` holds `cards` (definition name to the number of cards of that definition on the table) and
`stacks` (arrays of card ids; compared in any order). A case without `expect` checks only its steps.

