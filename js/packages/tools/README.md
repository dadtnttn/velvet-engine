# @velvet/tools

The command-line tools of Velvet JS, run as `velvet-js`. Three commands:

- `new` creates a game folder from the template: package.json with `link:` dependencies to the engine's
  packages, Vite, TypeScript and Vitest at the workspace's versions, a title screen, two locales and a test.
- `check` reads a game's data, validates each file with the package that owns its format, and checks the
  links between the files.
- `i18n` reports, for each language, the keys it lacks, the keys nothing uses and the texts it keeps from
  the source. With `--pseudo` it writes a test language for layout checks.

The package is a tool, not a runtime library. It is private, like the rest of the workspace.

## Running it

From `js/`:

    pnpm velvet-js --help
    pnpm velvet-js check examples/hello-cards
    pnpm velvet-js new ../games/my-game --id my-game --title "My Game"

Or directly: `node packages/tools/bin/velvet-js.mjs <command>`. Every command takes `--help`, `--json` for
machine output, and `-h` as a short form of `--help`. The exit code is 0 on success and 1 when the command
fails. For `check` and `i18n` that means an error in the game; warnings never fail a command.

### Why the bin registers a resolver hook

The packages are TypeScript source with no build step, and they import each other without file extensions
(`import { Locale } from "./locale"`), the bundler rule that Vite follows. Node 25 strips the types of a
`.ts` file on its own, but it does not add a missing extension. So `bin/velvet-js.mjs` calls
`module.registerHooks` (a Node API, so there is no dependency) before it loads `src/cli.ts`. The hook tries
the failed relative import again with `.ts`, and then with `/index.ts`. Nothing else changes, and the tools
need neither a build nor a loader flag.

## Commands

### `velvet-js new <dir> --id <id> --title <title> [--engine <js dir>] [--force]`

Writes the template in `templates/game/` into `<dir>`:

| File | What it holds |
|---|---|
| `velvet-game.json` | the manifest: id, title, version 0.1.0, entry `index.html`, a 1280 by 720 window |
| `package.json` | scripts (`dev`, `build`, `test`, `typecheck`, `check:data`); `@velvet/core`, `@velvet/i18n` and `@velvet/ui` as `link:` paths from `<dir>` to the engine's packages; devDependencies at the workspace's exact versions (TypeScript 7.0.2, Vitest 4.1.12, `@types/node` 25.9.9, and Vite 8.3.4 as hello-cards pins it) |
| `vite.config.ts` | `base: "./"`, `assetsInlineLimit: 0`, and `server.fs.allow` for the engine's js folder |
| `tsconfig.json`, `vitest.config.ts` | extend the engine's `tsconfig.base.json`; tests in `src/**/*.test.ts` |
| `index.html`, `src/main.ts`, `src/game.ts`, `src/style.css`, `src/env.d.ts` | a stage, a title screen, a turn counter, a save slot, the game loop and the i18n setup |
| `public/data/locales/es/00_ui.json`, `public/data/locales/en/00_ui.json` | the texts: Spanish is the source, English falls back to it |
| `src/game.test.ts` | a sample test of the rules |
| `README.md`, `.gitignore` | the next steps and the files; `_gitignore` in the template becomes `.gitignore` |

The engine folder defaults to the one this tool belongs to. `--engine` names another one, which must hold
`packages/core`, `packages/i18n`, `packages/ui` and `packages/tools`. The links are computed from `<dir>`, so
move the game only together with the engine, or run `new` again.

`<dir>` must be missing or empty. `--force` writes into a folder that is not empty and overwrites files of
the same names; it never deletes anything. The id follows the host's rule (1 to 64 lowercase letters, digits,
`-` or `_`, and not a Windows device name), and the title is at most 120 characters with no control characters.

When the tools were written, a scaffolded game was checked by hand: `tsc --noEmit` passes with the engine's
compiler, `vitest run` passes its three tests, and `vite build` writes `dist/`. The tests of this package
scaffold a game and run the data check on it.

### `velvet-js check <game dir>`

Finds the game's data and reads each file with the package that owns its format:

