// Tests for drawing a tile layer, with a recording stand-in for the 2D context (happy-dom has no canvas). The
// recorder keeps the transform the way a canvas does, so the tests check where each flipped tile lands on the
// device, not just which calls were made. They check the culling, the source and destination rectangles, the
// bottom alignment of big tiles, the flips, the animation, the opacity, and that the state is balanced.

import { describe, expect, it } from "vitest";
import { drawTileLayer, type TileDrawingContext, type TileImages } from "./draw";
import { TilesError } from "./error";
import { loadTileMap } from "./load";
import { findLayer } from "./map";
import { map, terrainTileset, tileLayer } from "./test-support";
import type { TileLayer, TileMap } from "./types";
import { TILE_FLIP_DIAGONAL, TILE_FLIP_HORIZONTAL, TILE_FLIP_VERTICAL } from "./types";

/** An affine matrix as a canvas keeps it: a, b, c, d, e, f. */
type Matrix = readonly [number, number, number, number, number, number];

/** One recorded drawImage call, with the transform and alpha in force at the time. */
interface Draw {
  readonly image: unknown;
  readonly source: readonly number[];
  readonly dest: readonly number[];
  readonly matrix: Matrix;
  readonly alpha: number;
}

/** Multiplies the current transform by a new one, as `transform` does on a canvas. */
function multiply(current: Matrix, next: Matrix): Matrix {
  const [a0, b0, c0, d0, e0, f0] = current;
  const [a, b, c, d, e, f] = next;
  return [a0 * a + c0 * b, b0 * a + d0 * b, a0 * c + c0 * d, b0 * c + d0 * d, a0 * e + c0 * f + e0, b0 * e + d0 * f + f0];
}

/** Where a point of the transformed space lands on the device. */
function apply(matrix: Matrix, x: number, y: number): [number, number] {
  return [matrix[0] * x + matrix[2] * y + matrix[4], matrix[1] * x + matrix[3] * y + matrix[5]];
}

/** A stand-in for a 2D context that records what is drawn, with the transform and alpha in force. */
class Recorder implements TileDrawingContext {
  globalAlpha = 1;
  matrix: Matrix = [1, 0, 0, 1, 0, 0];
  readonly draws: Draw[] = [];
  private readonly stack: { matrix: Matrix; alpha: number }[] = [];

  /** How many saves are not yet restored. Zero when the state is balanced. */
  get depth(): number {
    return this.stack.length;
  }

  save(): void {
    this.stack.push({ matrix: this.matrix, alpha: this.globalAlpha });
  }

  restore(): void {
    const top = this.stack.pop();
    if (top !== undefined) {
      this.matrix = top.matrix;
      this.globalAlpha = top.alpha;
    }
  }

  transform(a: number, b: number, c: number, d: number, e: number, f: number): void {
    this.matrix = multiply(this.matrix, [a, b, c, d, e, f]);
  }

  drawImage(
    image: CanvasImageSource,
    sx: number,
    sy: number,
    sw: number,
    sh: number,
    dx: number,
    dy: number,
    dw: number,
    dh: number,
  ): void {
    this.draws.push({ image, source: [sx, sy, sw, sh], dest: [dx, dy, dw, dh], matrix: this.matrix, alpha: this.globalAlpha });
  }
}

/** A picture for the tests. Drawing only passes it on. */
const picture = { name: "terrain-picture" } as unknown as CanvasImageSource;
const images: TileImages = { terrain: picture };

/** Where a source pixel (u, v) of a draw lands on the device, after the draw's transform. */
function deviceOf(draw: Draw, u: number, v: number): [number, number] {
  const [, , sw, sh] = draw.source as [number, number, number, number];
  const [dx, dy, dw, dh] = draw.dest as [number, number, number, number];
  return apply(draw.matrix, dx + (u * dw) / sw, dy + (v * dh) / sh);
}

