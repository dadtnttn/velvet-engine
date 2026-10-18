// Writes the level of hello-2d as a Tiled map: public/maps/level1.tmj (orthogonal, 32-pixel tiles, CSV data, one
// external tileset in maps/tiles.tsj, which tools/make-art.mjs writes).
//
// The level is drawn below as text, one character per 32-pixel cell, so that it is easy to read and to change.
// Run `node tools/make-level.mjs` after a change; the Tiled editor can open the result and edit it too.
//
// Characters of the world (the rows of WORLD):
//   #  ground: grass where the cell above is not ground, dirt where it is
//   =  stone block (solid)
//   -  one-way plank (the player stands on it and passes it from below)
//   ^  spikes (hurt the player; they stand on the floor of a cell, and the player walks into them)
//   o  coin (a point object)
//   S  the player's start (a point object, at the bottom centre of its cell)
//   C  a checkpoint (a point object, at the bottom centre of its cell)
//   F  the goal flag (a point object, at the bottom centre of its cell)
//   .  nothing
//
// The scenery (the background layer, drawn at a parallax factor in the game) has hills (b) and clouds (c), and
// it is set by the table below, not by the text.
//
// Layers, in the order of the file: background (tiles), ground (tiles), platforms (tiles), hazards (tiles) and
// objects (points). The tile ids are those of maps/tiles.tsj: 0 grass, 1 dirt, 2 stone, 3 plank, 4 spikes,
// 7 hill, 8 cloud. A cell's gid is its tile id plus one (the tileset starts at gid 1).

import { mkdirSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
const OUT = join(HERE, "..", "public", "maps", "level1.tmj");

const TILE = 32;

/** Pads a run of one character. */
const dots = (count) => ".".repeat(count);
const solid = (count) => "#".repeat(count);

/** The world, row by row. Each row is made of the segments of the level, from the left, in the order given. */
const WORLD = [];
for (let row = 0; row < 30; row += 1) WORLD.push(dots(120));

/** Sets a row from its segments. Every segment must have the width given in its comment. */
function setRow(row, segments) {
  WORLD[row] = segments.join("");
}

// Segment widths: start 14, pit 3, flat 11, spikes 6, stone 6, stairs 8, flat 5, pit 6, spikes and checkpoint 13,
// tower 9, planks 12, flat 3, pit 3, flat 12, goal 8, and one cell at the end. They add up to 120.
setRow(21, [dots(14), dots(3), dots(11), dots(6), dots(6), dots(8), dots(5), dots(6), dots(13), dots(9), "........oo..", dots(3), dots(3), dots(12), dots(8), dots(1)]);
setRow(22, [dots(14), "ooo", dots(11), "..ooo.", dots(6), dots(8), dots(5), "..ooo.", "..o...o......", dots(9), ".......----.", dots(3), "ooo", ".....o......", dots(8), dots(1)]);
setRow(23, [dots(14), dots(3), "...oooo....", dots(6), dots(6), dots(8), dots(5), dots(6), dots(13), dots(9), "...oooo.....", dots(3), dots(3), dots(12), dots(8), dots(1)]);
setRow(24, ["....oooo......", dots(3), "...----....", dots(6), dots(6), "...===..", dots(5), dots(6), dots(13), "...==....", "...----.....", dots(3), dots(3), dots(12), dots(8), dots(1)]);
setRow(25, ["..S" + dots(11), dots(3), dots(11), "...^..", "..==..", ".=====..", dots(5), dots(6), "..^...^...C..", "...==....", dots(12), dots(3), dots(3), ".....^......", ".....F..", dots(1)]);
setRow(26, [solid(14), "^^^", solid(11), solid(6), solid(6), solid(8), solid(5), "##^^^#", solid(13), solid(9), solid(12), solid(3), "^^^", solid(12), solid(8), solid(1)]);
setRow(27, [solid(14), solid(3), solid(11), solid(6), solid(6), solid(8), solid(5), solid(6), solid(13), solid(9), solid(12), solid(3), solid(3), solid(12), solid(8), solid(1)]);
setRow(28, [solid(120)]);
setRow(29, [solid(120)]);

// The scenery: hills on the row above the ground, and clouds in the sky.
const SCENERY = Array.from({ length: 30 }, () => Array.from({ length: 120 }, () => "."));
for (const column of [6, 18, 33, 50, 64, 83, 99, 112]) SCENERY[25][column] = "b";
for (const [row, column] of [[3, 10], [4, 27], [5, 44], [3, 61], [6, 79], [4, 96], [5, 110]]) SCENERY[row][column] = "c";

// Checks the drawing: every row of the world and of the scenery is 120 cells wide.
for (const [index, text] of WORLD.entries()) {
  if (text.length !== 120) throw new Error(`world row ${index} has ${text.length} cells, not 120`);
}
const WIDTH = 120;
const HEIGHT = WORLD.length;

// The tile ids, from the tileset.
const GRASS = 0;
const DIRT = 1;
const STONE = 2;
const PLANK = 3;
const SPIKES = 4;
const HILL = 7;
const CLOUD = 8;

/** The gid of a tile id: the tileset starts at gid 1. */
const gid = (tile) => tile + 1;

/** A tile layer's data: a gid for each cell, row by row, with 0 for no tile. */
function tileData(pick) {
  const data = [];
  for (let row = 0; row < HEIGHT; row += 1) {
    for (let column = 0; column < WIDTH; column += 1) {
      const tile = pick(row, column);
      data.push(tile === null ? 0 : gid(tile));
    }
  }
  return data;
}

const at = (row, column) => (row < 0 || row >= HEIGHT ? "." : WORLD[row]?.[column] ?? ".");

const layers = [];
let nextId = 1;

layers.push({
  id: nextId++,
  name: "background",
  type: "tilelayer",
  width: WIDTH,
  height: HEIGHT,
  x: 0,
  y: 0,
  opacity: 1,
  visible: true,
  data: tileData((row, column) => {
    const cell = SCENERY[row]?.[column];
    if (cell === "b") return HILL;
    if (cell === "c") return CLOUD;
    return null;
  }),
});

layers.push({
  id: nextId++,
  name: "ground",
  type: "tilelayer",
  width: WIDTH,
  height: HEIGHT,
  x: 0,
  y: 0,
  opacity: 1,
  visible: true,
  data: tileData((row, column) => {
    const cell = at(row, column);
    if (cell === "#") return at(row - 1, column) === "#" ? DIRT : GRASS;
    if (cell === "=") return STONE;
    return null;
  }),
});

layers.push({
  id: nextId++,
  name: "platforms",
  type: "tilelayer",
  width: WIDTH,
  height: HEIGHT,
  x: 0,
  y: 0,
  opacity: 1,
  visible: true,
  data: tileData((row, column) => (at(row, column) === "-" ? PLANK : null)),
});

layers.push({
  id: nextId++,
  name: "hazards",
  type: "tilelayer",
  width: WIDTH,
  height: HEIGHT,
  x: 0,
  y: 0,
  opacity: 1,
  visible: true,
  data: tileData((row, column) => (at(row, column) === "^" ? SPIKES : null)),
});

// The objects: points. A point at the bottom centre of a cell stands on the cell's floor.
const objects = [];
let nextObject = 1;
const KINDS = { S: ["spawn", "start", true], C: ["checkpoint", "checkpoint", true], F: ["goal", "goal", true], o: ["coin", "coin", false] };
for (let row = 0; row < HEIGHT; row += 1) {
  for (let column = 0; column < WIDTH; column += 1) {
    const kind = KINDS[at(row, column)];
    if (kind === undefined) continue;
    const [type, name, bottom] = kind;
    const x = column * TILE + TILE / 2;
    const y = bottom ? row * TILE + TILE : row * TILE + TILE / 2;
    objects.push({ id: nextObject++, name, type, point: true, x, y, width: 0, height: 0, rotation: 0, visible: true });
  }
}
layers.push({
  id: nextId++,
  name: "objects",
  type: "objectgroup",
  draworder: "topdown",
  x: 0,
  y: 0,
  opacity: 1,
  visible: true,
  objects,
});

const map = {
  type: "map",
  version: "1.10",
  tiledversion: "1.10.2",
  orientation: "orthogonal",
  renderorder: "right-down",
  infinite: false,
  width: WIDTH,
  height: HEIGHT,
  tilewidth: TILE,
  tileheight: TILE,
  nextlayerid: nextId,
  nextobjectid: nextObject,
  tilesets: [{ firstgid: 1, source: "tiles.tsj" }],
  layers,
};

mkdirSync(dirname(OUT), { recursive: true });
writeFileSync(OUT, `${JSON.stringify(map, null, 1)}\n`);

const coins = objects.filter((object) => object.type === "coin").length;
const checkpoints = objects.filter((object) => object.type === "checkpoint").length;
console.log(`wrote public/maps/level1.tmj: ${WIDTH} by ${HEIGHT} cells, ${coins} coins, ${checkpoints} checkpoints`);
