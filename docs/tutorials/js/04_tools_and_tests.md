# JS tutorial 04: tools and tests

The JS workspace has a command-line tool, `velvet-js`, that checks a game's data and its texts, a developer overlay for
dev builds, and a testing package that runs a game by hand, records a session and plays it back. This tutorial shows
each one with the examples. Run the commands from `js/`.

## The command-line tool

    pnpm velvet-js --help

    Commands:
      new      create a new game folder from the template (package.json, Vite, a title screen, locales, a test)
      check    check a game's data against every format, and the links between its files
      i18n     report each language's missing, unused and untranslated keys; --pseudo writes a test language

Every command takes `--help`, `--json` for machine output, and exits with 0 on success and 1 when there is an
error. Warnings never fail a command.

### check

`check` reads a game folder's data with the package that owns each format: stacks catalogs, guiones, art
manifests, locale catalogs and the rest. Then it checks the links between the files, such as an art id that a
sheet names. Run it on the examples:

    pnpm velvet-js check examples/hello-cards
    pnpm velvet-js check examples/hello-vn

    hello-cards: ok, 0 errors, 0 warnings
    public/audio/manifest.json: (file): warning: velvet-audio documents of this kind are not checked
    hello-vn: ok, 0 errors, 1 warning

The warning is a gap in the tool, not in the example: `velvet-audio` documents are not read by `check` yet.

### i18n

`i18n` compares the languages of a game's locale folders. It reports three things for each language: the keys it
lacks (missing), the keys that no code uses (unused), and the texts that are the same as the source (untranslated).

    pnpm velvet-js i18n examples/hello-vn

    public/audio/manifest.json: (file): warning: velvet-audio documents of this kind are not checked
    en: 0 missing, 0 unused, 0 untranslated
    es (source): 0 missing, 0 unused, 0 untranslated
    hello-vn: ok, 0 errors, 1 warning

`hello-cards` has five warnings in the same report: two unused keys in each language, and one English text that is the
same as the Spanish. They are in its own catalogs, and the report lists each with its file and key.

`--pseudo <tag>` writes a test language for layout checks. Its texts are accented and longer than the source, and
they are in brackets, so a label that is too narrow shows at once. It is run on a copy here, so the example's own
folder is unchanged:

    pnpm velvet-js i18n <a copy of hello-vn> --pseudo en-XA

    wrote 65 texts to .../public/data/locales/en-XA/00_pseudo.json

The tool does not overwrite a folder that holds a real translation.

### new

`new` makes a game folder from the template. It is in [JS tutorial 01](01_first_game.md).

## The developer overlay

`@velvet/devtools` is an overlay for development builds. In hello-vn and hello-cards, the game imports it with
`import.meta.env.DEV` behind a dynamic import, so `pnpm --filter <example> build` leaves it out of the release files.
Press F3 in a dev build to show or hide it. It has:

- a performance view: the frame times, the FPS, the long frames, and the update and render split of the loop;
- time controls: pause, resume, a single step, and the clock's speeds;
- an event log of the game's event bus;
- state views: values the game registers with `inspect`, shown as trees that open on demand;
- a console, with typed commands, history and Tab completion. The built-in commands are `help`, `pause`, `resume`,
  `step`, `speed`, `clear` and `fps`;
- watches: numbers pinned to the corner of the screen, which stay there while the panel is hidden.

The examples add their own commands and watches. In hello-vn (`src/devtools.ts`), `escena faro` starts a scene of the
story, and the watches show the sea glass, the ending and the number of boxes read. In hello-cards, `coins 3` puts coin
cards on the table, and the watches count the cards and the coins.

The game wires it like this (from `examples/hello-cards/src/game.ts`, abridged):

    if (import.meta.env.DEV) {
      const { mountDevtools } = await import("./devtools");
      game.devtools = mountDevtools({ host, clock: game.clock, loop: game.loop, stage, board: () => game.board, ... });
    }

## Tests with the driver

The tests of the examples run a whole game without a browser loop. `@velvet/testing` provides the parts:

- `createDriver()` runs the frames by hand: `driver.step(n)` runs `n` frames, `driver.until(predicate)` runs until a
  condition holds, and `driver.dispose()` puts the page back the way it was. The game builds its loop with
  `driver.createLoop(options)`, so every frame is one update, and the clock is the same at every run.
- Keys go to the page the way the keyboard sends them: `driver.keyDown(code)`, then a frame, then `driver.keyUp(code)`.

The hello-vn test presses a key for one frame like this (`examples/hello-vn/src/game.test.ts`):

    function tap(driver: Driver, code: string): void {
      driver.keyDown(code);
      driver.step(1);
      driver.keyUp(code);
      driver.step(2);
    }

and starts the game on the driver with memory for its saves and settings, so each test starts clean:

    const game = await Game.start(host, {
      createLoop: (options) => driver.createLoop(options),
      sound: false,
      devtools: false,
      saveBackend: createMemoryBackend(),
      settingsBackend: memorySettings(),
    });

The driver also has `driver.play(script)` for an `InputScript` (keys, pointer and pad events at frame numbers), which
`@velvet/testing`'s README describes, and `runSoak` plays many seeds with a bot and reports the numbers.

## Replays

A replay is a recorded session: its inputs, its seeds and its state hashes at a fixed interval. It plays back on the
same game, and the first checkpoint that differs stops it. hello-vn records each playthrough and checks it:

    const recorder = createRecorder({ game: "hello-vn", checkEvery: 30, hash: () => game.hash() });
    recorder.attach(driver);
    // ... play the game with tap() ...
    const text = serializeReplay(recorder.toReplay());   // the replay, as JSON text

    const result = await checkReplay(parseReplayText(text), async ({ driver: replayDriver }) => {
      const started = await startOn(replayDriver, mountHost());
      return { hash: () => started.hash() };
    });
    expect(result.ok).toBe(true);

The game's `hash()` returns the same text for the same state every time: hello-vn sorts the object keys and writes a
short fingerprint. Keep the hash to the state that matters (the screen, the story and the stage), so a change that is
not a bug does not break the replay.

## Running the tests

    pnpm test                                       # every package and example, with Vitest
    pnpm typecheck                                  # tsc over the same folders
    pnpm exec vitest run examples/hello-vn          # one example
    pnpm exec vitest run examples/hello-cards

Tests that need a page start with `// @vitest-environment happy-dom` at the top of the file. Every game and driver that a
test makes should be stopped even when the test fails: the examples keep a list of stops and run it in `afterEach`. A
screen that is still listening to the keys would otherwise take the next test's keys.

## Next

Once a game works in dev, [JS tutorial 05: release](05_release.md) packages it.
