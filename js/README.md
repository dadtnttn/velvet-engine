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

Packages are TypeScript **source**. Each `exports` entry points at `src/index.ts`, which Vite
consumes directly, so there is no build step for them. The one built part is the WebAssembly in
`packages/wasm/gen/` (git-ignored): `pnpm test` and `pnpm typecheck` build it first when it is
missing or older than the Rust sources. That needs Rust with the `wasm32-unknown-unknown` target
(`rustup target add wasm32-unknown-unknown`) and `cargo install wasm-bindgen-cli --version 0.2.126`.

## Commands

    pnpm install      # once, and after changing a package.json
    pnpm test         # Vitest over packages/*/src and examples/*/src
    pnpm typecheck    # tsc --noEmit over the same folders
    pnpm build:wasm   # rebuild the WebAssembly now

Run them from `js/`. Developed with Node.js 25 and pnpm 11.

## Layout

    js/
      packages/             libraries (private, source only)
      examples/             small games that use them (none yet)
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
