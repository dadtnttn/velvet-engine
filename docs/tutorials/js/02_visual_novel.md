# JS tutorial 02: a visual novel

This tutorial walks through `js/examples/hello-vn`, a short visual novel that uses the general-purpose
packages: a screen stack, a guion in the dialogue box, save slots, settings, two languages, art and sound by
id, motion, and the developer tools. Read `tutorials/js/01_first_game.md` first, or know how a game folder is laid
out.

## Run it

From `js/`, these commands are the ones the example's README lists and that were run while it was written:

    pnpm --filter hello-vn dev          # the page on http://localhost:5174
    pnpm --filter hello-vn build        # the web files, in examples/hello-vn/dist
    pnpm velvet-js check examples/hello-vn
    pnpm velvet-js i18n examples/hello-vn
    pnpm exec vitest run examples/hello-vn

To play in Velvet Desktop, keep the dev server running and, from the repository root, open the host on it:

    cargo run -p velvet-desktop -- js/examples/hello-vn --dev http://localhost:5174

`pnpm velvet-js check` reports one warning: it does not read `velvet-audio` documents yet. The audio manifest
follows `docs/formats/AUDIO.md`, so the warning goes away when the tool can read the format.

## The story is a guion

The story is in `public/data/guion/faro.md` (Spanish) and `faro.en.md` (English). A guion is Markdown, one `##`
scene per heading, one box per line. Here is the start of the first scene:

    ## puerto
    @fondo bg.puerto
    @musica music.pad
    Las farolas de Brisa Baja se han apagado una a una. ...
    **Tomé (preocupado):** ¡Por fin! ¿Eres la cartógrafa que mandó el gremio? ...
    ? ¿Qué haces?
    - Subir al faro con Tomé, ahora mismo -> faro
    - Pasar antes por la taberna a por el vidrio -> taberna

- `@fondo` and `@musica` are stage directives. The game reads them as it passes the boxes: `bg.puerto` is a
  background id and `music.pad` a sound id.
- `**Tomé (preocupado):**` is a line spoken by Tomé, with the mood `preocupado`. The mood picks the portrait
  variant.
- `? ...` opens a choice, and each `-> target` is an option. `[lente]` after an option's text closes it until the
  variable `lente` is set, and `{final = 1}` sets a variable when the option is taken.
- `@var lente` and `@salta faro` set a variable and jump to another scene of the same file.

The rules are in `docs/formats/GUION.md`. The two files have the same scenes, speakers, moods, directives and options, which
`content.test.ts` checks, because the language switch depends on it (see below).

## The story session

`src/story.ts` holds one conversation at a time. `Story` implements the dialogue session of `@velvet/ui`, so the
dialogue box plays it directly. It also keeps a record of the play: the scene the conversation started in, the
variables at that point, and the list of box actions since (`next`, or `choose` with an option index).

- A save is that record. `Story.restore` replays the actions on a fresh conversation, which gives the same box and
  the same variables (`story.test.ts` checks it).
- A language switch is the same replay on the other language's file, so the player stays on the same box. The
  game does it in `paintLanguage` when the language changes.
- The stage (background and music) is read from the directives of each box, replays included, so a restored play
  shows the right background and music.

## The dialogue box and the keys