/** Loads a map and returns it with its one tile layer, named "ground" unless the layer has another name. */
async function loadWith(layer: Record<string, unknown>, options: { tileset?: Record<string, unknown> } = {}): Promise<{ loaded: TileMap; layer: TileLayer }> {
  const loaded = await loadTileMap(
    map({
      width: (layer.width as number | undefined) ?? 4,
      height: (layer.height as number | undefined) ?? 3,
      layers: [layer],
      tilesets: [terrainTileset(options.tileset ?? {})],
    }),
  );
  const found = findLayer(loaded, String(layer.name)) as TileLayer;
  return { loaded, layer: found };
}

/** A view that holds the whole of a small map. */
const wholeMap = { x: -1000, y: -1000, width: 3000, height: 3000 };

describe("which tiles are drawn", () => {
  it("draws only the tiles that the view shows", async () => {
    const ones = Array.from({ length: 100 }, () => 1);
    const { loaded, layer } = await loadWith(tileLayer("ground", 10, 10, ones));
    const recorder = new Recorder();
    drawTileLayer(recorder, loaded, layer, images, { x: 32, y: 32, width: 32, height: 32 }, 0);
    const dests = recorder.draws.map((draw) => draw.dest.slice(0, 2));
    expect(dests).toHaveLength(4);
    expect(dests).toEqual(
      expect.arrayContaining([
        [32, 32],
        [48, 32],
        [32, 48],
        [48, 48],
      ]),
    );
  });

  it("draws nothing for a hidden layer or a layer with no opacity", async () => {
    const { loaded, layer } = await loadWith(tileLayer("ground", 4, 3, Array.from({ length: 12 }, () => 1), { visible: false }));
    const recorder = new Recorder();
    drawTileLayer(recorder, loaded, layer, images, wholeMap, 0);
    expect(recorder.draws).toEqual([]);
    expect(recorder.depth).toBe(0);

    const faint = await loadWith(tileLayer("ground", 4, 3, Array.from({ length: 12 }, () => 1), { opacity: 0 }));
    drawTileLayer(new Recorder(), faint.loaded, faint.layer, images, wholeMap, 0);
  });

  it("applies the layer's offset to every tile", async () => {
    const { loaded, layer } = await loadWith(tileLayer("ground", 1, 1, [1], { offsetx: 8, offsety: 4 }), {
      tileset: {},
    });
    const recorder = new Recorder();
    drawTileLayer(recorder, loaded, layer, images, wholeMap, 0);
    expect(recorder.draws[0]?.dest).toEqual([8, 4, 16, 16]);
  });
});

describe("source and destination rectangles", () => {
  it("cuts the picture by the margin and spacing of its tileset", async () => {
    const tileset = { margin: 1, spacing: 2, imagewidth: 80, imageheight: 40 };
    const { loaded, layer } = await loadWith(tileLayer("ground", 1, 1, [3]), { tileset });
    const recorder = new Recorder();
    drawTileLayer(recorder, loaded, layer, images, wholeMap, 0);
    expect(recorder.draws[0]?.source).toEqual([1 + 2 * 18, 1, 16, 16]);
  });

  it("draws a tile bigger than the map's cells with its bottom on the cell's bottom", async () => {
    const tileset = { tilewidth: 32, tileheight: 32, imagewidth: 128, imageheight: 64 };
    const { loaded, layer } = await loadWith(tileLayer("ground", 1, 1, [1]), { tileset });
    const recorder = new Recorder();
    drawTileLayer(recorder, loaded, layer, images, wholeMap, 0);
    expect(recorder.draws[0]?.dest).toEqual([0, -16, 32, 32]);
  });

  it("shifts the tiles of a tileset by its tile offset", async () => {
    const { loaded, layer } = await loadWith(tileLayer("ground", 1, 1, [1]), { tileset: { tileoffset: { x: 2, y: -4 } } });
    const recorder = new Recorder();
    drawTileLayer(recorder, loaded, layer, images, wholeMap, 0);
    expect(recorder.draws[0]?.dest).toEqual([2, -4, 16, 16]);
  });
});

