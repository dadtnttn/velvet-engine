// Draws one tile layer to a Canvas 2D context. Only the cells whose tiles touch the view are drawn. A tile is
// drawn with its bottom-left corner on its cell's bottom-left corner, shifted by the layer's offset and the
// tileset's offset, as Tiled does, so a tile larger than the map grid rises above its cell.
//
// The context must already hold the camera transform: the drawing uses world (map) pixels, and `view` says
// which of them are visible. The transforms that the flips need are applied on top of the camera and undone
// afterwards. A layer with a parallax factor is moved by one more translation, which is undone the same way.
//
// Seams. A camera that scales or sits between pixels puts tile edges between device pixels, and two neighbours
// then leave a faint line between them. When the context tells its transform (getTransform) and that transform
// neither rotates nor skews, every tile's rectangle is snapped to whole device pixels, so neighbours share their
// edges exactly. Smoothing can still blend in the pixels next to a tile in the picture: pixel art draws with
// smoothing off, and a smoothed tileset keeps a spacing between its tiles with their edges repeated.

import { TilesError } from "./error";
import { resolveGid } from "./map";
import { animatedLocalId } from "./tileset";
import {
  TILE_FLIP_DIAGONAL,
  TILE_FLIP_HORIZONTAL,
  TILE_FLIP_VERTICAL,
  type TileLayer,
  type TileMap,
  type TilePoint,
  type Tileset,
} from "./types";

/**
 * The part of a Canvas 2D context that drawing uses. A `CanvasRenderingContext2D` satisfies it, and so does a
 * recording stand-in in tests.
 */
export interface TileDrawingContext {
  /** The opacity of what is drawn next, from 0 to 1. Drawing multiplies it into the layer's opacity. */
  globalAlpha: number;
  /** Saves the state (the transform and the opacity). */
  save(): void;
  /** Restores the state that `save` saved. */
  restore(): void;
  /** Multiplies the transform by a matrix. The last call applies to a point first. */
  transform(a: number, b: number, c: number, d: number, e: number, f: number): void;
  /**
   * The current transform, as `CanvasRenderingContext2D.getTransform` gives it. Optional: without it the tiles are
   * drawn where the map puts them, with no snapping to device pixels.
   */
  getTransform?(): TileTransform;
  /** Draws a part of an image into a rectangle, as `CanvasRenderingContext2D.drawImage` does. */
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
  ): void;
}

/** A 2D transform as `DOMMatrix` gives it: a point (x, y) goes to (a x + c y + e, b x + d y + f). */
export interface TileTransform {
  readonly a: number;
  readonly b: number;
  readonly c: number;
  readonly d: number;
  readonly e: number;
  readonly f: number;
}

/** Options of {@link drawTileLayer}. */
export interface TileDrawOptions {
  /**
   * Snaps every tile's rectangle to whole device pixels, so that neighbours leave no seam. Default true. It needs the
   * context's `getTransform`, and a transform that neither rotates nor skews; otherwise the tiles are not snapped.
   */
  readonly snap?: boolean;
}

/** One axis of a transform that only scales and moves: device = scale * map + offset. */
interface Axis {
  readonly scale: number;
  readonly offset: number;
}

/**
 * Snaps a span on one axis: the start and the length, in map pixels, of the span whose device edges are the
 * rounded device edges of `start` and `start + length`.
 */
function snapSpan(axis: Axis, start: number, length: number): readonly [number, number] {
  const first = Math.round(axis.scale * start + axis.offset);
  const last = Math.round(axis.scale * (start + length) + axis.offset);
  return [(first - axis.offset) / axis.scale, (last - first) / axis.scale];
}

/**
 * The two axes of the context's transform when snapping can use it: the context tells its transform, which neither
 * rotates nor skews and scales by a finite non-zero amount on both axes. Otherwise null.
 */
function snapAxes(ctx: TileDrawingContext): { readonly x: Axis; readonly y: Axis } | null {
  const matrix = ctx.getTransform?.();
  if (matrix === undefined) return null;
  if (matrix.b !== 0 || matrix.c !== 0 || matrix.a === 0 || matrix.d === 0) return null;
  if (![matrix.a, matrix.d, matrix.e, matrix.f].every((value) => Number.isFinite(value))) return null;
  return { x: { scale: matrix.a, offset: matrix.e }, y: { scale: matrix.d, offset: matrix.f } };
}

/** The pictures of the tilesets, by tileset name. Each tileset that a visible tile uses must have one. */
export type TileImages = Readonly<Record<string, CanvasImageSource>>;

