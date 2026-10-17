# ADR 0009 — Two runtimes, Rust and JavaScript, sharing their formats

## Status

**Proposed** (2026-10-16). Nothing is built yet; the plan is [JS_FRAMEWORK_PLAN.md](../architecture/JS_FRAMEWORK_PLAN.md).

## Context

- Velvet Engine is a Rust workspace; games ship as native binaries (Demon Queen is the flagship).
- Its 3D places are prototyped first in Three.js. Nine prototypes share `tools/velvet-three-kit`
  (toon shading, outlines, baking, textures, a glTF exporter), and every place is then ported to
  Rust by hand, through exported glTF and JSON.
- The team iterates fastest in JavaScript, and a browser build is the easiest way to show a game.
- Content formats already outlive any one runtime: guiones (Markdown dialogue), `.vcss`, locale
  catalogs, zone JSON, glTF, VLZ packs.

## Decision

1. Grow the three-kit into **Velvet JS**, a JavaScript framework that can make whole games (loop,
   input, assets, scenes, UI, dialogue, saves, localization), built on Three.js.
2. Keep **Velvet Engine** (Rust) as the reference runtime: native shipping, the Velvet Editor,
   heavy simulation.
3. The two share **formats, not code**: each shared format gets a specification in
   `docs/formats/` with fixtures both runtimes pass.
4. Logic that must give the same results in both (navigation, terrain, war rules) is written once
   in Rust and used from JS as **WebAssembly**.
5. Later, split them into two products with their own versions and releases, joined by a formats
   version (`velvet new --runtime rust|js`).

## Consequences

- New top-level `js/` pnpm workspace when phase 2 starts; `tools/velvet-three-kit` moves there.
- `docs/formats/` becomes the contract between runtimes (VLZ is already there).
- Every duplicated feature costs conformance fixtures; features without them stay in one runtime.
- One maintainer carries two runtimes: the JS side ports only what JS games use.
