# Tile maps: the Tiled subset, collision, movement, paths and drawing

A game builds its level in the Tiled editor and saves it as JSON: a map (`.tmj`) and its tilesets (`.tsj`).
This document says which part of Tiled's JSON format the engine reads, what it refuses, and what each read
value means. It then says how a loaded map becomes collision, how boxes move across it, how a path is
found on it, and what drawing a layer does. The reference implementation is `js/packages/tiles`
(`@velvet/tiles`). The conformance cases live in `docs/formats/fixtures/tiles/`. Where this text and the code
disagree, the fixtures decide.

## Conventions

- Maps and tilesets are UTF-8 JSON, as Tiled 1.10 writes them. The reader follows the field names that Tiled
  writes (`tilewidth`, `firstgid`, `offsetx`, and so on). A field that the reader does not know is ignored.
- Positions and sizes are in **pixels**, and cells are counted in **map cells**. Y grows **downward**, as in
  Tiled and on a canvas.
- A **global id** (gid) is the number that a cell holds. 0 is an empty cell.
- A reader returns plain data. Tile cells are typed arrays (`Uint32Array` for ids, `Uint8Array` for flips).

## Loading

A map is loaded by `loadTileMap(input, options)`, which is asynchronous because compressed data is decoded
with the platform's `DecompressionStream`. The input is the JSON text, or the parsed object. An external
tileset (a map entry with a `source`) is fetched through the loader callback `loadTileset(source)`, which
receives the path as the map writes it and returns the parsed `.tsj` object. The loader resolves a relative
path against the map's folder. A map with an external tileset cannot be loaded without the callback.

