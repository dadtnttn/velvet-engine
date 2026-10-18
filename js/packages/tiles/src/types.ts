// The types of @velvet/tiles: maps, layers, chunks, objects, properties, tilesets and tiles, as the
// reader returns them. Every value is plain data and readonly. Positions and sizes are in pixels, and
// cells are counted in map tiles, as each field says. Y grows downward, as in Tiled and on a canvas.

/** The flip bit of a cell: the tile is mirrored left to right. */
export const TILE_FLIP_HORIZONTAL = 1;
/** The flip bit of a cell: the tile is mirrored top to bottom. */
export const TILE_FLIP_VERTICAL = 2;
/** The flip bit of a cell: the tile is transposed across its diagonal. It is applied before the other two. */
export const TILE_FLIP_DIAGONAL = 4;

/** A point in pixels. */
export interface TilePoint {
  readonly x: number;
  readonly y: number;
}

/** A rectangle in pixels: its top-left corner, and its size. Y grows downward. */
export interface Box {
  readonly x: number;
  readonly y: number;
  readonly width: number;
  readonly height: number;
}

/** A custom property, typed the way Tiled types it. A property without a type is a string. */
export type TileProperty =
  | { readonly type: "string"; readonly value: string }
  | { readonly type: "int"; readonly value: number }
  | { readonly type: "float"; readonly value: number }
  | { readonly type: "bool"; readonly value: boolean }
  /** `#AARRGGBB` text, or "" for a colour that is not set. */
  | { readonly type: "color"; readonly value: string }
  /** A path, as the map writes it. */
  | { readonly type: "file"; readonly value: string }
  /** The id of an object in the map, or 0 for none. */
  | { readonly type: "object"; readonly value: number }
  /**
   * A class: its name in the project, and its members. The members are typed by their JSON type, because
   * the map does not carry the class definition (see docs/formats/TILEMAP.md).
   */
  | { readonly type: "class"; readonly propertyType: string; readonly value: TileProperties };

/** Custom properties, by name. */
export type TileProperties = Readonly<Record<string, TileProperty>>;

/** The text of a text object, with Tiled's defaults where the map leaves a field out. */
export interface TileText {
  readonly text: string;
  readonly fontFamily: string;
  readonly pixelSize: number;
  readonly wrap: boolean;
  readonly color: string;
  readonly bold: boolean;
  readonly italic: boolean;
  readonly underline: boolean;
  readonly strikeout: boolean;
  readonly kerning: boolean;
  readonly halign: "left" | "center" | "right" | "justify";
  readonly valign: "top" | "center" | "bottom";
}

/** The fields that every object of an object layer, or of a tile's collision group, has. */
export interface TileObjectBase {
  /** The object's id in its map, unique there. 0 when the map does not give one. */
  readonly id: number;
  readonly name: string;
  /** Tiled's type (the class name in older files). "" when none. */
  readonly type: string;
  /** Left edge in pixels. For a tile object this is the bottom-left corner, as Tiled writes it. */
  readonly x: number;
  /** Top edge in pixels. For a tile object this is the bottom-left corner, as Tiled writes it. */
  readonly y: number;
  readonly width: number;
  readonly height: number;
  /** Degrees, clockwise, around the object's pivot (its top-left corner; a tile object pivots at its bottom-left). */
  readonly rotation: number;
  readonly visible: boolean;
  readonly properties: TileProperties;
}

/** A rectangle: the default shape of an object. */
export interface RectangleObject extends TileObjectBase {
  readonly shape: "rectangle";
}

/** A point: no area. */
export interface PointObject extends TileObjectBase {
  readonly shape: "point";
}

/** An ellipse, inside its width and height. */
export interface EllipseObject extends TileObjectBase {
  readonly shape: "ellipse";
}

/** A closed polygon of at least three points, relative to the object's position. */
export interface PolygonObject extends TileObjectBase {
  readonly shape: "polygon";
  readonly points: readonly TilePoint[];
}

/** An open line of at least two points, relative to the object's position. */
export interface PolylineObject extends TileObjectBase {
  readonly shape: "polyline";
  readonly points: readonly TilePoint[];
}

/** A tile placed as an object. `gid` has no flip flags; `flips` holds them. */
export interface TileObject extends TileObjectBase {
  readonly shape: "tile";
  readonly gid: number;
  readonly flips: number;
}

/** Text, kept as data: the drawing is up to the game. */
export interface TextObject extends TileObjectBase {
  readonly shape: "text";
  readonly text: TileText;
}

/** Any object of an object layer, or of a tile's collision group. */
export type MapObject =
  | RectangleObject
  | PointObject
  | EllipseObject
  | PolygonObject
  | PolylineObject
  | TileObject
  | TextObject;

/** A block of cells of a tile layer. A finite layer has one chunk, at 0,0, the size of the map. */
export interface TileChunk {
  /** The column of the chunk's first cell, in map cells. Negative in an infinite map. */
  readonly x: number;
  /** The row of the chunk's first cell, in map cells. Negative in an infinite map. */
  readonly y: number;
  readonly width: number;
  readonly height: number;
  /**
   * The global tile id of each cell, row by row, with the flip flags removed. 0 is an empty cell. The id
   * names a tile through `resolveGid`.
   */
  readonly gids: Uint32Array;
  /** The flip flags of each cell (`TILE_FLIP_*` bits), in the same order as `gids`. */
  readonly flips: Uint8Array;
}

