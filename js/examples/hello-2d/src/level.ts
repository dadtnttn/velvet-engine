// The level: the Tiled map of the game, read by @velvet/tiles, and the things that the map's object layer places in
// it (the start, the checkpoints, the coins and the goal). Collision is built from the ground and the platforms
// (the one-way planks among them); the spikes are a grid of their own, so that touching them hurts but does not
// stop the hero.

import {
  buildCollisionGrid,
  findLayer,
  loadTileMap,
  type CollisionGrid,
  type PointObject,
  type TileMap,
} from "@velvet/tiles";
import { LAYERS } from "./config";

/** A point of the level, in level pixels. */
export interface LevelPoint {
  readonly x: number;
  readonly y: number;
}

/** A level as the game plays it. Build one with {@link loadLevel} or {@link levelFromMap}. */
export interface Level {
  /** The map as @velvet/tiles loaded it, for drawing. */
  readonly map: TileMap;
  /** The size of the map in level pixels. */
  readonly width: number;
  readonly height: number;
  /** The cells that block the hero: the ground, the stone and the one-way planks. */
  readonly solid: CollisionGrid;
  /** The cells that hurt the hero: the spikes. They do not block it. */
  readonly hazards: CollisionGrid;
  /** Where the hero starts. */
  readonly spawn: LevelPoint;
  /** Where the hero returns after a hit: the start first, then the checkpoints in order of x. */
  readonly checkpoints: readonly LevelPoint[];
  /** The coins, in the order of the file. */
  readonly coins: readonly LevelPoint[];
  /** The goal flag. Reaching it ends the run. */
  readonly goal: LevelPoint;
}

/** Raised when a map is loaded but does not have what the game needs (one start, one goal, some coins). */
export class LevelError extends Error {
  /** @param message - what is missing. */
  constructor(message: string) {
    super(message);
    this.name = "LevelError";
  }
}

/**
 * Reads a Tiled map and makes the level from it.
 * @param text - the text of the map (`.tmj`).
 * @param loadTileset - fetches an external tileset by the path the map gives it.
 * @throws TilesError when the map is not valid, and LevelError when it lacks a start, a goal or the coins.
 */
export async function loadLevel(text: string, loadTileset: (source: string) => Promise<object>): Promise<Level> {
  return levelFromMap(await loadTileMap(text, { loadTileset }));
}

/**
 * Makes the level from a loaded map.
 * @param map - the map, as @velvet/tiles loads it.
 * @throws LevelError when the object layer is missing, or the map does not have exactly one start and one goal,
 * or it has no coins.
 */
export function levelFromMap(map: TileMap): Level {
  const objects = findLayer(map, LAYERS.objects);
  if (objects === undefined || objects.kind !== "objects") {
    throw new LevelError(`the map has no object layer named "${LAYERS.objects}"`);
  }
  const points = objects.objects.filter((object): object is PointObject => object.shape === "point");
  const ofType = (type: string): PointObject[] => points.filter((point) => point.type === type);

  const starts = ofType("spawn");
  const goals = ofType("goal");
  const coins = ofType("coin");
  const checkpoints = ofType("checkpoint").sort((a, b) => a.x - b.x);
  const [start] = starts;
  const [goal] = goals;
  if (starts.length !== 1 || start === undefined) throw new LevelError("the map needs exactly one spawn point");
  if (goals.length !== 1 || goal === undefined) throw new LevelError("the map needs exactly one goal");
  if (coins.length === 0) throw new LevelError("the map has no coins");

  const at = (point: PointObject): LevelPoint => ({ x: point.x, y: point.y });
  return {
    map,
    width: map.width * map.tileWidth,
    height: map.height * map.tileHeight,
    solid: buildCollisionGrid(map, {
      layers: [LAYERS.ground, LAYERS.platforms],
      nonEmpty: true,
      oneWayProperty: "one_way",
    }),
    hazards: buildCollisionGrid(map, { layers: [LAYERS.hazards], nonEmpty: true }),
    spawn: at(start),
    checkpoints: [start, ...checkpoints].map(at),
    coins: coins.map(at),
    goal: at(goal),
  };
}
