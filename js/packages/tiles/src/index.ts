// @velvet/tiles: tile maps for games. Reads Tiled maps (JSON, orthogonal) and their tilesets, builds
// collision grids from them, moves boxes across the grid without tunnelling, finds paths with A*, and draws
// the visible tiles of a layer to a Canvas 2D context. See README.md and docs/formats/TILEMAP.md.
//
// Import from the package root, for example:
//   import { loadTileMap, buildCollisionGrid, moveBox, PathSearch, drawTileLayer } from "@velvet/tiles";

export { TilesError } from "./error";
export type { TilesErrorCode, TilesIssue } from "./error";

export { loadTileMap } from "./load";
export type { LoadOptions } from "./load";
export { findLayer, resolveGid, tileLayers } from "./map";
export { animatedLocalId } from "./tileset";

export { TILE_FLIP_DIAGONAL, TILE_FLIP_HORIZONTAL, TILE_FLIP_VERTICAL } from "./types";
export type {
  Box,
  EllipseObject,
  ImageLayer,
  GroupLayer,
  Layer,
  LayerBase,
  MapObject,
  ObjectLayer,
  PointObject,
  PolygonObject,
  PolylineObject,
  RectangleObject,
  TextObject,
  TileChunk,
  TileFrame,
  TileInfo,
  TileLayer,
  TileMap,
  TileObject,
  TileObjectBase,
  TilePoint,
  TileProperties,
  TileProperty,
  TileRef,
  TileText,
  Tileset,
} from "./types";

export { CollisionGrid, buildCollisionGrid } from "./collision";
export type { CellKind, CollisionOptions } from "./collision";

export { moveBox } from "./movement";
export type { MoveOptions, MoveResult, Side, TouchedTile } from "./movement";

export { PathSearch, findPath } from "./pathfinding";
export type { PathCell, PathOptions } from "./pathfinding";

export { drawTileLayer } from "./draw";
export type { TileDrawOptions, TileDrawingContext, TileImages, TileTransform, TileView } from "./draw";
