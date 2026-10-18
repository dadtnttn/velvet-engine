# JS tutorial 03: cards on a board

This tutorial walks through `js/examples/hello-cards`, a small game in the style of Stacklands. Villagers gather
wood, stone and berries, wood becomes planks, and planks and stone build a house. A market turns berries and planks
into coins. The game uses every package of `js/packages` that a card game needs: `@velvet/cards` (the rules, in
WebAssembly), `@velvet/ui` (the table and the HUD), `@velvet/i18n` (the texts), `@velvet/guion` (the guide's
conversation) and `@velvet/core` (the loop, the clock, the keys and the saves).

## Run it

From `js/`, these are the commands that the example's README lists and that were run while it was written:

    pnpm --filter hello-cards dev       # the page on http://localhost:5173
    pnpm --filter hello-cards build     # the web files, in examples/hello-cards/dist
    pnpm velvet-js check examples/hello-cards
    pnpm exec vitest run examples/hello-cards

To play in Velvet Desktop, keep the dev server running and, from the repository root:

    cargo run -p velvet-desktop -- js/examples/hello-cards --dev http://localhost:5173

## The files

    public/data/catalog.json     the cards and the recipes, in velvet-stacks (docs/formats/STACKS.md)
    public/data/locales/         the texts: es (the source) and en
    public/data/guion/           the guide's conversation: intro.md (es) and intro.en.md (en)
    src/config.ts               every number a designer may change: speeds, the day length, the starting table
    src/layout.ts               the stack geometry on the table (pure, tested)
    src/drop.ts                 which card a release lands on (pure, tested)
    src/effects.ts              the board's events as effects: pops, fades and notices (pure, tested)
    src/board-view.ts           the cards and their progress bars, drawn as DOM from the board's snapshots
    src/drag.ts                 picks a card up (split), follows the pointer (moveStack), drops it (dropOn)
    src/game.ts                 the loop, the clock, the board, the saves and the texts, wired together

## The catalog

`public/data/catalog.json` is the rulebook. Each card has an id, a name (a text id, so the name is translated), a kind,
tags, and what its stack accepts. A recipe names its ingredients by tag or by id, how many of each, and whether each is
used up; it takes `seconds`, and makes outputs with a `chance`:

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

The recipes of the example:

| Recipe | Ingredients | Makes | Time |
|---|---|---|---|
| Chop wood | villager, forest | wood | 2 s |
| Mine stone | villager, quarry | stone (and rarely a coin) | 2.5 s |
| Pick berries | villager, berry bush | berries | 2 s |
| Saw planks | villager, 2 wood (used up) | plank | 3 s |
| Build a house | 2 planks, stone (used up) | house | 4 s |
| Build a market | plank, 2 stone (used up) | market | 4 s |
| Sell berries | market, 2 berries (used up) | coin | 1 s |
| Sell planks | market, plank (used up) | 2 coins | 1.5 s |

`src/game.ts` loads it with `loadCatalog`, and the rules are the engine's own: the same catalog gives the same results in
the Rust and the JavaScript runtimes.

## The board

`Board` is the table. The game makes one with `Board.create(catalog, seed)`, and places the starting cards with
`spawn(def, x, y)`. The calls that the game uses:

- `dropOn(card, onto)` stacks a card on another card, if the rules allow it. A refused drop changes nothing.
- `split(card)` takes a card and the cards above it into a new stack, which the drag then moves.
- `moveStack(card, x, y)` moves a whole stack, without events.
- `tick(dt)` advances the jobs by `dt` game seconds and returns the events in order. `takeEvents()` returns the events
  that the actions queued, so the game can show them before a save.
- `cards()` and `stacks()` are snapshots for drawing, and `save()` and `Board.load(catalog, save)` keep the table.

## The loop

`src/game.ts` runs the loop in fixed steps. Each step:

1. reads the keys (Space pauses, 1, 2 and 3 set the speed, F5 and F9 save and load, Escape opens the menu);
2. advances the clock by the real time of the step, and takes the game time that passed from it;
3. calls `board.tick(gameSeconds)`, and applies each event: a new card pops in, a consumed card fades, and a crafted
   thing makes a notice. Gathering stays quiet, because it finishes every few seconds.

A frame then draws the table from the board's snapshots, so the board is the only source of truth for what the table
shows. A game paused on the clock does not advance its jobs, because `gameSeconds` is zero.

## Drag and drop

`src/drag.ts` makes a drag from three calls. A press on a card splits it from its stack, and the pointer carries it with
`moveStack` in the frame. A release calls `dropOn` on the card under the pointer. If the rules refuse the drop, the stack
springs back to where it came from, with a notice. `src/drop.ts` decides which card a release lands on, and it is pure, so
its test does not need a browser.

## The guide and the texts

`src/dialogue.ts` plays the guide's conversation in a plate at the bottom of the table, with `@velvet/guion`. Its file
`public/data/guion/intro.md` has a scene per part, and a choice whose second option ends it. The English conversation is
its own file, `intro.en.md`, because a guion's text is not an i18n key (see JS tutorial 02).

Every visible word comes from `@velvet/i18n`. `src/texts.ts` loads the catalogs, and the HUD (`src/hud.ts`) and the pause
menu (`src/pause-menu.ts`) write their words again when the language changes.

`pnpm velvet-js check examples/hello-cards` reads the catalog, the guiones and the locales, and the output ends with
`hello-cards: ok, 0 errors, 0 warnings`.

## Developer tools

In a development build, F3 shows the developer tools. `src/devtools.ts` adds two watches (the cards on the table, and the
coins among them) and a console command: `coins 3` puts three coin cards on the table, at the free spots nearest the middle.
The command takes a whole number from 1 to 20. A release build does not contain the module.

## Tests

`pnpm exec vitest run examples/hello-cards` runs eleven files. The rules (the catalog, the economy, the layout, the drop and
the effects) are tested without a browser. The page parts run in happy-dom: the board view, the drag, the intro plate, the
pause menu, the devtools wiring, and one whole run of the game, where a villager gathers through the live loop.

## Next

[JS tutorial 04: tools and tests](04_tools_and_tests.md) explains the checks, the replays and the driver that the examples use.