describe("flipped tiles", () => {
  /** Draws one tile with the given flips at cell (0, 0), and returns the draw. */
  async function flipped(flips: number): Promise<Draw> {
    const gid = 2 + (flips & TILE_FLIP_HORIZONTAL ? 0x80000000 : 0) + (flips & TILE_FLIP_VERTICAL ? 0x40000000 : 0) + (flips & TILE_FLIP_DIAGONAL ? 0x20000000 : 0);
    const { loaded, layer } = await loadWith(tileLayer("ground", 1, 1, [gid]));
    const recorder = new Recorder();
    drawTileLayer(recorder, loaded, layer, images, wholeMap, 0);
    const draw = recorder.draws[0];
    if (draw === undefined) throw new Error("no draw");
    return draw;
  }

  it("mirrors left to right with the horizontal flip", async () => {
    const draw = await flipped(TILE_FLIP_HORIZONTAL);
    expect(deviceOf(draw, 0, 0)).toEqual([16, 0]);
    expect(deviceOf(draw, 16, 16)).toEqual([0, 16]);
  });

  it("mirrors top to bottom with the vertical flip", async () => {
    const draw = await flipped(TILE_FLIP_VERTICAL);
    expect(deviceOf(draw, 0, 0)).toEqual([0, 16]);
    expect(deviceOf(draw, 16, 16)).toEqual([16, 0]);
  });

  it("transposes across the diagonal with the diagonal flip", async () => {
    const draw = await flipped(TILE_FLIP_DIAGONAL);
    expect(deviceOf(draw, 0, 0)).toEqual([0, 0]);
    expect(deviceOf(draw, 16, 0)).toEqual([0, 16]);
  });

  it("turns the tile a quarter turn with the diagonal and horizontal flips, as Tiled does", async () => {
    const draw = await flipped(TILE_FLIP_DIAGONAL | TILE_FLIP_HORIZONTAL);
    // A quarter turn clockwise: the top-left corner goes to the top-right.
    expect(deviceOf(draw, 0, 0)).toEqual([16, 0]);
    expect(deviceOf(draw, 16, 0)).toEqual([16, 16]);
  });

  it("applies all three flips to reach the opposite corner", async () => {
    const draw = await flipped(TILE_FLIP_DIAGONAL | TILE_FLIP_HORIZONTAL | TILE_FLIP_VERTICAL);
    expect(deviceOf(draw, 0, 0)).toEqual([16, 16]);
  });

  it("fills the cell with a diagonally flipped tile that is not square", async () => {
    const tileset = { tilewidth: 32, tileheight: 16, imagewidth: 128, imageheight: 32 };
    const gid = 2 + 0x20000000;
    const { loaded, layer } = await loadWith(tileLayer("ground", 1, 1, [gid]), { tileset });
    const recorder = new Recorder();
    drawTileLayer(recorder, loaded, layer, images, wholeMap, 0);
    const draw = recorder.draws[0];
    expect(draw).toBeDefined();
    if (draw === undefined) return;
    expect(deviceOf(draw, 0, 0)).toEqual([0, 0]);
    expect(deviceOf(draw, 32, 16)).toEqual([32, 16]);
  });

  it("restores the transform after a flipped tile, so the next tile is drawn as it is", async () => {
    const { loaded, layer } = await loadWith(tileLayer("ground", 2, 1, [2 + 0x80000000, 3]));
    const recorder = new Recorder();
    drawTileLayer(recorder, loaded, layer, images, wholeMap, 0);
    expect(recorder.draws[1]?.matrix).toEqual([1, 0, 0, 1, 0, 0]);
    expect(recorder.depth).toBe(0);
  });
});

