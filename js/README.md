# Velvet JS

The JavaScript and TypeScript side of Velvet: the packages that games import, and the
examples that use them. Games run inside **Velvet Desktop**, a small Rust host with the
system webview (see `docs/architecture/JS_FRAMEWORK_PLAN.md`). A browser build is only a
development convenience.

This workspace is **private**. Nothing here is published to npm yet; games link the
packages by path.

## Packages

| Package | What it is |
|---|---|
| `packages/core` (`@velvet/core`) | Fixed-step loop, game clock, typed events, input actions, the Velvet Desktop bridge (`window.velvet`) and versioned save slots. |
| `packages/ui` (`@velvet/ui`) | The 16:9 letterboxed stage (logical 1920×1080, exact pointer conversion, native pixels) and DOM/SVG HUD primitives. |
| `packages/i18n` (`@velvet/i18n`) | Locale catalogs in the `velvet-locale` format (`docs/formats/LOCALE.md`): fallback, templates, plurals. |
| `packages/wasm` (`@velvet/wasm`) | The Rust crates that must match in both runtimes (`velvet-guion`, `velvet-stacks`), compiled to WebAssembly by `crates/velvet-wasm`. |
| `packages/guion` (`@velvet/guion`) | Guiones (Markdown dialogue, `docs/formats/GUION.md`) played by velvet-guion through WebAssembly. |
| `packages/cards` (`@velvet/cards`) | Cards on a board: stacks, recipes and timed jobs from velvet-stacks (`docs/formats/STACKS.md`). |
| `packages/assets` (`@velvet/assets`) | Art by id: the `velvet-art` manifest, files in AVIF, WebP, PNG or SVG, generated placeholders, preloading and the missing-art report (`docs/formats/ART_MANIFEST.md`). |
| `packages/characters` (`@velvet/characters`) | Characters: stats with formulas, needs with thresholds, professions with experience curves, affinity between characters, traits and saves (`docs/formats/CHARACTER.md`). |
| `packages/artprep` (`@velvet/artprep`) | Tooling, not runtime: prompts in three layers (style, identity, variation) for each generation profile, jobs for an external batch runner, and the selection gallery (`docs/formats/ART_PREP.md`). |
| `packages/tween` (`@velvet/tween`) | Motion: easings (CSS `cubicBezier` and `steps`), tweens of numbers, arrays, objects and OKLab colours, timelines with labels and seek, springs, and DOM transform and opacity helpers, on tickers that follow the game clock or run in real time (`packages/tween/README.md`). |
| `packages/audio` (`@velvet/audio`) | Sounds by id: the `velvet-audio` manifest (files in preference order, buses, volumes, loops, voice limits, cooldowns, variants), Web Audio buses with a dB volume curve, the music channel with crossfades, ducking, the pause and the browser's autoplay rule, saved mix (`docs/formats/AUDIO.md`). |
| `packages/devtools` (`@velvet/devtools`) | Developer tools for dev builds only: an F3 overlay with frame times and memory numbers, game time controls, an event log, a state inspector, a console with typed commands and watches (`packages/devtools/README.md`). |
| `packages/tools` (`@velvet/tools`) | The command-line tools, run as `pnpm velvet-js`: `new` scaffolds a game from the template, `check` validates a game's data against every format and the links between its files, `i18n` reports each language's missing, unused and untranslated keys and writes a pseudo-locale for layout tests (`packages/tools/README.md`). |

Packages are TypeScript **source**. Each `exports` entry points at `src/index.ts`, which Vite
consumes directly, so there is no build step for them. The one built part is the WebAssembly in
`packages/wasm/gen/` (git-ignored): `pnpm test` and `pnpm typecheck` build it first when it is
missing or older than the Rust sources. That needs Rust with the `wasm32-unknown-unknown` target
(`rustup target add wasm32-unknown-unknown`) and `cargo install wasm-bindgen-cli --version 0.2.126`.

## Examples

| Example | What it shows | Run it | Tutorial |
|---|---|---|---|
| `examples/hello-cards` | A cards-on-a-board game in the style of Stacklands: the card rules, the board view, drags, the guide's conversation, saves, and the developer tools. | `pnpm --filter hello-cards dev` (port 5173) | [03](../docs/tutorials/js/03_cards_on_a_board.md) |
| `examples/hello-vn` | A short visual novel: a screen stack, a guion in the dialogue box, save slots, settings, two languages, art and sound by id, motion, and the developer tools. | `pnpm --filter hello-vn dev` (port 5174) | [02](../docs/tutorials/js/02_visual_novel.md) |

The tutorials for the whole workspace are in `docs/tutorials/js/`, starting with [01: your first game](../docs/tutorials/js/01_first_game.md).

## Commands

    pnpm install      # once, and after changing a package.json
    pnpm test         # Vitest over packages/*/src and examples/*/src
    pnpm typecheck    # tsc --noEmit over the same folders
    pnpm build:wasm   # rebuild the WebAssembly now

Run them from `js/`. Developed with Node.js 25 and pnpm 11.

## Layout

    js/
      packages/             libraries (private, source only)
      examples/             small games that use them: hello-cards and hello-vn (see Examples)
      tsconfig.base.json    shared compiler options: strict, ES2022, bundler resolution
      tsconfig.json         typecheck entry point for all packages and examples
      vitest.config.ts      test discovery
      pnpm-workspace.yaml   packages/* and examples/*

## Versions

- `typescript` 7.0.2 and `vitest` 4.1.12, pinned exactly.
- Vitest 5.0.5 is the newest release, but its `engines` field does not list Node 25, so the
  workspace stays on 4.1.12 until that changes.

## Adding a package

Create `packages/<name>/` with a `package.json` (`"private": true`, and `"exports"` pointing
at `src/index.ts`) and a `src/` folder. Name its tests `*.test.ts`. The root test and
typecheck scripts pick the package up without other changes.