| Kind | Format and kind | Read by | Checked |
|---|---|---|---|
| stacks catalog | `velvet-stacks`, `catalog` | `@velvet/cards` (`loadCatalog`, through WebAssembly) | every velvet-stacks rule |
| stacks case | `velvet-stacks`, `case` | `check/cases.ts` (the steps of the shared fixture runner) | the output placement (`placement`, the row when it is left out); each step; expected events, drops, positions and the final table |
| stat schema | `velvet-character`, `schema` | `@velvet/characters` (`parseStatSchema`) | the schema's rules and formulas |
| character sheet | `velvet-character`, `character` | `@velvet/characters` (`parseCharacter`) against a schema | the sheet against the schema; it may be any schema of the game |
| art manifest | `velvet-art` | `@velvet/assets` (`parseArtManifest`) | every ART_MANIFEST rule |
| artprep | `velvet-artprep`: `profiles`, `style`, `identity`, `shots`, `results` | `@velvet/artprep` | each document's rules |
| guion | Markdown in a `guion` or `guiones` folder | `@velvet/guion` (`GuionLibrary.add`, through WebAssembly) | every problem with its line; two guiones with one id |
| locale catalogs | `velvet-locale`, one folder per language | `@velvet/i18n` (`Locale`, `validateCatalog`) | the catalog rules, holes, missing and unknown keys against the source |

The links are checked too:

- an art id named by a sheet (`art.portrait`, `art.fullbody`) or a shot (`art`, `references`, `edit.base`)
  must be in an art manifest, and a variant must be declared by its entry;
- a shot's `characters` must have an identity document;
- an id key in a data field (`name`, `label`, `title`, `reason`, in the catalogs, schemas and sheets) must
  have an entry in the source language. A text key (one with spaces or capitals, such as `Hola`) needs none.

A case runs on the game's catalog: the one in its own folder, or the only catalog of the game. Otherwise it
is not run, with a warning. Board saves (`board`), character states (`state`) and artprep jobs (`job`) are
not checked, because they are outputs of the game and of the tools, not inputs.

Output, one line per problem, then a summary:

    public/data/locales/en: entries["hud.day"]: error: en "hud.day" uses {date} where its source uses {day}
    checked hello-cards: 1 catalog, 0 cases, 0 stat schemas, 0 character sheets, 0 art manifests, 0 artprep documents, 2 guiones, 4 locale files in 2 languages
    hello-cards: ok, 0 errors, 0 warnings

Each problem line is `file: path: severity: message`. The path is a JSON path into the file
(`cards[2].name`, `entries["hud.day"]`), `line N` for a guion, or `(file)` for the whole file. `--json`
prints the same problems with their `code`, the counts and the verdict.

### `velvet-js i18n <game dir> [--pseudo <tag>]`

For each language folder, the report lists:

- **missing**: a key that the source language has, or a text that the code shows, and that this language has
  no entry for;
- **unused**: a key of this language that no string of the code, no call, no run-time prefix and no data
  field uses;
- **untranslated**: a text that equals the source text (its letters are compared, so `{count}` or `42` is
  not flagged).

The code scan reads every `.ts`, `.tsx`, `.js`, `.jsx`, `.mjs`, `.cjs`, `.mts`, `.cts` and `.html` file of the
game, except tests (`*.test.*`, `*.spec.*`) and `.d.ts` files. It finds:

- `t("key")`, `plural("key", n)` and `lookup("key")` with a literal key (the first argument);
- every quoted string, so a key held in a table or an object counts as used;
- keys that the code builds at run time: `` t(`recipe.${name}`) ``, `` { key: `done.${recipe}` } `` and
  `"pause." + mode` give the prefixes `recipe.`, `done.` and `pause.`, and every key that starts with one of
  them counts as used.

A key that the code asks for as an id (`card.wood`) and the source lacks is an **error**: the game would show
the id itself. Warnings do not fail the command.

`--pseudo <tag>` writes `<locales>/<tag>/00_pseudo.json` next to the source language, with every text of
the source and every text key of the code, and then exits. The tag must be a language tag in canonical case
(`en-XA` is one). See the next section for what the text looks like.

## The pseudo-locale

A pseudo-text is for layout tests, and it is never a translation:

- each ASCII letter is swapped for an accented look-alike (`Hello` becomes `Ĥéĺĺó`);
- the text grows by 35% of its source length, rounded up, with `~` at the end, so that a label too narrow
  for 35% more text shows at once;
- the whole text is in brackets: `[Ĥéĺĺó~~]`;
- holes (`{count}`), brace escapes (`{{`, `}}`) and plural forms are kept, so the file reads back with the
  same holes, and a text with no letter of its own (`{0} / {1}`, `42`) is left as it is.

The file carries the note `velvet-js pseudo-locale: ...`. A later `--pseudo` run replaces only a folder that
holds nothing else. A folder with a real catalog, or the tag of the source language, is refused, so a real
translation is never overwritten.

## Where the data is

The tools find data by content, not by where a file sits. The rules, in order:

1. **The manifest.** `velvet-game.json` must exist in the game folder. Its fields are read with the host's
   rules (`crates/velvet-desktop/src/manifest.rs`): `id`, `title`, `version`, `entry`, `window` and `icon`, each
   with the host's limits. The host refuses any other top-level key, so velvet-js reports it. Two more keys are
   read by the release step (`crates/velvet-cli/src/js_build_cmd.rs`) and not by the host: `release`
   (`exclude` patterns, `include` paths, `archive` of `none` or `zip`), checked with the same rules as the
   release. The icon must be a square PNG of at least 256 pixels (a smaller one is a warning), and every
   included file must exist in the game folder.
2. **The walk.** Every file under the game folder is visited, except the folders `node_modules`, `dist`,
   `release` and `coverage`, any hidden folder (its name starts with a dot), and the paths in the `ignore`
   hint. Symbolic links are not followed.
3. **Tooling files.** A file whose name starts with `_` is skipped (the locale format's rule, applied to all
   data).
4. **JSON documents.** A JSON file is a document when its `format` starts with `velvet-`. Its `kind` (for the
   formats that have one) says which reader checks it. JSON without a `format` (`package.json`,
   `tsconfig.json`, the manifest) is not data and is not read. A `velvet-` document of a kind that the tools
   do not read is a warning.
5. **Broken JSON.** A file that is not valid JSON is an error when a folder on its path is a data folder
   (`data`, `locales`, `art`, `characters`, `artprep`, `stacks`, `cases`, `guion`, `guiones`), and it is
   ignored elsewhere.
6. **Byte order marks.** A file that starts with a BOM is an error (the readers refuse it); the check reads it
   without the BOM, so the rest of its problems still show.
7. **Guiones.** A Markdown file is a guion only inside a folder named `guion` or `guiones`, at any depth, or
   inside a `guion` hint. A README outside those folders is not read.
8. **Locales.** A `velvet-locale` file belongs to the language of its folder, which must be a tag in canonical
   case (`es`, `pt-BR`). The source language is the `source` hint when given; otherwise `es` when there is an
   `es` folder, otherwise the first language folder in name order.

Hints, in `velvet-game.json`, under `data`. Every path is relative to the game folder, with forward slashes,
and never leaves it:

    "data": {
      "roots": ["public/data"],
      "guion": ["public/data/story"],
      "ignore": ["public/data/old"],
      "source": "es"
    }

- `roots`: the folders to search, instead of the whole game folder;
- `guion`: folders or files that hold guiones, besides the folders named `guion` or `guiones`;
- `ignore`: folders or files to leave out;
- `source`: the source language's tag.

**The host accepts the hints and ignores them.** Velvet Desktop reads `velvet-game.json` with
`deny_unknown_fields` (`crates/velvet-desktop/src/manifest.rs`), but `data` is a known, optional field that
only velvet-js reads. Without hints, the conventions above are all a game needs: `public/data/` with
`catalog.json`, `locales/`, `guion/`, and so on.

## What is not checked

- The key scan reads strings, not the syntax tree. A key that the code takes from a variable (`t(name)`) is
  invisible to it, so a key can be reported unused while the game uses it. A quote inside a comment can hide
  the code after it on the same line. A call to some other function named `t` with a dotted literal would be
  read as a key request, and reported as missing from the source if it is not one.
- A guion is read with the default cast. The tools do not read a game's cast yet, so the speaker and
  portrait names that a cast defines are not checked.
- Format readers stop at the first problem of a character sheet or a stat schema (their errors carry one
  message); an art manifest and an artprep document list every problem.
- `check` does not run a game, and it does not render anything.

## Tests

From `js/`, `pnpm test` runs them with the rest of the workspace: `packages/tools/src/*.test.ts`, the
`game/`, `check/`, `i18n/` and `new/` folders beside their code. They use temporary folders (removed after
each test), the example `examples/hello-cards` (which must pass `check`), the shared stacks fixtures
(`docs/formats/fixtures/stacks`, every case of which must pass the case runner), and the characters and artprep
fixtures, which are copied into the games that the tests build. One test runs the bin as a program, so the
hook is exercised the way `pnpm velvet-js` runs it.

Files of this package: `bin/velvet-js.mjs` (entry and resolver hook), `src/cli.ts` (the commands and the
exit codes), `src/command.ts` (argument parsing and help), `src/game/` (manifest, discovery, locales, keys),
`src/check/` (documents, cross-checks, cases, the command), `src/i18n/` (scan, coverage, pseudo, the command),
`src/new/` (scaffold and the command), `templates/game/` (the template).