A refused map gives one `TilesError` with every problem found, in the order of the document. Each problem has
a code and, when it is about a value in the document, a path (see [Validation](#validation)).

## The supported subset

| Part of Tiled's format | What the reader does |
|---|---|
| `orientation` | Must be `orthogonal`. Other orientations are refused. |
| `infinite` | Read. An infinite map's tile layers are made of `chunks`. |
| Tile layers | Read. `encoding` `csv` (default) or `base64`; `compression` empty, `gzip` or `zlib`. `zstd` is refused. |
| Object layers | Read: rectangles, points, ellipses, polygons, polylines, tile objects and text objects. |
| Image layers | Read as data (the picture's path, repeat flags and transparent colour). |
| Group layers | Read. Their visibility, opacity and offsets are combined into the layers they hold. |
| Tilesets | Read with one picture each: name, tile size, columns, tile count, margin, spacing, tile offset. |
| Tile properties, collision groups and animations | Read per tile (see [Tilesets](#tilesets)). |
| Custom properties | Read with their types: string (the default), int, float, bool, color, file, object and class. |
| Map `backgroundcolor` | Read as `#AARRGGBB` text, or undefined. |
| `properties` on any element | Read, as above. |

### Refused

A refused part gives the code `unsupported`, and the path names the field.

- A map that is not `orthogonal`, or a layer with `parallaxx` or `parallaxy` other than 1, or a layer with a
  non-empty `tintcolor`.
- Compression `zstd`.
- An image collection tileset (a tileset with no `image`, whose tiles each have their own `image`), and an
  image tile in a tileset with a picture.
- An object template (`template`).
- A property type that the reader does not know (any type other than the eight above).
- The hexagonal rotation bit in a tile id, which only hexagonal maps use. It is refused with code `value`.

### Read and ignored

The reader accepts these fields and does not use them: `version`, `tiledversion`, `renderorder`,
`compressionlevel`, `nextlayerid`, `nextobjectid`, the layer `x` and `y` of tile layers, the map's
`parallaxoriginx` and `parallaxoriginy` (they only matter with parallax, which is refused), a tile's
`probability` and `terrain`, and the `wangsets`, `grid`, `transformations` and `terrains` of a tileset. A tile
object's `visible` and an object's `visible` are read, but collision does not use them.

## Global ids

- The tilesets of a map own disjoint ranges of global ids. A tileset owns the ids from its `firstgid` to
  `firstgid + tilecount - 1`. Two tilesets whose ranges overlap are refused (`value`, at the second
  tileset's `firstgid`).
- A cell's id names a tile: the owning tileset is the one whose range holds the id, and the **local id** is
  the id less the tileset's `firstgid`. The local id is the tile's index in the picture, counted from 0.
- The map keeps its tilesets in order of `firstgid`. Tileset names are unique in a map (`duplicate`), because
  drawing takes the picture of each tileset by its name.
- Every non-zero id of a layer must name a tile of some tileset. An id that names none is refused (`value`,
  at the layer's `data`), and the message names the first such cell.

### Flip flags

Tiled stores the flips in the top bits of a 32-bit id. The reader removes them, and keeps them as three bits
of the cell's `flips` value:

| Bit | Value | Meaning |
|---|---|---|
| `TILE_FLIP_HORIZONTAL` | 1 | The tile is mirrored left to right (Tiled's bit 31). |
| `TILE_FLIP_VERTICAL` | 2 | The tile is mirrored top to bottom (bit 30). |
| `TILE_FLIP_DIAGONAL` | 4 | The tile is transposed across its diagonal (bit 29). |

Bit 28 (the hexagonal rotation) is refused. The `gid` of a cell or of a tile object never holds a flip.

## Layers

Every layer has the values that it is drawn with. A group's own values are combined into each layer it holds:

- `visible`: true only when the layer and every group above it are visible.
- `opacity`: the product of the layer's opacity and the opacities of its groups.
- `offsetX` and `offsetY`: the sum of the layer's offset and the offsets of its groups.

A tile layer of a finite map has one chunk at 0,0 that is the size of the map, and its size must equal the
map's size. A tile layer of an infinite map has one chunk per block: `x`, `y` (which may be negative), `width`,
`height` and `data`. Chunks that overlap are not checked, because Tiled never writes them.

Layer names are not unique. A search by name (`findLayer`) returns the first layer of that name, in depth-first
order.

## Objects

An object layer keeps its objects in the order of the file. Its `draworder` is `topdown` or `index`, and the
reader stores it; it does not sort the objects. The shape of an object is read from its fields:

1. `text`: a text object. Its fields are kept with Tiled's defaults (for example `pixelsize` 16 and `kerning`
   true).
2. `gid`: a tile object, with its tile id (no flips) and its flips. The id must name a tile of the map.
3. `point`: a point, with no area.
4. `ellipse`: an ellipse, inside the width and height.
5. `polygon`: a closed polygon with at least three points.
6. `polyline`: an open line with at least two points.
7. Otherwise: a rectangle.

Points are relative to the object's position. An object's `rotation` is in degrees, clockwise, around its
pivot: the top-left corner, or the bottom-left corner for a tile object. A tile object's `x` and `y` are its
bottom-left corner, as Tiled writes them.

## Properties

A property has a name, a type and a value. A property without a type is a string. The value must match the type:

| Type | Value | Rule |
|---|---|---|
| `string` | text | Any text. |
| `int` | number | A whole number. |
| `float` | number | A finite number. |
| `bool` | true or false | |
| `color` | `#RRGGBB` or `#AARRGGBB` text | `""` is an unset colour. |
| `file` | text | A path, as the map writes it. |
| `object` | whole number | An object id, or 0 for none. |
| `class` | object of members | The class name is kept. See below. |

A property whose `value` is left out takes the zero value of its type: an empty text, 0, or false. Tiled always
writes the value, and this reader accepts its absence without a problem.

A class property's members are typed by their JSON type, because the map does not carry the class definition
from the project: a string is `string`, a boolean is `bool`, a whole number is `int`, any other number is
`float`, and an object is a nested class with no class name. A member called `__proto__` is kept as a member.

Two properties of one element may not share a name (`duplicate`).

## Tilesets

A tileset is embedded in the map, or it is an external file (`source`). Either way it has one picture. Its
fields are read as follows:

- `name` (required, non-empty), `tilewidth` and `tileheight` (required), `image` (required), `columns` and
  `tilecount` (required, at least 1). `imagewidth`, `imageheight`, `margin` and `spacing` are read with defaults
  of 0.
- `tileoffset` `{ x, y }`: pixels added to the position of every tile when it is drawn and when its collision
  is placed. It defaults to 0 and 0.
- `properties`: the tileset's own properties.
- `tiles`: the tiles that say something. A tile entry has an `id` (from 0 to `tilecount - 1`) and may have:
  - `type` (or `class`), read as text;
  - `properties`;
  - `objectgroup`: the tile's **collision group**, whose objects are in pixels from the tile image's top-left
    corner;
  - `animation`: frames `{ tileid, duration }`, with `tileid` in the tileset and `duration` in whole
    milliseconds from 1.

  A tile entry that says nothing is not kept. Two entries with one `id` are refused (`duplicate`).

The **animation** of a tile loops over the total length of its frames. A tile shows the frame that is current
at the time, so `timeMs` 0 shows the first frame, and any finite time, negative ones included, is allowed.

## Collision

A collision grid is built from the tile layers of a map, by these rules, which add up: a cell is solid when any
rule makes it so.

- **Every tile** (`nonEmpty`): each non-empty tile of the layers is solid. This is the default when no other
  rule is named.
- **A property** (`solidProperty`): a tile whose boolean property of that name is true is solid.
- **Shapes** (`shapes`): a tile's collision group gives the solid parts of the tile. Each part is a
  rectangle inside the cell, in cell-local pixels.

The rules are applied to every layer named in `layers`, or to every tile layer of the map (groups included),
whatever its visibility.

### Shapes

- A rectangle is exact.
- An ellipse, and a polygon, are approximated by their **bounding box**. A rotated shape is approximated by the
  bounding box of its rotated corners, rotated around the shape's pivot.
- A point, a polyline, a text object and a tile object have no area, and are ignored.
- A tile image that is bigger or smaller than the map's cell is placed as drawing places it: its bottom-left
  corner is on the cell's bottom-left corner, shifted by the tileset's `tileoffset`. The parts are then
  clipped to the cell.
- A part that covers the whole cell makes the cell **solid**, not partial.

### Precedence and one-way

- A solid (whole) cell wins over the partial parts of the same cell.
- A cell is **one-way** only when every rule that made it solid says so. The one-way rule is the boolean
  property named by `oneWayProperty`, on a tile that is also solid.
- The grid covers the map's cells. An infinite map gets a grid over the bounds of its chunks. A cell outside
  the grid is empty. A grid of more than 16 777 216 cells is refused.
- Layer offsets are **ignored** by collision. Drawing shifts a layer by its offset, but the collision of its
  cells stays where the map puts them.

A grid has a `kindAt` (empty, solid or partial), an `isOneWayAt`, and a `partsAt` for each cell.

## Movement

`moveBox(grid, box, dx, dy, options)` moves a box with its top-left corner and size in pixels:

1. the box moves along **x** by `dx`, then along **y** by `dy`, with the result of the first as the start of
   the second;
2. each axis is walked in steps no longer than one map tile in that axis, so no tile is skipped, however far
   the box goes;
3. within a step, the box stops **flush** against the first solid rectangle that it would reach, and the step
   is cut there. A rectangle that the box reaches exactly at the end of a step is a contact too.

A box that already overlaps a solid rectangle in the direction of movement is not stopped by it, so a box
that starts inside a wall can leave the wall. A box touching a solid rectangle with a zero move is stopped
with a zero move, and the contact is reported. A call with `dy` of 1 is the standard way to ask whether a box
stands on the ground.

### One-way platforms

A one-way solid blocks a box only when it falls onto it from above: the box's bottom was at or above the top
of the platform, and it would pass through. A one-way solid never blocks sideways movement or upward movement.
`ignoreOneWay: true` makes every one-way solid pass.

### Results

`moveBox` returns:

- `box`: the box after the move;
- `contacts`: for each side (`left`, `right`, `top`, `bottom`), whether the move stopped against something on
  that side. `bottom` is set when the box lands, and `top` when it hits something above;
- `tiles`: the cells that stopped the box, each with the side that touched it. Every cell at the contact
  distance is listed, so a box that lands on three tiles at once lists three.

Slopes are not supported. Every solid rectangle is a box with flat faces.

## Pathfinding

`PathSearch.find(grid, from, to, options)` is an A* search over the cells of a grid. `findPath` is the same,
with a search made for one call.

- **Blocked cells**: a whole solid cell and a partial cell block the search. A one-way cell is walkable, because
  the search does not model jumps. The first cell may be blocked (a box can start inside a wall); the goal
  may not, unless it is the first cell.
- **Directions**: four by default, or eight with `diagonal`. A diagonal step costs the square root of 2. By
  default a diagonal step needs both cells beside it to be walkable (no corner cutting); `cornerCutting` lets it
  pass a blocked corner when the target cell is walkable.
- **Costs**: `cost(x, y)` adds a non-negative extra cost for entering a cell. Infinity makes the cell
  impassable. A negative or non-numeric cost is refused.
- **Limit**: `maxNodes` is the most cells that the search may expand. When it is reached, the search returns
  null.
- **Bounds**: the search stays inside the grid. A start or a goal outside the grid gives null.
- **Result**: the cells from the first to the goal, both included, as new objects in a new array, or null when
  there is no path.
- **Determinism**: the neighbours are visited in a fixed order, and two open cells with the same estimate are
  ordered by their remaining distance and then by their index. The same grid and options always give the
  same path.
- **Reuse**: a `PathSearch` keeps its working arrays, and grows them only when a larger grid comes. Its own
  work does not allocate; the returned path does.

## Drawing

`drawTileLayer(ctx, map, layer, images, view, timeMs)` draws the visible tiles of one tile layer to a 2D
context.

- The context already holds the **camera transform**. Drawing uses map pixels, and `view` is the part of the
  map that is visible (`x`, `y`, `width`, `height`). Only tiles that touch the view are drawn.
- A layer that is hidden, or that has no opacity, draws nothing. Otherwise the layer's opacity multiplies the
  context's alpha, and the state is restored when the layer is done, even when drawing fails.
- A tile is drawn from its picture rectangle (the margin, the spacing and the local id in the picture) to its
  cell: its left edge is the cell's left edge plus the layer's offset and the tileset's `tileoffset`, and its
  bottom edge is the cell's bottom edge, with the same offsets. A tile bigger than the map's cell therefore
  rises above it.
- **Flips** turn the tile about its centre, in this order: the diagonal flip first (which transposes the
  picture), then the horizontal flip, then the vertical one. The diagonal and horizontal flips together make a
  quarter turn clockwise, as Tiled shows them; the horizontal and vertical flips together make a half turn.
- **Animation** uses the time `timeMs` (see [Tilesets](#tilesets)).
- `images` gives the picture of each tileset, by the tileset's name. A visible tile whose tileset has no
  picture is an error (`missing-image`), and the state is restored before the error is thrown.

## Validation

A problem is reported with a code and a path. The first problem gives the code of the error.

| Code | When |
|---|---|
| `json` | The text is not JSON. |
| `structure` | A field is missing, or it has the wrong JSON type (for example `width` is text, or a layer has no `type`). |
| `value` | A value breaks a rule: a range, a grammar, a size, or a reference to a tile or a tileset that does not exist. |
| `duplicate` | Two tilesets share a name or a tile id, two tiles of a tileset share an id, or two properties share a name. |
| `unsupported` | The file uses a part of the format that the reader refuses (see [Refused](#refused)). |
| `data` | A layer's data does not decode: bad base64, a corrupt compressed stream, or the wrong number of cells. |
| `loader` | An external tileset is named and no loader was given, or the loader failed. The error keeps the loader's error as its `cause`. |
| `missing-image` | Drawing needs the picture of a tileset that the caller did not give. |
| `bad-input` | A call's argument is not valid: a missing layer, a layer that is not a tile layer, a grid that is too large, a distance that is not finite, a box with a negative size, a cell that is not a whole number, a `maxNodes` below 1, or a bad cost. |

An external tileset's problems are reported with the source as a prefix of their path, such as
`[tilesets/forest.tsj].tiles[2].id`.

## Versioning

This document describes version 1 of the subset. A change that makes a refused part accepted, or that changes a
rule above, is a new version. A new optional field that the reader may ignore does not change the version.

## Conformance fixtures

`docs/formats/fixtures/tiles/` holds the suite. Every reader runs `cases.json`:

- `valid`: each case states the sizes, the tilesets, the layers with their effective values, the cell ids and
  flips, the chunks of an infinite layer, the object shapes and some named objects. The cases cover the csv,
  base64, gzip and zlib encodings of one map (`csv.tmj`, `base64.tmj`, `gzip.tmj`, `zlib.tmj`), flips
  (`flips.tmj`), an infinite map (`infinite.tmj`), an external tileset with a second embedded one
  (`external.tmj` and `tilesets/forest.tsj`), every object shape with typed properties (`objects.tmj`), and
  nested groups with their combined opacity and offsets (`groups.tmj`). The tile animations and the collision
  groups of the tilesets are checked too.
- `invalid`: each file is a small map with one deliberate problem. Each case states the code, and the path
  where the case has one.

`make-encoded.mjs` wrote the three encoded variants from `csv.tmj`. Run it again only when `csv.tmj` changes.
