# hello-2d

A small platformer that shows the 2D packages working together: a Tiled level with a parallax background, one-way
planks and spikes; scene2d sprites and a HUD on a canvas at the screen's own pixels; a fixed-step loop with input
actions for the keyboard and a pad; movement with `moveBox`; sounds; achievements that unlock in the run; a screen
stack with a settings panel; two languages; the error overlay; and the developer tools. The art and the sounds are
drawn by scripts, so the example ships without image or audio files. Everything is flat and toon.

## Run it

From `js/`, start the web side (it serves the game on http://localhost:5175 and reloads on each save):

    pnpm --filter hello-2d dev

Open that address in a browser. To run it in Velvet Desktop, start the dev server as above and, from the
repository root, open the desktop host on it:

    cargo run -p velvet-desktop -- js/examples/hello-2d --dev http://localhost:5175

A release folder is made with `cargo run -p velvet-cli -- build --runtime js --path js/examples/hello-2d`.

Build the web files (`dist/`) and check the data (from `js/`):

    pnpm --filter hello-2d build
    pnpm velvet-js check examples/hello-2d

The art, the sounds and the level are generated, and the generated files are kept in the repository. To write them
again, from `js/`:

    node examples/hello-2d/tools/make-art.mjs      # the tileset, the hero and the coin, with their atlases and tileset
    node examples/hello-2d/tools/make-sounds.mjs   # the five sounds
    node examples/hello-2d/tools/make-level.mjs    # the level, from the text drawing in the script

## Play

- Left and right: the arrows, A and D, the pad's d-pad or its left stick.
- Jump: Space, the up arrow, W, or a pad's A. Hold it for the full jump; a tap gives a short hop.
- Escape, P, or a pad's Start pauses. In the pause menu: continue, restart the level, settings, or back to the title.
  Escape closes a menu. A pad's B goes back, and its A is Enter, through the menus' keys.
- Spikes send the hero back to the last checkpoint after a short wait. A fall out of the level does the same.
- The flag ends the run: the result shows the time, the coins and the hits, with two choices.
- The settings change the effects volume (0 to 10) and the language. The texts change at once, the HUD included.
- There are no save slots: the achievements and the settings are kept between runs.

In a development build, F3 shows the developer tools. The watches sit in the bottom right corner: the hero's x and
y, its speed, whether it is grounded, and the coins. The console takes three commands:

- `tp <x> <y>` puts the hero's feet at a point of the level, in level pixels (the level is 3840 by 960).
- `coins <n>` takes the next `n` coins, as ordinary pickups, so the achievements follow.
- `god on|off` keeps the hero from being hurt.

## The level

`public/maps/level1.tmj` is a Tiled map (orthogonal, 32-pixel tiles, CSV data) with an external tileset,
`public/maps/tiles.tsj`. Its layers are, in order: `background` (hills and clouds, drawn by the game at a parallax
factor of 0.4), `ground` (solid), `platforms` (one-way planks: the tile property `one_way` is true), `hazards`
(spikes), and `objects` (points: `spawn`, `checkpoint`, `coin` and `goal`, each at the bottom centre of its cell,
except coins, which sit at the centre). The map is 120 by 30 cells, with 29 coins, one checkpoint and the flag.

`tools/make-level.mjs` holds the level as text, one character per cell, and writes the map. Its header explains the
characters. Tiled can open the result and edit it too; the script is the source the example keeps.

Parallax is set by the game, not by the map: the map reader refuses a layer whose parallax factor is not 1
(docs/formats/TILEMAP.md, "Refused").

## Files

    velvet-game.json             the manifest (id hello-2d, a 1280 by 720 window)
    index.html                   the page; the game mounts in #game
    vite.config.ts               base "./", port 5175, the build target es2022
    tools/make-art.mjs           writes the tileset, the hero and the coin pictures and their atlases (no packages)
    tools/make-sounds.mjs        writes the five sounds (no packages)
    tools/make-level.mjs         writes the level from the text drawing (no packages)
    public/art/                  tiles.png (the tileset's picture), hero.png with hero.json (TexturePacker hash form),
                                 coins.png with coins.json (Aseprite export, tag "spin")
    public/maps/                 level1.tmj (the level) and tiles.tsj (its tileset)
    public/audio/                manifest.json (velvet-audio) and the sounds in sfx/ and ui/
    public/data/achievements.json the achievements (velvet-achievements, docs/formats/ACHIEVEMENTS.md)
    public/data/locales/         the texts: es (the source) and en (velvet-locale, docs/formats/LOCALE.md)
    src/config.ts                every number and id a designer may tune: the physics, the camera, the files and the sounds
    src/physics.ts               one step of the hero: gravity, coyote time, the jump buffer and the variable jump
    src/level.ts                 the level as the game plays it: the solid cells, the spikes, the start, coins and flag
    src/world.ts                 a run: the hero, the coins, the checkpoint, the hits and the time; it reports events
    src/view.ts                  the scene: the sky, the world and the screen, the tile layers, the sprites and the HUD
    src/game.ts                  the wiring: the loop, the keys, the stack, the sound, the achievements and the settings
    src/screens.ts               the menus: title, pause, result and settings
    src/achievements.ts          the achievements' runtime, with the game's own save key
    src/settings.ts              the settings store: the effects volume and the language
    src/texts.ts                 every text id the game uses, and the loader of the catalogs
    src/art.ts                   the pictures and the sheets, and the frame of a tile
    src/log.ts                   the logger and the error reports (the overlay, in the page's language)
    src/devtools.ts              the developer tools (loaded only in development builds)
    src/main.ts                  the entry point
    src/*.test.ts                the tests (see below); test-support.ts holds their helpers

## Packages

- `@velvet/core`: the fixed-step loop, the game clock, the keys and pads (`ActionMap`), the pad's menu keys, the
  settings store, the logger with the bridge and console sinks, and the error reporter.
- `@velvet/scene2d`: the scene with three layers, sprites, text and shapes, the camera with follow, deadzone and
  limits, the Aseprite and TexturePacker atlases with their animations, drawn at the stage's native pixels.
- `@velvet/tiles`: the map, its collision grids (with the one-way planks), `moveBox`, and `drawTileLayer` for the layers.
- `@velvet/ui`: the stage, the screen stack, the menus and the plates, the notices, the settings panel and the error overlay.
- `@velvet/tween`: the coin pop, the hurt flash and the screen fades, on tickers that follow the game clock or the real time.
- `@velvet/audio`: the sounds by id, with the manifest, the buses and the volume curve.
- `@velvet/achievements`: the counters, the goals (counter, all, manual) and the saved progress.
- `@velvet/i18n`: the texts, with Spanish as the source and English as the fallback.
- `@velvet/devtools` (development builds only) and `@velvet/testing` (a dev dependency for the tests).

## Tests

From `js/`, run the example's tests with the rest of the workspace (`pnpm test`), or on their own:

    pnpm exec vitest run examples/hello-2d

- `physics.test.ts`: landing on the ground and on a plank from above, a plank passed from below, a ceiling bumped,
  the coyote time, the jump buffer and the height of a jump held or tapped.
- `level.test.ts`: the shipped map has its start, checkpoint, coins and flag where the design puts them, and its
  solid cells, its planks and its spikes read as the rules say; levels without a start, a goal, coins or the object
  layer are refused.
- `world.test.ts`: a coin is taken once, the checkpoint returns the hero after the spikes, a fall hurts, god mode
  holds, the flag ends the run with its time, coins and hits, and a restart gives everything back.
- `content.test.ts`: both languages have every text the game uses, and nothing else; the achievements file is valid,
  asks for as many coins as the level has, and names the speedrun time that the game checks; every sound file exists;
  the pictures have the sizes that their sheets and tileset describe, and both atlases read.
- `game.test.ts`: the whole game in a page, driven by `@velvet/testing`. The menus, the pause, the language switch
  and the saved achievements are checked; a scripted bot plays the level to the flag, and its input is recorded as a
  replay that must reproduce every checkpoint hash.
- `devtools.test.ts`: the three commands, on a real run.
- `errors.test.ts`: an uncaught error is logged and shows the overlay with its report.

Every test runs in happy-dom, with no browser. The canvas gets a stand-in 2D context, and the scene draws into it.

## Known limits

- The art and the sounds are provisional, and generated: flat colours and an outline, no photographic textures.
- The achievements name icons by art id (`ach.first_coin`, and so on). The example ships no art manifest, so the
  icons are not drawn, and `velvet-js check` does not validate them yet.
- The hero is drawn at its last step (no interpolation between steps), which can judder on a display faster than 60 Hz.
- In a headless browser without a GPU the level ran at about ten frames a second; the update and render code of the
  game takes well under a millisecond of that, so the time goes to the canvas. Not measured on a GPU yet.
- The notice history drawer of `@velvet/ui` needs three text ids (`notice.history.*`); the example has them, and
  `velvet-js check` does not know them yet.

## Package gaps found while building the example

- `packages/scene2d/src/pass.ts:23` declares `Context2D` without `save` and `restore`, which a canvas context has.
  `packages/tiles/src/draw.ts:26` (`TileDrawingContext`) needs both, so `src/view.ts` casts the pass's context in
  one place, with the reason written beside it.
- `packages/scene2d/src/index.ts` does not export `DrawPass` (`pass.ts:85`), so a node that draws its own content has
  to name the pass structurally (`PassLike` in `src/view.ts`).
- `packages/ui/src/notices.ts:430` to `:440` read `notice.history.*` when the notices are made, in every language. A
  game that lacks them gets a warning from its own `onMissing` hook, and `velvet-js check` does not report them.
- `packages/achievements`: the game has to call `load()` at start (the package README says so). Until it runs, the
  runtime answers `isUnlocked` from memory alone. The first draft of this example forgot the call; `Game.start` now
  makes it, and `game.test.ts` checks that a saved unlock survives a new run.