/** The visible part of the map, in map pixels: the top-left corner and the size. */
export interface TileView {
  readonly x: number;
  readonly y: number;
  readonly width: number;
  readonly height: number;
}

/**
 * The part of a layer's own space that the view shows, for a layer with a parallax factor. The view is moved toward
 * the origin by the factor on each axis: a factor of 0 keeps the layer where the origin puts it, and a factor of 1
 * is the view itself.
 * @param view - the visible part of the map.
 * @param origin - the parallax origin.
 * @param parallax - the layer's factors.
 */
function parallaxView(view: TileView, origin: TilePoint, parallax: TilePoint): TileView {
  // A factor of 1 keeps the view's own value exactly, so that an axis without parallax has no rounding.
  return {
    x: parallax.x === 1 ? view.x : origin.x + (view.x - origin.x) * parallax.x,
    y: parallax.y === 1 ? view.y : origin.y + (view.y - origin.y) * parallax.y,
    width: view.width,
    height: view.height,
  };
}

/**
 * Draws the visible tiles of one layer.
 *
 * A layer whose parallax factor is not 1 scrolls at its own speed: with the camera at `view`, its tiles are drawn as
 * they would be at the view moved toward `origin` by the factor. Its tiles keep their places on the map. The tint of
 * a layer is not applied here: the game reads `layer.tint` and decides.
 *
 * @param ctx - the context, with the camera transform already applied.
 * @param map - the map that the layer belongs to.
 * @param layer - the tile layer to draw. A hidden layer, or one with zero opacity, draws nothing.
 * @param images - the picture of each tileset, by name.
 * @param view - the visible part of the map, in map pixels: where the camera is.
 * @param timeMs - the time in milliseconds, for animated tiles.
 * @param origin - the parallax origin: with the camera here, every layer is drawn where the map puts it. Defaults to
 * the map's `parallaxOrigin`, which is Tiled's, and to 0 and 0 when the map sets none.
 * @param options - whether tiles snap to device pixels (see {@link TileDrawOptions}).
 * @throws TilesError with code "missing-image" when a visible tile needs a tileset with no picture, and with code
 * "bad-input" when the view, the time or the origin is not a finite number.
 */
export function drawTileLayer(
  ctx: TileDrawingContext,
  map: TileMap,
  layer: TileLayer,
  images: TileImages,
  view: TileView,
  timeMs: number,
  origin: TilePoint = map.parallaxOrigin,
  options: TileDrawOptions = {},
): void {
  if (![view.x, view.y, view.width, view.height, timeMs, origin.x, origin.y].every((value) => Number.isFinite(value))) {
    throw new TilesError("bad-input", "the view, the time and the parallax origin must be finite numbers");
  }
  if (!layer.visible || layer.opacity <= 0) return;

  // The part of the layer's own space that is visible. Without parallax it is the view itself.
  const scrolled = layer.parallax.x !== 1 || layer.parallax.y !== 1;
  const area = scrolled ? parallaxView(view, origin, layer.parallax) : view;

  // The largest reach of a tile beyond its cell, so that cells just outside the view are considered too.
  let reachX = 0;
  let reachY = 0;
  for (const tileset of map.tilesets) {
    reachX = Math.max(reachX, tileset.tileWidth + Math.abs(tileset.offsetX));
    reachY = Math.max(reachY, tileset.tileHeight + Math.abs(tileset.offsetY));
  }
  const left = area.x - layer.offsetX;
  const top = area.y - layer.offsetY;
  const firstColumn = Math.floor((left - reachX) / map.tileWidth);
  const lastColumn = Math.floor((left + area.width + reachX) / map.tileWidth);
  const firstRow = Math.floor((top - reachY) / map.tileHeight) - 1;
  const lastRow = Math.floor((top + area.height + reachY) / map.tileHeight) + 1;

  const baseAlpha = ctx.globalAlpha;
  ctx.save();
  try {
    ctx.globalAlpha = baseAlpha * layer.opacity;
    // Moves the layer from the camera's place to its own: the context already holds the camera's transform.
    if (scrolled) ctx.transform(1, 0, 0, 1, view.x - area.x, view.y - area.y);
    // Read once, after the parallax move: every tile of the layer is drawn under this same transform.
    const axes = options.snap === false ? null : snapAxes(ctx);
    for (const chunk of layer.chunks) {
      const startColumn = Math.max(chunk.x, firstColumn);
      const endColumn = Math.min(chunk.x + chunk.width - 1, lastColumn);
      const startRow = Math.max(chunk.y, firstRow);
      const endRow = Math.min(chunk.y + chunk.height - 1, lastRow);
      for (let row = startRow; row <= endRow; row++) {
        for (let column = startColumn; column <= endColumn; column++) {
          const index = (row - chunk.y) * chunk.width + (column - chunk.x);
          const gid = chunk.gids[index] ?? 0;
          if (gid === 0) continue;
          const ref = resolveGid(map, gid);
          if (ref === undefined) continue;

          const tileset = ref.tileset;
          const shown = animatedLocalId(tileset, ref.localId, timeMs);
          const sourceColumn = shown % tileset.columns;
          const sourceRow = Math.floor(shown / tileset.columns);
          const sx = tileset.margin + sourceColumn * (tileset.tileWidth + tileset.spacing);
          const sy = tileset.margin + sourceRow * (tileset.tileHeight + tileset.spacing);
          const width = tileset.tileWidth;
          const height = tileset.tileHeight;
          const x = column * map.tileWidth + layer.offsetX + tileset.offsetX;
          const y = (row + 1) * map.tileHeight - height + layer.offsetY + tileset.offsetY;
          if (x + width <= area.x || x >= area.x + area.width) continue;
          if (y + height <= area.y || y >= area.y + area.height) continue;

          const image = imageOf(images, tileset);
          const flips = chunk.flips[index] ?? 0;
          const [dx, dw] = axes === null ? [x, width] : snapSpan(axes.x, x, width);
          const [dy, dh] = axes === null ? [y, height] : snapSpan(axes.y, y, height);
          if (flips === 0) {
            ctx.drawImage(image, sx, sy, width, height, dx, dy, dw, dh);
          } else {
            drawFlipped(ctx, image, { sx, sy, width, height, x: dx, y: dy, drawWidth: dw, drawHeight: dh, flips });
          }
        }
      }
    }
  } finally {
    ctx.restore();
  }
}

