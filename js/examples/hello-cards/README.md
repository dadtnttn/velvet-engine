# hello-cards

A small cards-on-a-board game in the style of Stacklands. It is public example code, and it uses
every package of `js/packages` inside Velvet Desktop. Villagers gather wood, stone and berries.
Wood becomes planks. Planks and stone build a house. A market turns berries and planks into coins.

## Run it

From `js/`, start the web side, then open the desktop host on it (from the repository root):

    pnpm --filter hello-cards dev
    cargo run -p velvet-desktop -- js/examples/hello-cards --dev http://localhost:5173

Ship a release (the host is built in release mode, and the folder lands in `release/`):

    cargo run -p velvet-cli -- build --runtime js --path js/examples/hello-cards

Check the code (from `js/`): `pnpm test` and `pnpm typecheck`.

## Play

- Drag a card onto another card to stack them. A stack that the rules refuse springs back, with a
  notice. Dropping a stack on empty table leaves it there.
- A villager on a forest, a quarry or a berry bush gathers, and uses nothing up.
- Space pauses the clock. The keys 1, 2 and 3 set the speed. F5 quick-saves and F9 quick-loads.
- Escape opens the pause menu: continue, save, load, the language, full screen, and quit.
- Enter and Space go on through the guide's conversation.

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

## Files

    velvet-game.json          the manifest (id hello-cards, a 1280 by 720 window)
    index.html                the page; the game mounts in #game
    vite.config.ts            base "./", and the .wasm stays a file
    public/data/catalog.json  the cards and recipes, in velvet-stacks (docs/formats/STACKS.md)
    public/data/locales/      the texts: es (the source) and en, in velvet-locale (docs/formats/LOCALE.md)
    public/data/guion/        the guide's conversation: intro.md (es) and intro.en.md (en)
    src/config.ts             every number a designer may change: speeds, day length, slots, data URLs
    src/layout.ts             stack geometry on the table (pure, tested)
    src/drop.ts               the drop decision: which card a release lands on (pure, tested)
    src/effects.ts            board events to effects: pops, fades, notices (pure, tested)
    src/board-view.ts         cards and progress bars drawn as DOM from the board's snapshots
    src/drag.ts               pick up (split), follow the pointer (moveStack), drop (dropOn) or spring back
    src/hud.ts                the top bar and the key hints
    src/dialogue.ts           the guion conversation in a plate, with a menu for choices
    src/pause-menu.ts         the pause menu
    src/game.ts               wires it together: the loop, the clock, the board, the saves, the texts
    src/platform.ts           the bridge: logs, settings, full screen, quit (with browser fallbacks)
    src/style.css             the theme (the --vu-* properties of @velvet/ui) and the geometry

## Packages

- `@velvet/core`: `GameLoop` (fixed 60 Hz steps), `GameClock` (speeds 1, 2 and 3, a day every 120
  game seconds, `onDay`, `snapshot` and `restore`), `ActionMap` (the keys), `createSaveStore` (the
  bridge in Velvet Desktop, localStorage in a browser), and `getBridge` (logs and settings).
- `@velvet/ui`: `createStage` (1920 by 1080, letterboxed, `toLogical` and `rectOf` for the pointer),
  `createPlate`, `createMenu`, `createButton`, `createKeycap`, `createChip`, `createMeter` and
  `createToast`, with `velvet-ui.css` themed in `style.css`.
- `@velvet/i18n`: `I18n` with Spanish as the source and English as the fallback. Every visible word
  comes from the catalogs, including a plural (`hud.coins`), and the whole screen changes language live.
- `@velvet/cards`: `loadCatalog`, and `Board` (`spawn`, `dropOn`, `split`, `moveStack`, `remove`,
  `tick`, `takeEvents`, `progress`, `cards`, `stacks`, `save`, `load`). Refused drops are told apart
  by their `CardsError` codes.
- `@velvet/guion`: `GuionLibrary` (`add`, `start`) and `Conversation` (`current`, `next`, `choose`).
- `@velvet/wasm`: loaded by `@velvet/cards` and `@velvet/guion`; the game does not call it directly.

## Decisions

- **Two guiones, not one.** Guion text is not an i18n key, so the English conversation is its own
  file. The conversation starts in the language in use, and a language change applies from the next
  conversation. The file is `intro.en.md`, so its id is `intro_en`.
- **Copies are moved.** The engine places a copy 1.5 units to the right of its source. The game then
  moves a new card to the nearest free place (`findFreeSpot`), so cards never stack by accident.
- **Notices are quiet for gathering.** Gathering recipes finish every few seconds, so they only pop
  a card in; crafting and trading say so in a notice.

## Known limits

- Cards are moved with the pointer only. Keyboard players can use the menus and the keys, not move cards.
- Only the intro is written. The guide does not come back after it.
- A drop that is refused puts the stack back on the card it came from. If that card has gone (a job
  used it up during the drag), the stack returns to its old place instead.

## Tests

`src/*.test.ts` cover the pure parts (layout, the drop decision, the effects, the save shape, the
card rules, the texts and the guiones) and the page parts in happy-dom (the board view, the intro
plate, the pause menu, the drag controller, and one whole run of the game). `test-support.ts` holds
their helpers. happy-dom has no layout, so `stubLayout` gives each element the rectangle that its
inline position implies.
