# hello-vn

A short visual novel, *El faro de Brisa Baja* (The Lighthouse of Brisa Baja), that shows the parts of Velvet JS
that are not about cards: a screen stack, a guion in the dialogue box, save slots, settings, two languages, art by
id, sounds and motion, and the developer tools. It is public example code, with original light-hearted fantasy
content for adults. The story has three scenes, two choices and two endings: take the sea glass from the tavern
for the light ending, or ring the bell for the other one.

## Run it

From `js/`, start the web side (it serves the game on http://localhost:5174 and reloads on each save):

    pnpm --filter hello-vn dev

Open that address in a browser. To run it in Velvet Desktop, start the dev server as above and, from the
repository root, open the desktop host on it:

    cargo run -p velvet-desktop -- js/examples/hello-vn --dev http://localhost:5174

A release folder is made with `cargo run -p velvet-cli -- build --runtime js --path js/examples/hello-vn`.

Build the web files (`dist/`) and check the code and the data (from `js/`):

    pnpm --filter hello-vn build
    pnpm velvet-js check examples/hello-vn
    pnpm velvet-js i18n examples/hello-vn

The sounds are generated, and the generated files are kept in the repository. To write them again:

    node examples/hello-vn/tools/make-sounds.mjs

## Play

- Enter or Space goes on. Click the text to go on too. Hold F to fast-forward.
- The choices take the arrows and Enter, or the keys 1 to 4, or a click.
- Escape (or a pad's Start) opens the menu: continue, save, load, settings, and back to the title. A pad's B goes
  back, and its A is Enter.
- The Auto and Skip buttons, and Log (the backlog), are on the dialogue box. Escape closes the log.
- The settings change the text speed, the wait of Auto, the volume of the music and of the sound effects, and the
  language. The language can change in the middle of a scene: the same box is played in the other language.
- Saves are automatic at each new scene (the slot "Autoguardado"), and there are three manual slots.
- In a development build, F3 shows the developer tools. The console command `escena <scene>` jumps to a scene
  (`puerto`, `taberna` or `faro`), and the watches show the sea glass, the ending and the number of boxes.

## Files

    velvet-game.json           the manifest (id hello-vn, a 1280 by 720 window)
    index.html                 the page; the game mounts in #game
    vite.config.ts             base "./", and the .wasm stays a file
    tools/make-sounds.mjs      writes the three sounds into public/audio (no packages needed)
    public/data/guion/         the story: faro.md (es) and faro.en.md (en), in the guion format (docs/formats/GUION.md)
    public/data/locales/       the texts: es (the source) and en, in velvet-locale (docs/formats/LOCALE.md)
    public/art/manifest.json   the art by id (velvet-art, docs/formats/ART_MANIFEST.md); no image files yet
    public/audio/              the sounds (velvet-audio, docs/formats/AUDIO.md), and their manifest
    src/config.ts              every number and id a writer may change: the languages, the story files, the sounds
    src/story.ts               one conversation at a time: the dialogue session, the record of the play, the stage
    src/save-data.ts           the save document, and its check when it is read back
    src/settings.ts            the settings: text speed, auto wait, volumes and language
    src/art.ts                 the art by id, with the portraits by speaker and mood
    src/stage-view.ts          the game screen: the background (crossfaded), the top bar and the key hints
    src/screens.ts             the menus: title, pause, settings, save list and the ending card
    src/game.ts                the wiring: the loop, the keys, the stack, the dialogue, the sound and the saves
    src/devtools.ts            the developer tools (loaded only in development builds)
    src/texts.ts               the text ids that the game uses, and the loader of the catalogs
    src/*.test.ts              the tests (see below)

## Packages

- `@velvet/core`: the loop, the clock, the keys (`ActionMap`), the pad's menu keys, the save store and the settings
  store.
- `@velvet/ui`: the stage, the screen stack, the dialogue box, the save list, the notices and the menu buttons.
- `@velvet/guion`: the story (`GuionLibrary`, `Conversation`), through velvet-wasm.
- `@velvet/i18n`: the texts, with Spanish as the source and English as the fallback, and a plural.
- `@velvet/assets`: the art by id, with the generated placeholders that the game shows while it has no pictures.
- `@velvet/audio`: the sounds, the music with its crossfade, and the buses that follow the clock's pause.
- `@velvet/tween`: the fades of the screens and of the backgrounds, and the slide of a portrait.
- `@velvet/devtools`: the F3 overlay, in development builds only.
- `@velvet/testing` (a dev dependency): the driver and the replays of the tests.

## Tests

From `js/`, run the example's tests with the rest of the workspace (`pnpm test`), or on their own:

    pnpm exec vitest run examples/hello-vn

- `content.test.ts`: both languages have every text the game uses, and nothing else; both guiones parse without
  problems and have the same scenes, speakers, moods and directives; every art id, portrait mood and sound
  file that the story names exists.
- `story.test.ts`: the two routes to the two endings, the closed option, the stage directives, the record of the
  story and its replay, the language switch in the middle of a scene, and the hooks.
- `save.test.ts`: a save goes through the save store and comes back to the same box; a document that is not this
  game's is refused.
- `settings.test.ts`: the defaults, the persistence, and a damaged value that does not stop the game.
- `game.test.ts`: the whole game in a page, driven by `@velvet/testing`. A playthrough to each ending is recorded
  as a replay and checked against the state hashes; the pause menu, the settings, the language switch, the ending
  card and a save and load through the save list are checked too. Each test is in happy-dom, with no browser.

## Known limits

- The art is provisional: every id shows its placeholder, because the game ships no image files. A picture goes in
  `public/art/<kind>/<id>.webp` (or `.png`, `.svg`, `.avif`), and its path, such as `bg/bg.puerto.webp`, goes into
  the `files` argument of `createArt` in `src/game.ts`: the game only uses the files it is given.
- `pnpm velvet-js check` warns that `velvet-audio` documents are not checked yet. The manifest follows the format.
- The developer tools are not in a release build (the import is behind `import.meta.env.DEV`).
