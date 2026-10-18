# {{GAME_TITLE}}

A Velvet JS game, created from the `velvet-js new` template. Its id is `{{GAME_ID}}`.

It is a small starting point: a title screen with a new game, a continue button, a language switch
and a counter of turns, a save slot, and one test. Replace the rules in `src/game.ts` and the texts in
`public/data/locales/` with your own.

## Next steps

1. Install the packages (once, and after changing `package.json`):

       pnpm install

2. Start the web side. It serves the game on http://localhost:5173 and reloads on each save:

       pnpm dev

3. Run the game in Velvet Desktop, from the engine's repository (`{{ENGINE_REPO}}`), with this folder
   and the address of the dev server:

       cargo run -p velvet-desktop -- <path to this folder> --dev http://localhost:5173

4. Check the game's data (the texts and any catalogs, sheets or guiones): `pnpm check:data`.
   Its rules are in the engine's `js/packages/tools/README.md`.

5. Run the tests and the type check: `pnpm test` and `pnpm typecheck`.

6. Make a release, from the engine's repository. It runs `pnpm run build` here and writes the folder
   `release/` in this game:

       cargo run -p velvet-cli -- build --runtime js --path <path to this folder>

## Files

    velvet-game.json                 the manifest: id, title, window (only the fields Velvet Desktop reads)
    index.html                       the page; the game mounts in #game
    vite.config.ts                   base "./", and the engine's folder is allowed for the linked packages
    package.json                     the scripts, and the engine's packages linked with `link:`
    tsconfig.json                    the engine's shared compiler options
    public/data/locales/es/          the texts in the source language (velvet-locale, docs/formats/LOCALE.md)
    public/data/locales/en/          the English texts, which fall back to Spanish for any missing id
    src/main.ts                      the stage, the texts, the save slot and the game loop
    src/game.ts                      the rules of the game (no browser needed)
    src/game.test.ts                 a sample test of the rules
    src/style.css                    the page's look; the HUD's defaults come from @velvet/ui

## Packages

- `@velvet/core`: `GameLoop` (a fixed-step loop), `createSaveStore` (save slots, through Velvet Desktop
  or the browser).
- `@velvet/ui`: `createStage` (a 1920 by 1080 surface, letterboxed, native pixels) and `createButton`.
- `@velvet/i18n`: `I18n` with Spanish as the source and English as the fallback, `t` and `plural`.

Add the engine's other packages (cards, guion, characters, assets) the same way: a `link:` line in
`package.json` pointing at `packages/<name>`, then `pnpm install`.