describe("animated tiles", () => {
  it("draws the frame of an animated tile that is current at the time", async () => {
    const { loaded, layer } = await loadWith(tileLayer("ground", 1, 1, [4]));
    const early = new Recorder();
    drawTileLayer(early, loaded, layer, images, wholeMap, 0);
    expect(early.draws[0]?.source.slice(0, 2)).toEqual([48, 0]);
    const later = new Recorder();
    drawTileLayer(later, loaded, layer, images, wholeMap, 200);
    expect(later.draws[0]?.source.slice(0, 2)).toEqual([0, 16]);
    const looped = new Recorder();
    drawTileLayer(looped, loaded, layer, images, wholeMap, 300);
    expect(looped.draws[0]?.source.slice(0, 2)).toEqual([48, 0]);
  });
});

describe("opacity and state", () => {
  it("multiplies the layer's opacity into the alpha of each tile, and restores the alpha after", async () => {
    const { loaded, layer } = await loadWith(tileLayer("ground", 2, 1, [1, 1], { opacity: 0.5 }));
    const recorder = new Recorder();
    recorder.globalAlpha = 0.8;
    drawTileLayer(recorder, loaded, layer, images, wholeMap, 0);
    expect(recorder.draws.map((draw) => draw.alpha)).toEqual([0.4, 0.4]);
    expect(recorder.globalAlpha).toBe(0.8);
    expect(recorder.depth).toBe(0);
  });

  it("leaves the state balanced when a picture is missing, and names the tileset", async () => {
    const { loaded, layer } = await loadWith(tileLayer("ground", 1, 1, [1]));
    const recorder = new Recorder();
    expect(() => drawTileLayer(recorder, loaded, layer, {}, wholeMap, 0)).toThrow(TilesError);
    expect(() => drawTileLayer(recorder, loaded, layer, {}, wholeMap, 0)).toThrow(/"terrain"/);
    expect(recorder.depth).toBe(0);
    expect(recorder.globalAlpha).toBe(1);
  });

  it("does not need the picture of a tileset whose tiles are out of view", async () => {
    const { loaded, layer } = await loadWith(tileLayer("ground", 1, 1, [1]));
    const recorder = new Recorder();
    expect(() => drawTileLayer(recorder, loaded, layer, {}, { x: 500, y: 500, width: 10, height: 10 }, 0)).not.toThrow();
    expect(recorder.draws).toEqual([]);
  });

  it("refuses a view or a time that is not a finite number", async () => {
    const { loaded, layer } = await loadWith(tileLayer("ground", 1, 1, [1]));
    const recorder = new Recorder();
    expect(() => drawTileLayer(recorder, loaded, layer, images, { x: Number.NaN, y: 0, width: 1, height: 1 }, 0)).toThrow(TilesError);
    expect(() => drawTileLayer(recorder, loaded, layer, images, wholeMap, Infinity)).toThrow(TilesError);
  });
});