/**
 * The fields of a tile to draw: its place in the picture (`sx`, `sy`, `width`, `height`), the rectangle it fills on
 * the map (`x`, `y`, `drawWidth`, `drawHeight`, which snapping may make a little larger or smaller than the tile),
 * and its flip flags.
 */
interface TileDraw {
  readonly sx: number;
  readonly sy: number;
  readonly width: number;
  readonly height: number;
  readonly x: number;
  readonly y: number;
  readonly drawWidth: number;
  readonly drawHeight: number;
  readonly flips: number;
}

/**
 * Draws a flipped tile. The picture is drawn into the cell's rectangle, turned about its centre by the flips:
 * the diagonal flip first (it transposes the picture, so the picture is drawn in a rectangle with the width and
 * height swapped), then the horizontal flip, then the vertical one. The transforms are undone afterwards.
 */
function drawFlipped(ctx: TileDrawingContext, image: CanvasImageSource, tile: TileDraw): void {
  const centreX = tile.x + tile.drawWidth / 2;
  const centreY = tile.y + tile.drawHeight / 2;
  const diagonal = (tile.flips & TILE_FLIP_DIAGONAL) !== 0;
  const drawWidth = diagonal ? tile.drawHeight : tile.drawWidth;
  const drawHeight = diagonal ? tile.drawWidth : tile.drawHeight;

  ctx.save();
  try {
    // The calls apply in reverse: a point meets the last transform first, so the order is V, H, then D.
    if ((tile.flips & TILE_FLIP_VERTICAL) !== 0) ctx.transform(1, 0, 0, -1, 0, 2 * centreY);
    if ((tile.flips & TILE_FLIP_HORIZONTAL) !== 0) ctx.transform(-1, 0, 0, 1, 2 * centreX, 0);
    if (diagonal) ctx.transform(0, 1, 1, 0, centreX - centreY, centreY - centreX);
    ctx.drawImage(
      image,
      tile.sx,
      tile.sy,
      tile.width,
      tile.height,
      centreX - drawWidth / 2,
      centreY - drawHeight / 2,
      drawWidth,
      drawHeight,
    );
  } finally {
    ctx.restore();
  }
}

/** The picture of a tileset. Throws when the caller did not give one. */
function imageOf(images: TileImages, tileset: Tileset): CanvasImageSource {
  if (!Object.hasOwn(images, tileset.name)) {
    throw new TilesError("missing-image", `no picture was given for the tileset "${tileset.name}"`);
  }
  const image = images[tileset.name];
  if (image === undefined) {
    throw new TilesError("missing-image", `no picture was given for the tileset "${tileset.name}"`);
  }
  return image;
}
