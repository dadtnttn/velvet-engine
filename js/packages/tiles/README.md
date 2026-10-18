# @velvet/tiles

Tile maps for games: load a Tiled map (JSON, orthogonal, finite or infinite), build a collision grid from its
tiles, move boxes across the grid without tunnelling, find paths with A*, and draw the visible tiles of a layer
to a 2D canvas. It has no dependencies. The file format, the rules of each part and the refusals are in
`docs/formats/TILEMAP.md`, with the conformance cases in `docs/formats/fixtures/tiles/`.

- **Loading** (`loadTileMap`): the map text or object, its embedded and external tilesets (through a loader
  callback), its layers with their effective visibility, opacity, offsets and parallax factors, each layer's own
  tint (kept as data), typed custom properties, object
  shapes, tile flips, and the tile properties, collision groups and animations. Compressed data (gzip and zlib)
  is decoded with the platform's `DecompressionStream`, so loading is asynchronous. A refused map reports every
  problem, each with a code and a path such as `layers[2].data`.
- **Collision** (`buildCollisionGrid`, `CollisionGrid`): solid cells from the tiles of the layers, from a
  property of the tiles, or from their collision shapes, with one-way cells.
- **Movement** (`moveBox`): a box moves along x, then y, one tile at a time, and stops flush against what it
  reaches. It reports the contacts on each side and the cells that stopped it.
- **Paths** (`PathSearch`, `findPath`): A* over the grid, in four or eight directions, with costs, a node limit,
  and the same path for the same input. A `PathSearch` can be kept and reused.
- **Drawing** (`drawTileLayer`): the visible tiles of one layer, with flips, animation and parallax (scrolled about
  the map's parallax origin, or one given), on a context that already has the camera transform.
- **Queries**: `findLayer`, `tileLayers`, `resolveGid` and `animatedLocalId`.

## Usage

    import {
      buildCollisionGrid, drawTileLayer, findLayer, loadTileMap, moveBox, PathSearch, tileLayers,
    } from "@velvet/tiles";

    // Load the map. An external tileset is fetched by the path the map gives, relative to the map's folder.
    const level = await loadTileMap(await (await fetch("maps/forest.tmj")).text(), {
      loadTileset: async (source) => (await fetch(`maps/${source}`)).json(),
    });

    // Collision: the tiles of the "walls" layer, the tiles with a true `solid` property, and their collision
    // shapes. A tile with a true `oneWay` property as well is a platform that is only solid from above.
    const grid = buildCollisionGrid(level, {
      layers: ["walls"],
      solidProperty: "solid",
      oneWayProperty: "oneWay",
      shapes: true,
    });

    // Move the player for this frame. The result says where it stopped and what it touched.
    const step = moveBox(grid, player, velocityX * dt, velocityY * dt);
    player = step.box;
    onGround = moveBox(grid, player, 0, 1).contacts.bottom; // a probe: is there ground under the box?

    // A path from one cell to another, with diagonal steps. Keep one search for the whole game.
    const search = new PathSearch();
    const route = search.find(grid, { x: 3, y: 4 }, { x: 20, y: 9 }, {
      diagonal: true,
      maxNodes: 4000,
      cost: (x, y) => (waterCells.has(`${x},${y}`) ? 5 : 0),
    });
    // route is null when there is no way, otherwise the cells from the start to the goal.

    // Drawing. The context already has the camera transform, and `view` is what the camera shows.
    const images = { forest: forestPicture, props: propsPicture }; // by tileset name
    for (const layer of tileLayers(level)) {
      drawTileLayer(ctx, level, layer, images, view, performance.now());
    }

    // A layer by its name, for example to read its properties.
    const spawns = findLayer(level, "spawns");

## Notes

- Positions are in pixels and y grows downward. Cells are in map cells, which are the map's tile size.
- Collision ignores layer offsets, parallax and tint: drawing shifts and scrolls a layer, but its cells keep their
  place in the grid.
- Shapes are approximated: an ellipse or a polygon counts as its bounding box. Slopes are not supported.
- The search never enters a cell outside the grid, and a one-way cell is walkable for it.
- Images are never loaded by this package. The caller gives the pictures to `drawTileLayer`, keyed by the
  tileset's name, and a visible tile whose picture is missing is an error.

## Tests

From `js/`, run `pnpm exec vitest run packages/tiles`. The tests cover each part, the conformance suite in
`docs/formats/fixtures/tiles/cases.json`, and drawing through a recording stand-in for the 2D context, which
checks where each flipped tile lands on the device.
