// Loads a Tiled map (.tmj, JSON) into a TileMap. The map's header is checked, then its tilesets (embedded,
// or external through the loader callback), then its layers. The tilesets are checked against each other:
// names are unique, and the ranges of global ids do not overlap. Every problem is collected, and one
// TilesError lists them all with their paths. The rules are in docs/formats/TILEMAP.md.

import { readLayers, TOP_LEVEL, type LayerContext } from "./layers";
import { readProperties } from "./properties";
import { Fields, isRecord, itemPath, Problems } from "./reader";
import { TilesError, type TilesIssue } from "./error";
import { readTileset } from "./tileset";
import type { TileMap, Tileset } from "./types";

/** Options for `loadTileMap`. */
export interface LoadOptions {
  /**
   * Loads an external tileset, for a map entry that has a `source` path. It receives the path as the map writes
   * it, and resolves a relative path against the map's own folder. It returns the parsed JSON of the .tsj file.
   * A map with external tilesets cannot be loaded without it.
   */
  loadTileset?(source: string): Promise<object>;
}

/** A tileset that was read, with the path of its entry in the map. */
interface TilesetEntry {
  readonly tileset: Tileset;
  readonly path: string;
}

/**
 * Loads a map.
 * @param input - the map's JSON text, or its parsed JSON object.
 * @param options - the loader for external tilesets.
 * @returns the map, with its tilesets in order of their first global id.
 * @throws TilesError with code "json" when the text is not JSON, "loader" when an external tileset cannot be
 * loaded, and otherwise the code of the first problem found. The error's `issues` lists every problem.
 */
export async function loadTileMap(input: string | object, options: LoadOptions = {}): Promise<TileMap> {
  const document = typeof input === "string" ? parseText(input) : input;
  if (!isRecord(document)) {
    throw new TilesError("structure", "a map must be a JSON object", [
      { code: "structure", message: "a map must be a JSON object" },
    ]);
  }

  const problems = new Problems();
  const root = new Fields(problems, document, "");
  checkType(root, "map");
  const orientation = root.string("orientation");
  if (orientation !== "orthogonal") {
    root.fail("unsupported", `the map is ${orientation}: only orthogonal maps are supported`, "orientation");
  }
  const infinite = root.bool("infinite", false);
  const width = infinite ? 0 : root.int("width", { min: 1 });
  const height = infinite ? 0 : root.int("height", { min: 1 });
  const tileWidth = root.int("tilewidth", { min: 1 });
  const tileHeight = root.int("tileheight", { min: 1 });
  const background = root.string("backgroundcolor", "");
  const properties = readProperties(problems, root.raw("properties"), root.at("properties"));

  const entries: TilesetEntry[] = [];
  const names = new Map<string, string>();
  for (const [index, item] of root.array("tilesets").entries()) {
    const entryAt = itemPath(root.at("tilesets"), index);
    if (!isRecord(item)) {
      problems.add("structure", "a tileset must be an object", entryAt);
      continue;
    }
    const entry = new Fields(problems, item, entryAt);
    const firstGid = entry.int("firstgid", { min: 1 });
    const tileset = entry.has("source")
      ? await loadExternal(problems, entry.string("source"), firstGid, options)
      : readTileset(problems, item, entryAt, firstGid);
    if (tileset === undefined) continue;
    // A repeated name is reported, but the tileset is kept so that its ids do not cause more problems.
    const earlier = names.get(tileset.name);
    if (earlier !== undefined) {
      problems.add("duplicate", `the tileset name "${tileset.name}" is already used by ${earlier}`, entryAt);
    } else {
      names.set(tileset.name, entryAt);
    }
    entries.push({ tileset, path: entryAt });
  }
  const tilesets = checkRanges(problems, entries);

  const context: LayerContext = { problems, tilesets, width, height, infinite };
  const layers = await readLayers(context, root.array("layers", true), root.at("layers"), TOP_LEVEL);

  if (problems.issues.length > 0) throw refuse(problems.issues);
  return {
    width,
    height,
    tileWidth,
    tileHeight,
    infinite,
    backgroundColor: background === "" ? undefined : background,
    properties,
    tilesets,
    layers,
  };
}

/** Parses the map's text. A text that is not JSON is refused with code "json". */
function parseText(text: string): unknown {
  try {
    return JSON.parse(text);
  } catch (error) {
    throw new TilesError("json", "the map text is not JSON", [{ code: "json", message: "the text is not JSON" }], {
      cause: error,
    });
  }
}

/** Checks the `type` field of a document, when it has one: a map, or a tileset. */
function checkType(fields: Fields, expected: string): void {
  if (!fields.has("type")) return;
  const type = fields.string("type");
  if (type !== expected) {
    fields.fail("value", `the document is not a ${expected}: its type is "${type}"`, "type");
  }
}

/**
 * Loads an external tileset through the loader and reads it. Its problems are reported with the source as the
 * prefix of their paths, such as `[tilesets/forest.tsj].tiles[2].id`.
 */
async function loadExternal(
  problems: Problems,
  source: string,
  firstGid: number,
  options: LoadOptions,
): Promise<Tileset | undefined> {
  if (options.loadTileset === undefined) {
    throw new TilesError(
      "loader",
      `the map uses the external tileset "${source}", but no loadTileset option was given`,
    );
  }
  let data: object;
  try {
    data = await options.loadTileset(source);
  } catch (error) {
    const detail = error instanceof Error ? error.message : String(error);
    throw new TilesError("loader", `could not load the tileset "${source}": ${detail}`, [], { cause: error });
  }
  if (!isRecord(data)) {
    problems.add("structure", "the tileset file must hold a JSON object", `[${source}]`);
    return undefined;
  }
  checkType(new Fields(problems, data, `[${source}]`), "tileset");
  return readTileset(problems, data, `[${source}]`, firstGid);
}

/**
 * Checks that the ranges of global ids of the tilesets do not overlap.
 * @returns the tilesets, in order of their first id.
 */
function checkRanges(problems: Problems, entries: readonly TilesetEntry[]): Tileset[] {
  const sorted = [...entries].sort((a, b) => a.tileset.firstGid - b.tileset.firstGid);
  for (let index = 1; index < sorted.length; index++) {
    const previous = sorted[index - 1];
    const current = sorted[index];
    if (previous === undefined || current === undefined) continue;
    const end = previous.tileset.firstGid + previous.tileset.tileCount;
    if (current.tileset.firstGid < end) {
      problems.add(
        "value",
        `the tileset "${current.tileset.name}" starts at global id ${current.tileset.firstGid}, ` +
          `inside "${previous.tileset.name}"`,
        `${current.path}.firstgid`,
      );
    }
  }
  return sorted.map((entry) => entry.tileset);
}

/** Makes the error of a refused map: its code is the first issue's, and it lists every issue. */
function refuse(issues: readonly TilesIssue[]): TilesError {
  const first = issues[0];
  if (first === undefined) return new TilesError("structure", "the map is refused");
  const where = first.path === undefined ? "" : ` at ${first.path}`;
  const count = issues.length === 1 ? "1 problem" : `${issues.length} problems`;
  return new TilesError(first.code, `the map has ${count}; the first is${where}: ${first.message}`, issues);
}
