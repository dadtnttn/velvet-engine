# Demon Queen: shared toon shading and dungeon art

## Reference-driven visual pass (2026-09-24)

The September 24 user reference is an art-direction target, not a background
texture. The following changes render live in both the shipped game and lab:

- `surfaces.rs`: staggered masonry courses in the forgotten ruins; natural
  Voronoi slabs elsewhere. Chipped corners, broad planar faces, lit bevels,
  vertex-painted cavities and mineral variation replace the folded-looking
  panels. Perimeter returns face inward: reversing these normals exposes seams
  under back-face culling. Tests cover that regression.
- Rooms, bridges, slopes and stairs have recessed flagstones with narrow mossy
  joints. Their relief is bounded to 5.5 cm **below** the original walking plane.
  Hanging moss roots are buried into the recessed ceiling; small rubble stays
  below 17 cm. Both are baked into the room chunk, not per-leaf draw calls.
- Wall cell size adapts to surface area, retaining the existing 12k-triangle
  per-room wall budget. The 48-chunk cache, portals and authored asset LODs remain.
- `scene.rs`: warm localized torch pools, cooler ambient depth, teal crystal
  landmarks, restrained flicker. The hand torch has a reusable carved handle /
  iron-basket mesh, a faceted flame and three small animated embers.
- `Material3D::with_emission`: textured self-lighting uses the previously unused
  positive range of `occlusion_window.x`, leaving the negative floor sentinel,
  AO and reveal payload unchanged. It does not imply light emission into the
  scene; separate point lights provide that. Renderer defaults remain unchanged.
- Dungeon-only toon settings are restored after drawing, so its softer lighting
  profile does not leak into other scenes sharing the renderer.
- `finish.rs`: bright-pass extraction and separable glow at quarter resolution,
  then one native-resolution composite with a subtle vignette. Resources persist
  between frames. HUD is drawn afterward and stays sharp. No expensive
  full-resolution blur, screen-space GI or inverted-hull outline pass.
- `hud_skin.rs`: native 1920x1080 petrol/gold frames, serif hierarchy, rotating
  compass, live seal/key/status, health, inventory counters, interaction prompt
  and framed controls. All ornament is vector geometry in the existing HUD batch;
  text remains a shared atlas. Existing diagnostic/seed screens scale uniformly.

`surface_styles.json` controls the stone/floor/moss palette without touching the
logical generator or save version. Geometry is still procedural, not destructible.
The authored GLBs and their source textures are preserved. Point lights still use
the forward renderer's unshadowed local-light path; per-torch shadow maps and
screen-space contact shadows are not implemented by this pass.

Verification for this pass:

- Built `demon-queen`, `demon-queen-3d` and `dungeon-lab`.
- Dungeon lab: 11 tests passed; renderer library: 72 tests passed.
- Scripted live-controller gameplay: seeds 42..61, 20 completions / zero failures.
- Real GPU captures: seed 42, compact seed 8675309, expedition seed 4815. The
  expedition swept all 46 rooms with no dropped visible chunks; cache stayed at
  48. Ordinary captures include oblique architecture, close walls, torch-off,
  ceiling, stair and debug views.
- Artifacts: `temp/dungeon-reference-final`, `temp/dungeon-reference-compact`,
  `temp/dungeon-reference-expedition`, `temp/dungeon-reference-replay.json`.
  `temp/dungeon-before-reference` preserves matching pre-pass screenshots.
- Timings in the capture JSON include CPU submission + synchronous GPU wait,
  not gameplay FPS. Concurrent unrelated GPU load was observed during this
  session, so these runs are not a controlled before/after performance benchmark.

Reproduce from repository root:

```powershell
cargo test -p demon-queen --bin dungeon-lab
cargo test -p velvet-render --lib
target/debug/dungeon-lab.exe --seed 42 --capture temp/dungeon-reference-final
target/debug/dungeon-lab.exe --seed 42 --replay --count 20 --report temp/dungeon-reference-replay.json
target/debug/dungeon-lab.exe --seed 8675309 --config crates/velvet-dungeon/data/compact.json --capture temp/dungeon-reference-compact
target/debug/dungeon-lab.exe --seed 4815 --config crates/velvet-dungeon/data/expedition.json --capture temp/dungeon-reference-expedition --sweep
```

