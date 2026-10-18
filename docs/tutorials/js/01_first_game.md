# JS tutorial 01: your first game

This tutorial makes a game folder from the template, checks its data, runs it in a browser and in Velvet
Desktop, and runs its tests. It takes about ten minutes once the tools are installed.

## Before you start

- Node.js 25 and pnpm 11 (the JS workspace is developed with them; see `js/README.md`).
- Rust, for Velvet Desktop: `cargo` must be on the path.
- On Windows, the WebView2 runtime (Windows 11 and Microsoft Edge already include it).
- The engine checked out, with its JS workspace installed once from `js/`:

      pnpm install

## 1. Make the game folder

From `js/`, `pnpm velvet-js new` writes the template into a folder that is missing or empty:

    cd js
    pnpm velvet-js new ../games/my-first-game --id my-first-game --title "My First Game"

It prints the files it wrote and the next steps:

    created .../games/my-first-game with 15 files:
      README.md
      ...
    next: cd .../games/my-first-game, then pnpm install and pnpm dev (see its README.md)

The id is 1 to 64 lowercase letters, digits, `-` or `_`. The title is at most 120 characters. `--force`
writes into a folder that is not empty, and never deletes anything. The template's `package.json` links the
engine's packages (`@velvet/core`, `@velvet/i18n` and `@velvet/ui`) by path, so the folder can stay where
it is, as long as the engine stays where it was when the folder was made.

## 2. Check the data

`velvet-js check` reads every data file of the folder with the package that owns its format. From `js/`:

    pnpm velvet-js check ../games/my-first-game

A clean game ends with one line:

    my-first-game: ok, 0 errors, 0 warnings

The check exits with 1 when there is an error, so it can run in a script.

## 3. Run it in a browser

The template's scripts are in its `package.json`. From the game folder, install its packages once, then
start the dev server:

    cd ../games/my-first-game
    pnpm install
    pnpm dev

The dev server serves the game on http://localhost:5173 (the port is set in `vite.config.ts`) and reloads
the page when a file changes. A browser can play the game without Velvet Desktop, which is useful for
layout work.

## 4. Run it in Velvet Desktop

Velvet Desktop shows the game in a native window. In development it loads the dev server instead of the
game's files. From the engine's repository root, with the dev server from step 3 running:

    cargo run -p velvet-desktop -- games/my-first-game --dev http://localhost:5173

The folder is still read for `velvet-game.json`, and the window takes the size of its `window` section. The
other flags of the host (`--devtools`, `--size` and `--capture`) are in `crates/velvet-desktop/README.md`.

## 5. Run the tests and the type check

From the game folder:

    pnpm test
    pnpm typecheck

The sample test in `src/game.test.ts` checks the rules of the game with no browser. `pnpm check:data` runs
the data check of step 2 on the game folder itself.

## 6. Change the game

- `src/game.ts` holds the rules: a turn counter, and the save format that goes with it.
- `src/main.ts` holds the page: the stage, the texts, the save slot and the game loop.
- `public/data/locales/es/00_ui.json` is the source language, and `en/00_ui.json` the English text. A missing
  English text falls back to the Spanish one.

Each change can be checked again with step 2 and step 5.

## Next

[JS tutorial 02: a visual novel](02_visual_novel.md) walks through a larger example, `js/examples/hello-vn`.
