# JS tutorial 05: release

`velvet build --runtime js` packages a JavaScript game folder with the Velvet Desktop host, into a folder that you can
copy and run. This tutorial covers what that command does today. The packaging is still being extended, so run
`--help` for the options of the build you have.

## Build a release

From the engine's repository root (the `cargo run` runs the `velvet-cli` crate, which is the `velvet` command):

    cargo run -p velvet-cli -- build --runtime js --path js/examples/hello-cards

It printed these lines when it was run for this tutorial:

    release folder: C:\dev\velvet-engine\js\examples\hello-cards\release\hello-cards-0.1.0
      16 files, 1.94 MiB
      run with no arguments: C:\dev\velvet-engine\js\examples\hello-cards\release\hello-cards-0.1.0\hello-cards.exe

The release is written to a staging folder beside its final place and moved in at the end. On Windows a folder that
another program watches cannot be renamed, and a running dev server (`pnpm dev`) watches the whole game folder. The
examples and the `velvet-js new` template tell Vite not to watch `release/`, and when a move is still refused the build
copies the folder into place instead, so you can build while the dev server runs.

## What it does

1. If the game's `package.json` has a `build` script, it runs `pnpm run build` in the folder and packages `dist/`.
   Otherwise it packages the folder itself. hello-cards has a `build` script (`vite build`).
2. It builds the host with `cargo build -p velvet-desktop --release`, unless you pass `--host <exe>` with a host that
   you built already.
3. It writes the release folder `release/<id>-<version>/` in the game folder. The folder holds the host, named
   `<id>.exe` (no extension on Linux and macOS), and `game/`, which has the web files and `velvet-game.json`.
4. The copy leaves out `node_modules`, `.git`, the `release` folder at the top, and source maps (`.map`) unless you
   pass `--sourcemaps`.

The current build also writes `build-info.json` and `SHA256SUMS` next to the host. The folder of the run above had:

    hello-cards.exe
    build-info.json
    SHA256SUMS
    game/velvet-game.json
    game/index.html
    game/assets/...
    game/data/...

The release folder is in `.gitignore` (`/js/examples/*/release/`), so it is not committed.

## Options

These are the options that `velvet build --help` listed when this tutorial was written:

| Option | What it does |
|---|---|
| `--path <PATH>` | the game folder (for `--runtime js`) |
| `--runtime js` | packages a JavaScript game; without it, the command builds the Rust engine |
| `--host <PATH>` | copies a prebuilt `velvet-desktop` executable instead of building the host |
| `--sourcemaps` | also copies the source maps into `game/` |
| `--force` | replaces a release folder that already exists |
| `--zip` | also writes the release as a zip archive |

## Run a release

The host with no arguments runs the `game/` folder next to it. From a terminal, in the release folder:

    .\hello-cards.exe

The window opens the packaged game. The game's data folder and its log file come from the manifest's `id`
(`velvet-desktop.log` in the data folder, on Windows).

## The web files alone

To test the web files without the host, serve the built folder with any static server. The example's own build
writes it, from `js/`:

    pnpm --filter hello-cards build        # examples/hello-cards/dist

## Next

Start the next game with [JS tutorial 01: your first game](01_first_game.md), or read the examples in
`js/examples/`.