/**
 * The fields that every layer has. The visibility, the opacity and the offset are the values that the layer is
 * drawn with.
 */
export interface LayerBase {
  /** The layer's id in its map. 0 when the map does not give one. */
  readonly id: number;
  readonly name: string;
  /** Whether the layer is shown, with the visibility of its groups combined in. */
  readonly visible: boolean;
  /** The opacity from 0 to 1, with the opacities of its groups multiplied in. */
  readonly opacity: number;
  /** The horizontal offset in pixels, with the offsets of its groups added in. Drawing uses it; collision does not. */
  readonly offsetX: number;
  /** The vertical offset in pixels, with the offsets of its groups added in. Used for drawing; collision ignores it. */
  readonly offsetY: number;
  readonly properties: TileProperties;
}

/** A layer of tile cells. */
export interface TileLayer extends LayerBase {
  readonly kind: "tiles";
  readonly chunks: readonly TileChunk[];
}

/** A layer of objects. */
export interface ObjectLayer extends LayerBase {
  readonly kind: "objects";
  /** "topdown" sorts the objects by y when drawn; "index" keeps the file's order. */
  readonly drawOrder: "topdown" | "index";
  readonly objects: readonly MapObject[];
}

/** A picture laid over the map, kept as data: its path is relative to the map file. */
export interface ImageLayer extends LayerBase {
  readonly kind: "image";
  readonly image: string;
  readonly repeatX: boolean;
  readonly repeatY: boolean;
  /** The colour that is left out when drawing the picture, as `#RRGGBB` text, or undefined. */
  readonly transparentColor: string | undefined;
}

/** A group of layers. Its own visibility, opacity and offset are combined into each layer it holds. */
export interface GroupLayer extends LayerBase {
  readonly kind: "group";
  readonly layers: readonly Layer[];
}

/** Any layer of a map. */
export type Layer = TileLayer | ObjectLayer | ImageLayer | GroupLayer;

/** One frame of a tile animation. */
export interface TileFrame {
  /** The local id of the tile to show, in the same tileset. */
  readonly tileId: number;
  /** How long the frame shows, in milliseconds. A whole number from 1. */
  readonly duration: number;
}

/** What a tileset says about one of its tiles. Only tiles with something to say have an entry. */
export interface TileInfo {
  /** The tile's local id: its index in the tileset, from 0. */
  readonly id: number;
  /** Tiled's type (the class name in older files). "" when none. */
  readonly type: string;
  readonly properties: TileProperties;
  /**
   * The objects of the tile's collision group, in pixels from the tile image's top-left corner. Empty when the
   * tile has no collision group.
   */
  readonly collision: readonly MapObject[];
  /** The animation frames, in order. Empty for a tile that does not animate. */
  readonly animation: readonly TileFrame[];
}

/** A tileset: a picture cut into a grid of tiles, with the global ids that it owns. */
export interface Tileset {
  /** The name. It is unique in the map and is the key of the images that drawing takes. */
  readonly name: string;
  /** The global id of the tileset's first tile. Global ids run from here to `firstGid + tileCount - 1`. */
  readonly firstGid: number;
  readonly tileCount: number;
  /** The size of one tile image in pixels. It may differ from the map's tile size. */
  readonly tileWidth: number;
  readonly tileHeight: number;
  readonly columns: number;
  /** The path of the picture, as the tileset writes it. It is relative to the tileset's own file. */
  readonly image: string;
  /** The size of the picture in pixels, as the tileset writes it (0 when it does not). */
  readonly imageWidth: number;
  readonly imageHeight: number;
  /** Pixels between the picture's edge and its first tile. */
  readonly margin: number;
  /** Pixels between two tiles of the picture. */
  readonly spacing: number;
  /** The pixel offset of the tile images when they are drawn (Tiled's tile offset). */
  readonly offsetX: number;
  readonly offsetY: number;
  readonly properties: TileProperties;
  /** The tiles that say something, by local id. */
  readonly tiles: ReadonlyMap<number, TileInfo>;
}

/** The result of `resolveGid`: which tileset owns a global id, and which of its tiles it is. */
export interface TileRef {
  readonly tileset: Tileset;
  /** The tile's local id: the global id less the tileset's first id. */
  readonly localId: number;
  /** What the tileset says about the tile, or undefined when it says nothing. */
  readonly info: TileInfo | undefined;
}

/** A tile map: its size and tile size, its tilesets and its layers. */
export interface TileMap {
  /** The width in cells of a finite map. 0 for an infinite map, whose cells are in its chunks. */
  readonly width: number;
  /** The height in cells of a finite map. 0 for an infinite map. */
  readonly height: number;
  /** The size of a map cell in pixels. */
  readonly tileWidth: number;
  readonly tileHeight: number;
  readonly infinite: boolean;
  /** The background colour as `#AARRGGBB` text, or undefined when the map sets none. */
  readonly backgroundColor: string | undefined;
  readonly properties: TileProperties;
  /** The tilesets, in order of their first global id. */
  readonly tilesets: readonly Tileset[];
  /** The layers, in the order of the file, with the layers of a group inside the group. */
  readonly layers: readonly Layer[];
}
