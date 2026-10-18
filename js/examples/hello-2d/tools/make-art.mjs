// Writes the art of hello-2d: no image files come with the example, so this script draws them.
//
// It writes, into public/:
// - art/tiles.png: nine 32-pixel tiles in five columns (ground, dirt, stone, one-way plank, spikes, coin, flag,
//   hill and cloud), with a dark outline and flat colours.
// - maps/tiles.tsj: the Tiled tileset that names that picture. Tile 3 (the plank) carries `one_way: true`.
// - art/hero.png and art/hero.json: the round hero, eight 32-pixel frames (idle 2, run 4, jump 1, fall 1), with a
//   TexturePacker atlas in the hash form.
// - art/coins.png and art/coins.json: six 16-pixel frames of a spinning coin, with an Aseprite export whose tag
//   "spin" loops over them.
//
// It needs only Node.js (zlib compresses the PNG data), so `node tools/make-art.mjs` from the example folder
// rewrites the files. The same bytes come out every time.

import { deflateSync } from "node:zlib";
import { mkdirSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const PUBLIC = join(dirname(fileURLToPath(import.meta.url)), "..", "public");

// ---------------------------------------------------------------------------------------------------------------
// Colours
// ---------------------------------------------------------------------------------------------------------------

const INK = "#2b1d24";
const GRASS = "#6fbf4a";
const GRASS_LIGHT = "#9ad96a";
const GRASS_DARK = "#3f7f2c";
const DIRT = "#8b5a3c";
const DIRT_DARK = "#6b4128";
const DIRT_LIGHT = "#a46f4a";
const STONE = "#8d93a6";
const STONE_LIGHT = "#b8bfd0";
const STONE_DARK = "#666c80";
const WOOD = "#c48a4f";
const WOOD_LIGHT = "#e3ad6c";
const WOOD_DARK = "#8f5f32";
const SPIKE = "#e6e9f2";
const SPIKE_DARK = "#9aa3b8";
const GOLD = "#ffcc33";
const GOLD_LIGHT = "#ffe77a";
const GOLD_DARK = "#b7791f";
const POLE = "#7a4a2a";
const RED = "#e0463c";
const CREAM = "#fff2d0";
const HILL = "#5fae6a";
const HILL_DARK = "#4a9256";
const HILL_LIGHT = "#84cc8a";
const CLOUD = "#ffffff";
const CLOUD_SHADE = "#dbeaff";
const CLOUD_EDGE = "#9cc4ea";
const HERO = "#ff8a5b";
const HERO_DARK = "#3a2f4f";
const SCARF = "#3fb6d6";
const CHEEK = "#ff9ab0";
const EYE = "#241c2e";
const WHITE = "#ffffff";

/** A colour as [r, g, b, a], from `#rrggbb`. */
function rgba(hex) {
  const value = Number.parseInt(hex.slice(1), 16);
  return [(value >> 16) & 255, (value >> 8) & 255, value & 255, 255];
}

// ---------------------------------------------------------------------------------------------------------------
// Pixels and masks. A mask has one byte per pixel: 1 inside the shape, 0 outside.
// ---------------------------------------------------------------------------------------------------------------

/** A picture of RGBA pixels, transparent at the start. */
class Pixels {
  constructor(width, height) {
    this.width = width;
    this.height = height;
    this.data = new Uint8Array(width * height * 4);
  }

  /** Writes one pixel, when it is inside the picture. */
  put(x, y, hex) {
    if (x < 0 || y < 0 || x >= this.width || y >= this.height) return;
    const [r, g, b, a] = rgba(hex);
    const index = (y * this.width + x) * 4;
    this.data[index] = r;
    this.data[index + 1] = g;
    this.data[index + 2] = b;
    this.data[index + 3] = a;
  }

  /** Writes the pixels of a mask in one colour. */
  paint(mask, hex) {
    for (let index = 0; index < mask.length; index++) {
      if (mask[index]) this.put(index % this.width, Math.floor(index / this.width), hex);
    }
  }

  /** Copies the opaque pixels of another picture into this one, with its top-left corner at (left, top). */
  copyFrom(other, left, top) {
    for (let y = 0; y < other.height; y++) {
      for (let x = 0; x < other.width; x++) {
        const from = (y * other.width + x) * 4;
        if (other.data[from + 3] === 0) continue;
        const to = ((top + y) * this.width + (left + x)) * 4;
        this.data.set(other.data.subarray(from, from + 4), to);
      }
    }
  }
}

/** An empty mask. */
function blank(width, height) {
  return new Uint8Array(width * height);
}

/** A mask of an ellipse centred at (cx, cy) with the radii rx and ry, sampled at pixel centres. */
function ellipse(width, height, cx, cy, rx, ry) {
  const mask = blank(width, height);
  for (let y = 0; y < height; y++) {
    for (let x = 0; x < width; x++) {
      const dx = (x + 0.5 - cx) / rx;
      const dy = (y + 0.5 - cy) / ry;
      if (dx * dx + dy * dy <= 1) mask[y * width + x] = 1;
    }
  }
  return mask;
}

/** A mask of the rectangle from (x0, y0) to (x1, y1), both corners included. */
function rect(width, height, x0, y0, x1, y1) {
  const mask = blank(width, height);
  for (let y = Math.max(0, y0); y <= Math.min(height - 1, y1); y++) {
    for (let x = Math.max(0, x0); x <= Math.min(width - 1, x1); x++) mask[y * width + x] = 1;
  }
  return mask;
}

/** A mask of the triangle with the corners (ax, ay), (bx, by) and (cx, cy), sampled at pixel centres. */
function triangle(width, height, ax, ay, bx, by, cx, cy) {
  const mask = blank(width, height);
  const side = (px, py, x1, y1, x2, y2) => (px - x2) * (y1 - y2) - (x1 - x2) * (py - y2);
  for (let y = 0; y < height; y++) {
    for (let x = 0; x < width; x++) {
      const px = x + 0.5;
      const py = y + 0.5;
      const d1 = side(px, py, ax, ay, bx, by);
      const d2 = side(px, py, bx, by, cx, cy);
      const d3 = side(px, py, cx, cy, ax, ay);
      const negative = d1 < 0 || d2 < 0 || d3 < 0;
      const positive = d1 > 0 || d2 > 0 || d3 > 0;
      if (!(negative && positive)) mask[y * width + x] = 1;
    }
  }
  return mask;
}

/** The union of a list of masks of one size. */
function unionAll(width, height, masks) {
  const out = blank(width, height);
  for (const mask of masks) {
    for (let index = 0; index < out.length; index++) if (mask[index]) out[index] = 1;
  }
  return out;
}

/** The pixels of `mask` that are not in `other`. */
function minus(mask, other) {
  return mask.map((value, index) => (value && !other[index] ? 1 : 0));
}

/** The pixels that both masks have. */
function both(a, b) {
  return a.map((value, index) => (value && b[index] ? 1 : 0));
}

/** The mask grown by one pixel up, down, left and right. */
function grow(mask, width, height) {
  const out = blank(width, height);
  for (let y = 0; y < height; y++) {
    for (let x = 0; x < width; x++) {
      if (!mask[y * width + x]) continue;
      out[y * width + x] = 1;
      if (x > 0) out[y * width + x - 1] = 1;
      if (x < width - 1) out[y * width + x + 1] = 1;
      if (y > 0) out[(y - 1) * width + x] = 1;
      if (y < height - 1) out[(y + 1) * width + x] = 1;
    }
  }
  return out;
}

/** Paints a dark outline around a silhouette (the pixels just outside it), then the silhouette in its colour. */
function inked(pixels, silhouette, hex) {
  const outside = minus(grow(silhouette, pixels.width, pixels.height), silhouette);
  pixels.paint(outside, INK);
  pixels.paint(silhouette, hex);
}

/** Scatters pixels of one colour in a fixed pattern, so the art is the same on every run. */
function speckle(pixels, hex, step, offset) {
  for (let y = 0; y < pixels.height; y++) {
    for (let x = 0; x < pixels.width; x++) {
      if ((x * 7 + y * 13 + offset) % step === 0) pixels.put(x, y, hex);
    }
  }
}

// ---------------------------------------------------------------------------------------------------------------
// The tileset: nine 32-pixel tiles, five to a row
// ---------------------------------------------------------------------------------------------------------------

const TILE = 32;
const TILE_COLUMNS = 5;
const TILE_COUNT = 9;

/** The tile ids, as the level and the tileset use them. */
const TILES = {
  grass: 0,
  dirt: 1,
  stone: 2,
  plank: 3,
  spikes: 4,
  coin: 5,
  flag: 6,
  hill: 7,
  cloud: 8,
};

/** The ground's top: a grass band, a darker lower edge and an outline along the top. */
function drawGrass() {
  const pixels = new Pixels(TILE, TILE);
  pixels.paint(rect(TILE, TILE, 0, 0, TILE - 1, TILE - 1), DIRT);
  speckle(pixels, DIRT_DARK, 10, 3);
  pixels.paint(rect(TILE, TILE, 0, 0, TILE - 1, 9), GRASS);
  pixels.paint(rect(TILE, TILE, 0, 8, TILE - 1, 9), GRASS_DARK);
  for (let x = 0; x < TILE; x += 6) pixels.paint(rect(TILE, TILE, x + 1, 2, x + 2, 3), GRASS_LIGHT);
  pixels.paint(rect(TILE, TILE, 0, 0, TILE - 1, 1), INK);
  return pixels;
}

/** The dirt inside the ground: brown, with darker and lighter flecks. */
function drawDirt() {
  const pixels = new Pixels(TILE, TILE);
  pixels.paint(rect(TILE, TILE, 0, 0, TILE - 1, TILE - 1), DIRT);
  speckle(pixels, DIRT_DARK, 7, 5);
  speckle(pixels, DIRT_LIGHT, 11, 7);
  return pixels;
}

/** A stone block: a light top, a shaded bottom, cracks, and an outline on each side. */
function drawStone() {
  const pixels = new Pixels(TILE, TILE);
  pixels.paint(rect(TILE, TILE, 0, 0, TILE - 1, TILE - 1), STONE);
  pixels.paint(rect(TILE, TILE, 0, 0, TILE - 1, 2), STONE_LIGHT);
  pixels.paint(rect(TILE, TILE, 0, TILE - 3, TILE - 1, TILE - 1), STONE_DARK);
  pixels.paint(rect(TILE, TILE, 0, 15, TILE - 1, 15), STONE_DARK);
  pixels.paint(rect(TILE, TILE, 15, 0, 15, 15), STONE_DARK);
  pixels.paint(rect(TILE, TILE, 7, 16, 7, 31), STONE_DARK);
  pixels.paint(rect(TILE, TILE, 23, 16, 23, 31), STONE_DARK);
  pixels.paint(rect(TILE, TILE, 0, 0, 0, TILE - 1), INK);
  pixels.paint(rect(TILE, TILE, TILE - 1, 0, TILE - 1, TILE - 1), INK);
  pixels.paint(rect(TILE, TILE, 0, 0, TILE - 1, 0), INK);
  pixels.paint(rect(TILE, TILE, 0, TILE - 1, TILE - 1, TILE - 1), INK);
  return pixels;
}

/** A one-way plank: a wooden board in the top ten pixels, with grain and an outline. */
function drawPlank() {
  const pixels = new Pixels(TILE, TILE);
  const board = rect(TILE, TILE, 0, 0, TILE - 1, 9);
  inked(pixels, board, WOOD);
  pixels.paint(rect(TILE, TILE, 1, 1, TILE - 2, 2), WOOD_LIGHT);
  pixels.paint(rect(TILE, TILE, 2, 5, TILE - 3, 5), WOOD_DARK);
  pixels.paint(rect(TILE, TILE, 9, 3, 9, 7), WOOD_DARK);
  pixels.paint(rect(TILE, TILE, 22, 3, 22, 7), WOOD_DARK);
  return pixels;
}

/** Four spikes standing on the bottom of the tile, each with an outline and a darker right half. */
function drawSpikes() {
  const pixels = new Pixels(TILE, TILE);
  const centres = [4, 12, 20, 28];
  const spikes = centres.map((centre) => triangle(TILE, TILE, centre - 4.5, 31, centre + 4.5, 31, centre, 13));
  inked(pixels, unionAll(TILE, TILE, spikes), SPIKE);
  const shades = centres.map((centre) => triangle(TILE, TILE, centre, 31, centre + 4.5, 31, centre, 14));
  pixels.paint(unionAll(TILE, TILE, shades), SPIKE_DARK);
  return pixels;
}

/** A coin for the HUD: a gold disc with a lighter face and an outline. */
function drawCoinIcon() {
  const pixels = new Pixels(TILE, TILE);
  inked(pixels, ellipse(TILE, TILE, 16, 16, 12, 12), GOLD);
  pixels.paint(ellipse(TILE, TILE, 16, 16, 8, 8), GOLD_DARK);
  pixels.paint(ellipse(TILE, TILE, 16, 16, 7, 7), GOLD_LIGHT);
  pixels.paint(rect(TILE, TILE, 12, 8, 13, 11), GOLD_DARK);
  return pixels;
}

/** The goal: a pole with a red flag and a cream emblem, standing on a small stone. */
function drawFlag() {
  const pixels = new Pixels(TILE, TILE);
  const pole = rect(TILE, TILE, 6, 2, 8, 30);
  const flag = rect(TILE, TILE, 9, 3, 27, 15);
  const base = rect(TILE, TILE, 3, 28, 11, 31);
  inked(pixels, unionAll(TILE, TILE, [pole, flag, base]), POLE);
  pixels.paint(flag, RED);
  pixels.paint(base, STONE);
  pixels.paint(pole, POLE);
  pixels.paint(ellipse(TILE, TILE, 18, 9, 3, 3), CREAM);
  return pixels;
}

/** A background hill: a dome that rises from the bottom of the tile, with a light cap and a darker base. */
function drawHill() {
  const pixels = new Pixels(TILE, TILE);
  const dome = ellipse(TILE, TILE, 16, 36, 19, 22);
  pixels.paint(dome, HILL);
  pixels.paint(dome.map((value, index) => (value && index < 22 * TILE && index % TILE > 8 && index % TILE < 24 ? 1 : 0)), HILL_LIGHT);
  pixels.paint(dome.map((value, index) => (value && index >= 27 * TILE ? 1 : 0)), HILL_DARK);
  return pixels;
}

/** A background cloud: three round puffs and a flat base, pale, with a soft outline. */
function drawCloud() {
  const pixels = new Pixels(TILE, TILE);
  const puffs = unionAll(TILE, TILE, [
    ellipse(TILE, TILE, 10, 18, 6, 6),
    ellipse(TILE, TILE, 18, 14, 8, 8),
    ellipse(TILE, TILE, 24, 20, 6, 6),
    rect(TILE, TILE, 6, 18, 28, 24),
  ]);
  pixels.paint(minus(grow(puffs, TILE, TILE), puffs), CLOUD_EDGE);
  pixels.paint(puffs, CLOUD);
  pixels.paint(puffs.map((value, index) => (value && index >= 22 * TILE ? 1 : 0)), CLOUD_SHADE);
  return pixels;
}

/** The tileset picture: the nine tiles in their cells. The tenth cell stays empty. */
function buildTiles() {
  const sheet = new Pixels(TILE * TILE_COLUMNS, TILE * 2);
  const tiles = [
    [TILES.grass, drawGrass()],
    [TILES.dirt, drawDirt()],
    [TILES.stone, drawStone()],
    [TILES.plank, drawPlank()],
    [TILES.spikes, drawSpikes()],
    [TILES.coin, drawCoinIcon()],
    [TILES.flag, drawFlag()],
    [TILES.hill, drawHill()],
    [TILES.cloud, drawCloud()],
  ];
  for (const [id, pixels] of tiles) {
    sheet.copyFrom(pixels, (id % TILE_COLUMNS) * TILE, Math.floor(id / TILE_COLUMNS) * TILE);
  }
  return sheet;
}

// ---------------------------------------------------------------------------------------------------------------
// The hero: a round character in 32-pixel frames
// ---------------------------------------------------------------------------------------------------------------

const FRAME = 32;

/** A mask of the box from (x0, y0) to (x1, y1), both corners included, in one hero frame. */
function boxMask([x0, y0, x1, y1]) {
  return rect(FRAME, FRAME, x0, y0, x1, y1);
}

/**
 * One hero frame. The body is a round ellipse with a scarf across it, two eyes with a highlight each, a small
 * mouth and two cheeks. `lift` raises the body (the breathing of the idle frames), `feet` and `arms` are boxes,
 * and the feet are dark.
 */
function heroFrame({ lift = 0, arms = [], feet, bodyRy = 11 }) {
  const body = ellipse(FRAME, FRAME, 16, 15 - lift, 10, bodyRy);
  const shoes = unionAll(FRAME, FRAME, feet.map(boxMask));
  const sleeves = unionAll(FRAME, FRAME, arms.map(boxMask));
  const pixels = new Pixels(FRAME, FRAME);
  inked(pixels, unionAll(FRAME, FRAME, [body, shoes, sleeves]), HERO);
  pixels.paint(shoes, HERO_DARK);
  pixels.paint(both(boxMask([5, 21 - lift, 26, 22 - lift]), body), SCARF);
  pixels.paint(boxMask([12, 12 - lift, 13, 14 - lift]), EYE);
  pixels.paint(boxMask([18, 12 - lift, 19, 14 - lift]), EYE);
  pixels.put(12, 12 - lift, WHITE);
  pixels.put(18, 12 - lift, WHITE);
  pixels.paint(boxMask([15, 18 - lift, 16, 18 - lift]), EYE);
  pixels.paint(both(boxMask([9, 17 - lift, 10, 18 - lift]), body), CHEEK);
  pixels.paint(both(boxMask([21, 17 - lift, 22, 18 - lift]), body), CHEEK);
  return pixels;
}

/** The eight hero frames, in the order of the atlas: idle 2, run 4, jump 1, fall 1. */
function heroFrames() {
  const standing = [
    [12, 25, 15, 31],
    [17, 25, 20, 31],
  ];
  const stepLeft = [
    [6, 26, 11, 31],
    [20, 25, 24, 30],
  ];
  const stepRight = [
    [10, 25, 15, 30],
    [20, 26, 25, 31],
  ];
  return [
    ["idle_0", heroFrame({ feet: standing })],
    ["idle_1", heroFrame({ lift: 1, feet: standing })],
    ["run_0", heroFrame({ feet: stepLeft })],
    ["run_1", heroFrame({ feet: standing })],
    ["run_2", heroFrame({ feet: stepRight })],
    ["run_3", heroFrame({ feet: standing })],
    [
      "jump_0",
      heroFrame({
        feet: [[12, 26, 19, 30]],
        arms: [
          [4, 9, 7, 13],
          [24, 9, 27, 13],
        ],
      }),
    ],
    [
      "fall_0",
      heroFrame({
        feet: [
          [7, 26, 12, 31],
          [19, 26, 24, 31],
        ],
        arms: [
          [3, 16, 6, 19],
          [25, 16, 28, 19],
        ],
      }),
    ],
  ];
}

/** Lays pictures side by side in one strip. */
function strip(frames, frameWidth, frameHeight) {
  const sheet = new Pixels(frameWidth * frames.length, frameHeight);
  frames.forEach((frame, index) => sheet.copyFrom(frame, index * frameWidth, 0));
  return sheet;
}

// ---------------------------------------------------------------------------------------------------------------
// The coin: six frames of a spin, each a narrower ellipse
// ---------------------------------------------------------------------------------------------------------------

const COIN = 16;
const COIN_FRAMES = 6;

/** One frame of the spinning coin: its width follows the cosine of the turn, so it is edge-on at frame 3. */
function coinFrame(index) {
  const turn = (index * Math.PI) / COIN_FRAMES;
  const rx = Math.max(1.5, 7 * Math.abs(Math.cos(turn)));
  const pixels = new Pixels(COIN, COIN);
  inked(pixels, ellipse(COIN, COIN, 8, 8, rx, 7), GOLD);
  pixels.paint(ellipse(COIN, COIN, 8, 8, Math.max(1, rx - 2), 5), GOLD_LIGHT);
  if (rx > 4) pixels.put(5, 5, CREAM);
  return pixels;
}

// ---------------------------------------------------------------------------------------------------------------
// PNG and atlas files
// ---------------------------------------------------------------------------------------------------------------

const CRC_TABLE = (() => {
  const table = new Uint32Array(256);
  for (let n = 0; n < 256; n++) {
    let c = n;
    for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    table[n] = c >>> 0;
  }
  return table;
})();

/** The CRC-32 of some bytes, as a PNG chunk needs it. */
function crc32(bytes) {
  let c = 0xffffffff;
  for (const byte of bytes) c = CRC_TABLE[(c ^ byte) & 0xff] ^ (c >>> 8);
  return (c ^ 0xffffffff) >>> 0;
}

/** One PNG chunk: its length, its type, its data, and the CRC of the type and the data. */
function chunk(type, data) {
  const length = Buffer.alloc(4);
  length.writeUInt32BE(data.length, 0);
  const typed = Buffer.concat([Buffer.from(type, "ascii"), data]);
  const crc = Buffer.alloc(4);
  crc.writeUInt32BE(crc32(typed), 0);
  return Buffer.concat([length, typed, crc]);
}

/** Encodes a picture as a PNG file: 8-bit RGBA, filter 0 on every row, and one deflate stream. */
function encodePng(pixels) {
  const { width, height, data } = pixels;
  const rowBytes = width * 4;
  const rows = Buffer.alloc((rowBytes + 1) * height);
  for (let y = 0; y < height; y++) {
    rows[y * (rowBytes + 1)] = 0;
    Buffer.from(data.buffer, y * rowBytes, rowBytes).copy(rows, y * (rowBytes + 1) + 1);
  }
  const header = Buffer.alloc(13);
  header.writeUInt32BE(width, 0);
  header.writeUInt32BE(height, 4);
  header[8] = 8; // bit depth
  header[9] = 6; // colour type: RGBA
  header[10] = 0; // compression
  header[11] = 0; // filter method
  header[12] = 0; // no interlace
  return Buffer.concat([
    Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]),
    chunk("IHDR", header),
    chunk("IDAT", deflateSync(rows, { level: 9 })),
    chunk("IEND", Buffer.alloc(0)),
  ]);
}