## Runtime

- `crates/velvet-render/src/toon.rs`: four diffuse bands, softened thresholds,
  cool shadow tint, restrained highlights/rim. CPU previews use the same ramp.
- `mesh3d.rs`: the GPU world shader selects `ShadingStyle3D::Toon` without a
  second geometry pass. Other engine clients retain the default `Lit` style.
  Normal transforms use inverse transpose, including non-uniform art fitting.
- `demos/demon-queen/src/exploration3d.rs`: toon enabled for the entire world
  renderer: Queen/NPCs, castle, vegetation, villa and dungeon.
- `src/bin/shop_preview.rs`: toon lighting for every loaded item; original
  glass transparency, depth ordering and liquid containment/physics retained.
  Bitmap UI icons are not post-processed or regenerated.

This is a lighting style, not an automatic texture repaint. Existing realistic
terrain textures and authored baked shading remain visible. There is no costly
inverted-hull outline pass and no extra per-instance geometry duplication.

## Procedural cave surfaces (2026-09-23)

`src/dungeon/surfaces.rs` replaces the previous flat/noise-faceted shell with
seeded, anisotropic Voronoi stone plates. The same small mesher supplies natural
walls, more regular masonry, ceilings, sloping tunnels and secret stone doors.
Bevels, sparse fractures and moss lips are geometry/vertex colour, not additional
draw calls. `visual.rs` still builds the exact validated floor and stair treads;
its shared 256px pigment texture now has broad quiet variation instead of pores.

`surface_styles.json` owns stone/floor/moss palettes by biome ID. These are
render-only values: changing art does not change the generator catalog hash,
layout, loot, controller, saves or RNG streams. Unknown biome IDs fall back to
their own catalog palette. The mesher uses `hewn_surface_v1`; placement retains
its separate dressing stream.

Safety constraints:

- Relief recedes away from the walkable area. Socket apertures are subtracted
  before building faces; the roof never extends below the validated ceiling.
- Full-depth backing plus perimeter returns close seams. Do not restore a
  backing fan that tapers to the front boundary: that sheet intersects the rock
  faces and causes triangular visual cutouts (covered by a regression test).
- Tunnel shell strips are approximately 3.2m long, independent of the number of
  stair treads. The full-width lower 2.65m remains clear before the roof bevel.
- Authored wall formations are shallow relief with <=10cm inward projection,
  more sparsely placed. In bridge rooms they sit on the pit floor, not in midair.
  Ceiling caps extend behind the roof backing rather than floating below it.
- Secret passages use shared two-sided stone panels instead of plain cubes;
  opening them keeps the existing interaction and collision logic.

Meshes are generated only on chunk cache misses, not every frame. Existing
portal traversal, 48-chunk LRU and shared asset LODs remain in use. No voxel data,
mining, destructible walls or replacement hand tool is implemented in this pass.

Surface QA commands (repository root):

```powershell
target/debug/dungeon-lab.exe --seed 42 --capture temp/cavern-style/final-42
target/debug/dungeon-lab.exe --seed 8675309 --config crates/velvet-dungeon/data/compact.json --capture temp/cavern-style/compact
target/debug/dungeon-lab.exe --seed 4815 --config crates/velvet-dungeon/data/expedition.json --capture temp/cavern-style/expedition --sweep
target/debug/dungeon-lab.exe --seed 0 --replay --count 100 --report temp/cavern-style/replay-100.json
```

Regular captures now include close wall, torch-off, ceiling and sloping-stair
views in addition to the room shots. GPU QA: 7 normal + 7 compact + all 46
expedition room views, no dropped visible chunks. On RTX 5080 Laptop / DX12 /
1920x1080 MSAA4, warmed synchronous render+wait medians were 1.08–3.06ms across
these views. First-view generation/upload/render reached 25.06ms; this is not a
guarantee of frame rate on other hardware. Metrics live next to the screenshots.

