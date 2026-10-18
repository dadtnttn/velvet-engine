# Velvet JS — plan for a JavaScript game framework

**Status:** proposed (2026-10-16), not started; decisions of 2026-10-18 below (no npm, desktop
only, shared rules through WebAssembly). Decision record: [ADR 0009](../adr/0009-rust-and-js-runtimes.md).

Velvet Engine is a Rust engine. Most of its 3D work, though, starts in JavaScript: every Demon Queen
place (castle, village, forest, baths, brothel, tavern, caves, el Peñascal, the war) was first built
as a Three.js prototype, and `tools/velvet-three-kit` already holds what those prototypes share
(toon shading, ink outlines, baking, painted textures, the glTF exporter). Iterating there is fast;
moving each result into Rust is a second, manual port.

This plan turns that kit into a framework for making games in JavaScript, and later splits Velvet
into two engines that share their formats: **Velvet Engine** (Rust: native, shipping performance)
and **Velvet JS** (JavaScript: browser, desktop shells, fast iteration).

## Goals

1. A game can be **made in JS**, not only prototyped: loop, input, assets, scenes, UI, dialogue,
   saves and localization, with no Rust toolchain needed.
2. **Same content, two runtimes.** Guiones (Markdown dialogue), `.vcss` styles, locale catalogs,
   scene/zone JSON, glTF models and VLZ packs mean the same thing in both. A place built in JS can
   still be exported to a Rust game, as today.
3. **One truth for heavy logic.** Simulation that must match (navigation, terrain, the war's battle
   and campaign rules) is written once in Rust and reused from JS through WebAssembly.
4. **Honest parity.** Anything implemented twice has conformance fixtures that both runtimes pass.

## Non-goals

- Replacing the Rust engine. Native builds, the Velvet Editor and shipping games stay in Rust.
- A renderer-agnostic JS layer at the start: Velvet JS builds on Three.js (WebGL now, WebGPU when
  Three.js does).
- Porting every Rust crate. Only what a JS game needs, in the order games ask for it.

## Shape

```text
js/                          pnpm workspace (new top-level folder)
  packages/
    core/        loop, fixed step, events, input actions (mirrors velvet-input), saves
    three/       toon shading, outline, bake, textures, geo — grown from tools/velvet-three-kit
    assets/      glTF, textures, VLZ reader, prefetch
    ui/          HUD primitives in the Persona style the games use (slanted plates, keycaps, chips)
    guion/       guion parser and player (same Markdown as velvet-guion)
    i18n/        catalog loader with the same rules as the Rust one (files in name order, templates)
    export/      the exporter to Rust games (today's tools/velvet-three-kit/export)
    wasm/        Rust crates compiled to WebAssembly (nav, terrain, war rules) with JS bindings
  create-velvet/ project scaffolder (`pnpm create velvet my-game`)
  examples/      small games and the prototypes that become examples
docs/formats/    the shared formats, one specification each (VLZ is the first)
```

ES modules, TypeScript types (JSDoc first, so packages run without a build step), Vite for
development, Vitest for tests. The CI already installs pnpm and Node for the VS Code extension.

## Phases

| Phase | What | Done when |
|---|---|---|
| **0 — Plan** | This document and ADR 0009. The three-kit stays where it is and keeps serving the prototypes. | Agreed. |
| **1 — Formats** | Write the shared formats down in `docs/formats/`: guion, locale catalogs, zone/scene JSON, crowd animation parameters (beside VLZ). | Each format has a spec and fixtures. |
| **2 — Kit to packages** | Move `tools/velvet-three-kit` to `js/packages/three` and `js/packages/export`; types, tests, a CI job. The prototypes import the packages. | The prototypes build and export unchanged. |
| **3 — Core runtime** | `core` and `assets`: loop, input, assets, saves, scenes on Three.js. A small example game runs in the browser. | An example ships as a static site. |
| **4 — Content parity** | `guion`, `i18n`, `ui`: the same guion fixtures and locale catalogs give the same results in Rust and JS. | Conformance tests pass in both. |
| **5 — Shared logic** | `wasm`: `velvet-nav`, `velvet-terrain` and the war rules compiled to WebAssembly; JS games call them. | The battle and campaign sims match their Rust runs on recorded seeds. |
| **6 — The split** | Two products with their own versions, docs and releases: `velvet-engine` (crates) and `velvet-js` (npm). They share a formats version; `velvet new --runtime rust|js` picks one. | Both release from the same repository (or two, if that is simpler then). |

## Rules while both exist

- **Formats first.** A feature that writes or reads content defines its format in `docs/formats/`
  before it lands in either runtime.
- **No silent duplication.** Logic written twice ships with shared fixtures; logic that must match
  exactly goes through WebAssembly instead.
- **The Rust engine is not blocked by JS.** JS packages follow the formats; they never require a
  Rust change to land.
- **Same licence** (MIT OR Apache-2.0) and the same contribution rules.

## Decisions of 2026-10-18

- **No npm for now.** The packages live in the `js/` workspace; games link them by path. Publishing
  waits for the split (phase 6) and an API worth freezing.
- **Desktop only, for now.** JS games run inside **Velvet Desktop**, a small Rust host. There is no
  browser release; the browser stays a development convenience.
- **Rules that must match are Rust, used through WebAssembly**, from the first game on. The first
  test game is a card game: its stacks and recipes engine extends the `velvet-cards` crate, and
  `@velvet/cards` wraps its WebAssembly build. A native Rust build of the same game then shares the
  rules and data and only rewrites the front end.
- Phase 3's "an example ships as a static site" becomes "an example ships as a desktop folder".

### Velvet Desktop (new crate)

```text
JS game  ── uses ──►  @velvet/* packages (js/packages)
   │                        └─ @velvet/cards ── WebAssembly ──► velvet-cards (Rust)
   ▼
runs inside ► Velvet Desktop (.exe)
               ├─ window (tao/winit) + system webview (wry: WebView2 on Windows, WebKitGTK on Linux)
               ├─ the game served from a custom protocol (velvet://game/): no server, works offline
               └─ native bridge `window.velvet`: saves under the user's data folder, settings,
                  VLZ packs, window/fullscreen, quit, logging
```

- **Why a webview and not an embedded JS interpreter (QuickJS, V8) in the native renderer:** an
  interpreter needs JS bindings for the renderer, UI and input, and gives up the DOM, SVG and
  Three.js. A webview gives those for free, and on Windows it uses the WebView2 already installed,
  so the host stays small.
- **Development:** `pnpm dev` runs Vite, and the host loads the dev server's address with hot
  reload, inside the real window.
- **Release:** `velvet build --runtime js` (to add to the CLI) puts the host executable next to the
  built game (or the game packed as VLZ), like the native games' release folders.
- **Native games are unchanged.** Demon Queen stays fully native.

## Open questions

- ~~Publish to npm from the start?~~ No (2026-10-18): private until phase 6.
- TypeScript sources or JSDoc types only?
- Should the JS runtime read VLZ packs directly (a JS decoder) or only through WebAssembly?
- One maintainer carries both runtimes: which Rust crates are worth a WebAssembly build first?

## Risks

| Risk | Mitigation |
|---|---|
| The two runtimes drift apart | Shared fixtures in `docs/formats/`; WebAssembly for logic that must match |
| Double the maintenance | Port only what a JS game uses; keep Rust the reference |
| Three.js API churn | Pin versions per release; the kit already isolates most Three.js calls |
| WebAssembly size and start time | Build only the crates a game imports; lazy-load them |