/** A frame of a TexturePacker atlas, in the hash form: the stored rectangle and the untrimmed size. */
function packedEntry(x, y, size) {
  return {
    frame: { x, y, w: size, h: size },
    rotated: false,
    trimmed: false,
    spriteSourceSize: { x: 0, y: 0, w: size, h: size },
    sourceSize: { w: size, h: size },
  };
}

/** Writes a file, creating its folder first, and says what it wrote. */
function write(path, bytes) {
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, bytes);
  const shown = path.slice(PUBLIC.length + 1).replaceAll("\\", "/");
  console.log(`wrote public/${shown} (${bytes.length} bytes)`);
}

// ---------------------------------------------------------------------------------------------------------------
// Run
// ---------------------------------------------------------------------------------------------------------------

write(join(PUBLIC, "art", "tiles.png"), encodePng(buildTiles()));

const tileset = {
  type: "tileset",
  version: "1.10",
  tiledversion: "1.10.2",
  name: "tiles",
  image: "../art/tiles.png",
  imagewidth: TILE * TILE_COLUMNS,
  imageheight: TILE * 2,
  margin: 0,
  spacing: 0,
  tilewidth: TILE,
  tileheight: TILE,
  columns: TILE_COLUMNS,
  tilecount: TILE_COUNT,
  tiles: [{ id: TILES.plank, properties: [{ name: "one_way", type: "bool", value: true }] }],
};
write(join(PUBLIC, "maps", "tiles.tsj"), Buffer.from(`${JSON.stringify(tileset, null, 2)}\n`));