Verification: game binary 33 tests passed (one unrelated explicit GPU test
ignored), dungeon lab 7 passed, generator 10 passed, live-controller replay 100
seeds completed / 0 failures. Surface tests check deterministic meshes, finite
unit normals, triangle budgets, clear sockets, outward-only walls, roof
headroom, sloping tunnel boundaries and unchanged blueprint serialization.

## Seven supplied assets

`src/dungeon/assets.json` maps semantic IDs to local GLBs and optional LODs.
Models are content, never dependencies of `velvet-dungeon` generation.

| ID | Use | High / low triangles |
| --- | --- | --- |
| cave_wall_rock | Recessed cave-wall formations | 5427 / 1600 |
| cave_stalagmites | Wall-foot formations | 4158 / 1200 |
| cave_stalactites | Ceiling groups above jump headroom | 4089 / 1100 |
| ruins_archway | Socket framing | 3538 / 1499 |
| azure_crystal_cluster | Biome landmarks and rest markers | 3601 / 1300 |
| ancient_runestone | Entrance, puzzle and memory markers | 3892 / 1399 |
| ember_sanctum_altar | Final extraction altar | 4181 / 1800 |

`src/dungeon/dressing.rs` caches deterministic placements per room with an
independent `tripo_dressing_v1` stream. Door landings remain clear; wall relief
stays predominantly behind the navigation wall and ceiling bottoms stay above
3.2 m. The original closed shell remains behind the art. Interactive markers
retain the existing interaction/movement semantics; these are not mesh colliders.

`scene.rs` submits only portal-visible regions. Detail stops beyond 48 m;
arches survive to 75 m; low LOD starts at 24 m. Geometry handles are shared,
textures are content-deduplicated across LODs, and chunk storage remains bounded
to 48. This is shared-resource drawing, not a new hardware-instancing system.

Both LODs use the high mesh's normalization, so switching LOD does not change
the pivot/size. The actual high/low arch triangles are tested for a full-width
2.65 m opening, including the jumping player. Without the complete optional art
kit, primitive shell decorations/door frames remain available.

## Offline reimport

Original files in Downloads are untouched. Blender normalizes feet to Y=0,
preserves UVs, caps textures at 1K/2K, writes decimated LODs and a source/hash/
triangle/texture report. No Tripo API call or generation credit is required.

From the repository root (PowerShell):

```powershell
& 'C:\Program Files\Blender Foundation\Blender 5.2\blender.exe' --background --factory-startup --python demos/demon-queen/tools/import_dungeon_art.py -- 'C:\Users\usuario\Downloads\Nueva carpeta (3)'
```

Outputs: `demos/demon-queen/data/assets/3d/dungeon_tripo/`. Media remain local/
ignored, like the other demo packs. Ship that folder with the game; the portable
copy script now includes it. No portable release was built by this art pass.

## Reproducible QA

```powershell
cargo test -p velvet-render --lib
cargo test -p velvet-dungeon
cargo test -p demon-queen --bin demon-queen-3d
cargo build -p demon-queen --bin demon-queen --bin demon-queen-3d --bin dungeon-lab
target/debug/dungeon-lab.exe --art-gallery temp/toon-review/gallery
target/debug/dungeon-lab.exe --seed 42 --capture temp/toon-review/after
target/debug/dungeon-lab.exe --seed 0 --replay --count 100 --report temp/toon-review/replay-100.json
target/debug/demon-queen-3d.exe --capture-toon temp/toon-review/worlds
```

`--capture-toon` renders castle, day/night villa and four shop products through
the production frame function, without a window or writing a player save.
The Windows GUI executable may require `Start-Process -Wait` when scripting.
Gallery captures compare Lit/Toon on each supplied model. Dungeon captures
record synchronous CPU-submit + GPU-completion timings, not gameplay FPS.

Verified on RTX 5080 Laptop, DX12, 1920x1080, MSAA4: initial six seed-42 views
were roughly 1.4–2.4 ms median warm render+wait, 58K–300K static/imported
triangles, at most 40 resident chunks. This is not a low-end-hardware guarantee.

Remaining art work: authored enemy meshes, texture/palette unification of older
packs, a better tunnel/arch interior transition and per-biome art variants.
The generator, progression, save version and original seeds are unchanged.