`createDialogueBox` in `src/game.ts` shows the lines: the typewriter runs on the game's clock, so the text stops
while the game is paused. The box takes these options: `speed` and `autoDelaySeconds` (from the settings),
`portrait` (the speaker's picture), `onLetter` (a blip for each letter), and `onEnd` (the ending card).

| Input | What it does |
|---|---|
| Enter, Space, a click on the text | goes on past the line |
| Arrows and Enter on a choice, or keys 1 to 4 | takes an option (a closed option is ignored) |
| Hold F | fast-forward |
| Escape, or a pad's Start | opens the menu |
| Escape in the log | closes the log |
| A pad's A and B | Enter and Escape (`startPadMenuKeys` from `@velvet/core`) |

The keys of the story work only on the game screen, so a menu on top keeps its own keys.

## Screens

`src/screens.ts` builds the menus, and `src/game.ts` puts them on a `createScreenStack`:

- `title` is the bottom screen: new game, load, settings.
- `game` is a screen (it hides what is under it). Escape pushes the pause menu over it.
- `pause` (continue, save, load, settings, back to the title), `settings`, `saves` and `ending` are overlays. They
  sit over the game under a scrim.
- The game's `onPause` and `onResume` hooks stop and start the clock, so the text and the music stop behind a
  menu. The ending card cannot be closed with Escape; its button goes back to the title.

Screens are faded in with `tween` from `@velvet/tween`.

## Saves

The save list is `createSaveSlots` from `@velvet/ui`, over `createSaveStore` from `@velvet/core`:

- The autosave row `auto` is written at each new scene. Three manual slots are written from the pause menu.
- What is written is the record of the story (`src/save-data.ts`). `parseSave` checks its shape when it is read
  back, and the guion checks the actions when they are replayed, so a damaged save is refused and the game keeps
  running.
- The save store keeps the document in the bridge (Velvet Desktop), in localStorage (a browser), or in memory.

## Settings

`src/settings.ts` makes a `createSettingsStore` under the namespace `hello-vn`. It keeps the text speed, the wait
of Auto, the music volume, the sound-effect volume and the language. The settings menu cycles each value with
Enter or a click; the dialogue box and the sound follow at once. A saved value of the wrong type or out of range
is replaced by its default, so a damaged file does not stop the game.

## Languages

`src/texts.ts` lists every text id that the game uses (`TEXT_KEYS`), including the ids that the save list, the
dialogue box and the notices look up themselves. The catalogs are in `public/data/locales/es` (the source) and
`en`. `pnpm velvet-js i18n examples/hello-vn` reports the missing, unused and untranslated keys: the example has
none. The places (`scene.puerto` and so on) and the endings are in `10_story.json`.

## Art by id

`public/art/manifest.json` declares the backgrounds, the title and the portraits. The example has no image files:
every id shows its generated placeholder, which is the intended look of provisional art. `src/art.ts` asks
`@velvet/assets` for a picture by id, and for a portrait by speaker and mood: `portrait.tome` with the variant
`preocupado`. A mood that the entry does not declare shows the base portrait.

The background crossfades when the place changes, and a portrait slides in when its line appears. Both are
tweens on the game's real-time ticker (`src/stage-view.ts` and `portraitOf` in `src/game.ts`).

## Sound

`tools/make-sounds.mjs` writes three 16-bit WAV files into `public/audio`: a text blip, a select click and a soft
pad that loops. It needs no packages:

    node tools/make-sounds.mjs

The generated files are kept in the repository. `public/audio/manifest.json` names them with their bus and their
loop, and `createAudio` plays them: the blip once per letter (the engine's cooldown keeps it quiet), the click on
each choice, and the pad as music that crossfades in when `@musica` names it. The engine follows the game clock, so
the music stops while the menu is open.

## Developer tools

In a development build (`import.meta.env.DEV`), `src/game.ts` loads `src/devtools.ts` with a dynamic import, and
F3 shows the overlay. It has three watches (the sea glass, the ending and the number of boxes), a state view of
the story, and a console command: `escena faro` starts the lighthouse scene, with the variables that are there. A
release build does not contain the module: `pnpm --filter hello-vn build` produces no devtools file.

## Tests

`pnpm exec vitest run examples/hello-vn` runs five files:

- `content.test.ts`: the texts of both languages, the guiones, the art and the sound files.
- `story.test.ts`: the two routes to the endings, the closed option, the stage, the record and the hooks.
- `save.test.ts` and `settings.test.ts`: the save round trip, and the settings' persistence.
- `game.test.ts`: the whole game in a page, with `@velvet/testing`. The playthrough to each ending is recorded and
  checked as a replay. [JS tutorial 04](04_tools_and_tests.md) explains the driver and the replays with it.

## Next

[JS tutorial 03: cards on a board](03_cards_on_a_board.md) shows a different kind of game with the same packages.