const heroList = heroFrames();
write(join(PUBLIC, "art", "hero.png"), encodePng(strip(heroList.map(([, frame]) => frame), FRAME, FRAME)));
const heroAtlas = {
  frames: Object.fromEntries(heroList.map(([name], index) => [name, packedEntry(index * FRAME, 0, FRAME)])),
  meta: {
    app: "hello-2d tools/make-art.mjs",
    version: "1.0",
    image: "hero.png",
    format: "RGBA8888",
    size: { w: FRAME * heroList.length, h: FRAME },
    scale: "1",
  },
};
write(join(PUBLIC, "art", "hero.json"), Buffer.from(`${JSON.stringify(heroAtlas, null, 2)}\n`));

const coinList = Array.from({ length: COIN_FRAMES }, (_, index) => coinFrame(index));
write(join(PUBLIC, "art", "coins.png"), encodePng(strip(coinList, COIN, COIN)));
const coinExport = {
  frames: Object.fromEntries(
    coinList.map((_, index) => [`coin ${index}`, { ...packedEntry(index * COIN, 0, COIN), duration: 90 }]),
  ),
  meta: {
    app: "hello-2d tools/make-art.mjs",
    version: "1.0",
    image: "coins.png",
    format: "RGBA8888",
    size: { w: COIN * COIN_FRAMES, h: COIN },
    scale: "1",
    frameTags: [{ name: "spin", from: 0, to: COIN_FRAMES - 1, direction: "forward" }],
  },
};
write(join(PUBLIC, "art", "coins.json"), Buffer.from(`${JSON.stringify(coinExport, null, 2)}\n`));
