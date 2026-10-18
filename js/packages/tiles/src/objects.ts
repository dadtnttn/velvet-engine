// Reads objects: the entries of an object layer, and the objects of a tile's collision group. The shape
// is read from the flags Tiled writes: text, then a tile (a gid), then a point, an ellipse, a polygon or
// a polyline, and a rectangle when none of those is set. Object templates are refused.

import { splitGid, GID_LIMIT } from "./gid";
import { readProperties } from "./properties";
import { Fields, readItems, type Problems } from "./reader";
import type { MapObject, TileObjectBase, TilePoint, TileText } from "./types";

/** Options for reading objects. */
export interface ObjectOptions {
  /**
   * Says whether a global tile id names a tile of the map. An object that places a tile with an id that the
   * map does not have is refused. Left out for the objects of a tile's collision group, whose ids are not checked.
   */
  readonly knownGid?: (gid: number) => boolean;
}

/** The text alignments that Tiled allows for a text object. */
const HALIGN = ["left", "center", "right", "justify"] as const;
/** The vertical alignments that Tiled allows for a text object. */
const VALIGN = ["top", "center", "bottom"] as const;

/**
 * Reads a list of objects.
 * @param problems - where the problems go.
 * @param list - the JSON list.
 * @param path - the path of the list in the document.
 * @param options - the checks to apply.
 * @returns the objects that read cleanly, in the order of the list.
 */
export function readObjects(
  problems: Problems,
  list: readonly unknown[],
  path: string,
  options: ObjectOptions = {},
): MapObject[] {
  return readItems(problems, list, path, "an object", (fields) => readObject(fields, options));
}

/**
 * Reads one object.
 * @returns the object, or undefined when it is a template (refused) or has no valid shape.
 */
function readObject(fields: Fields, options: ObjectOptions): MapObject | undefined {
  if (fields.has("template")) {
    fields.fail("unsupported", "object templates are not supported: put the object's fields in the map", "template");
    return undefined;
  }
  const base: TileObjectBase = {
    id: fields.int("id", { min: 0, default: 0 }),
    name: fields.string("name", ""),
    type: fields.has("type") ? fields.string("type") : fields.string("class", ""),
    x: fields.number("x", { default: 0 }),
    y: fields.number("y", { default: 0 }),
    width: fields.number("width", { min: 0, default: 0 }),
    height: fields.number("height", { min: 0, default: 0 }),
    rotation: fields.number("rotation", { default: 0 }),
    visible: fields.bool("visible", true),
    properties: readProperties(fields.recorder, fields.raw("properties"), fields.at("properties")),
  };

  if (fields.has("text")) {
    const text = fields.object("text", true);
    if (text === undefined) return undefined;
    return { ...base, shape: "text", text: readText(text) };
  }
  if (fields.has("gid")) {
    const split = splitGid(fields.int("gid", { min: 1, max: GID_LIMIT }));
    if (split.hexRotation) {
      fields.fail("value", "the hexagonal rotation flag is only for hexagonal maps", "gid");
    }
    if (options.knownGid !== undefined && !options.knownGid(split.gid)) {
      fields.fail("value", `the tile id ${split.gid} is not a tile of any tileset`, "gid");
    }
    return { ...base, shape: "tile", gid: split.gid, flips: split.flips };
  }
  if (fields.bool("point", false)) return { ...base, shape: "point" };
  if (fields.bool("ellipse", false)) return { ...base, shape: "ellipse" };
  if (fields.has("polygon")) {
    return { ...base, shape: "polygon", points: readPoints(fields, "polygon", 3) };
  }
  if (fields.has("polyline")) {
    return { ...base, shape: "polyline", points: readPoints(fields, "polyline", 2) };
  }
  return { ...base, shape: "rectangle" };
}

/** Reads the points of a polygon or a polyline: each is `{ x, y }`, at least `minimum` of them. */
function readPoints(fields: Fields, key: string, minimum: number): TilePoint[] {
  const list = fields.array(key, true);
  const points = readItems(fields.recorder, list, fields.at(key), "a point", (point) => ({
    x: point.number("x", { default: 0 }),
    y: point.number("y", { default: 0 }),
  }));
  if (list.length < minimum) {
    fields.fail("value", `"${key}" needs at least ${minimum} points, got ${list.length}`, key);
  }
  return points;
}

/** Reads the text of a text object, with Tiled's defaults for the fields that are left out. */
function readText(fields: Fields): TileText {
  return {
    text: fields.string("text", ""),
    fontFamily: fields.string("fontfamily", "sans-serif"),
    pixelSize: fields.int("pixelsize", { min: 1, default: 16 }),
    wrap: fields.bool("wrap", false),
    color: fields.string("color", "#000000"),
    bold: fields.bool("bold", false),
    italic: fields.bool("italic", false),
    underline: fields.bool("underline", false),
    strikeout: fields.bool("strikeout", false),
    kerning: fields.bool("kerning", true),
    halign: fields.choice("halign", HALIGN, "left"),
    valign: fields.choice("valign", VALIGN, "top"),
  };
}