describe("parallax and tint", () => {
  /** A recorder whose context holds the camera's transform for a view at (x, y), as drawing expects. */
  function cameraAt(x: number, y: number): Recorder {
    const recorder = new Recorder();
    recorder.transform(1, 0, 0, 1, -x, -y);
    return recorder;
  }

  /** Where the top-left corner of each drawn tile lands on the device, sorted by x and then y. */
  function devicePlaces(recorder: Recorder): [number, number][] {
    return recorder.draws.map((draw) => deviceOf(draw, 0, 0)).sort((a, b) => a[0] - b[0] || a[1] - b[1]);
  }

  const ones = Array.from({ length: 100 }, () => 1);

  it("scrolls a layer by its parallax factor, and keeps its tiles where the map puts them", async () => {
    const { loaded, layer } = await loadWith(tileLayer("far", 10, 10, ones, { parallaxx: 0.5 }));
    const recorder = cameraAt(64, 0);
    drawTileLayer(recorder, loaded, layer, images, { x: 64, y: 0, width: 32, height: 32 }, 0);
    // With the camera at x 64 and a factor of 0.5, the layer shows the part from x 32: its tiles at x 32 and 48
    // land on the left of the screen.
    expect(devicePlaces(recorder)).toEqual([
      [0, 0],
      [0, 16],
      [16, 0],
      [16, 16],
    ]);
  });

  it("keeps a layer whose factor is 0 in place, whatever the camera does", async () => {
    const { loaded, layer } = await loadWith(tileLayer("sky", 10, 10, ones, { parallaxx: 0, parallaxy: 0 }));
    const recorder = cameraAt(200, 100);
    drawTileLayer(recorder, loaded, layer, images, { x: 200, y: 100, width: 32, height: 32 }, 0);
    expect(devicePlaces(recorder)).toEqual([
      [0, 0],
      [0, 16],
      [16, 0],
      [16, 16],
    ]);
  });

  it("draws a parallax layer where the map puts it when the camera is on the origin", async () => {
    const { loaded, layer } = await loadWith(tileLayer("far", 10, 10, ones, { parallaxx: 0.5, parallaxy: 0.5 }));
    const recorder = cameraAt(64, 32);
    drawTileLayer(recorder, loaded, layer, images, { x: 64, y: 32, width: 32, height: 32 }, 0, { x: 64, y: 32 });
    expect(devicePlaces(recorder)).toEqual([
      [0, 0],
      [0, 16],
      [16, 0],
      [16, 16],
    ]);
  });

  it("uses the map's parallax origin when the call gives none, and the given one when it does", async () => {
    const loaded = await loadTileMap(
      map({
        width: 10,
        height: 10,
        layers: [tileLayer("far", 10, 10, ones, { parallaxx: 0.5 })],
        extra: { parallaxoriginx: 64 },
      }),
    );
    const layer = findLayer(loaded, "far") as TileLayer;
    expect(loaded.parallaxOrigin).toEqual({ x: 64, y: 0 });
    // The origin is at x 64, so the camera at x 64 does not scroll the layer: its tiles at x 64 and 80 are on the left.
    const atOrigin = cameraAt(64, 0);
    drawTileLayer(atOrigin, loaded, layer, images, { x: 64, y: 0, width: 32, height: 32 }, 0);
    expect(devicePlaces(atOrigin)).toEqual([
      [0, 0],
      [0, 16],
      [16, 0],
      [16, 16],
    ]);
    // With the origin at 0 instead, the same view shows the layer at half speed, as in the first case.
    const atZero = cameraAt(64, 0);
    drawTileLayer(atZero, loaded, layer, images, { x: 64, y: 0, width: 32, height: 32 }, 0, { x: 0, y: 0 });
    expect(devicePlaces(atZero)).toEqual([
      [0, 0],
      [0, 16],
      [16, 0],
      [16, 16],
    ]);
    expect(atZero.draws.map((draw) => draw.dest.slice(0, 2)).sort((a, b) => a[0]! - b[0]! || a[1]! - b[1]!)).toEqual([
      [32, 0],
      [32, 16],
      [48, 0],
      [48, 16],
    ]);
  });

  it("does not apply a tint: a tinted layer is drawn with its opacity alone, and the tint stays data", async () => {
    const { loaded, layer } = await loadWith(tileLayer("ground", 1, 1, [1], { tintcolor: "#80ff0000" }));
    const recorder = new Recorder();
    drawTileLayer(recorder, loaded, layer, images, wholeMap, 0);
    expect(recorder.draws.map((draw) => draw.alpha)).toEqual([1]);
    expect(layer.tint).toBe("#80ff0000");
  });

  it("refuses a parallax origin that is not a finite number", async () => {
    const { loaded, layer } = await loadWith(tileLayer("ground", 1, 1, [1]));
    expect(() => drawTileLayer(new Recorder(), loaded, layer, images, wholeMap, 0, { x: Number.NaN, y: 0 })).toThrow(TilesError);
  });
});
