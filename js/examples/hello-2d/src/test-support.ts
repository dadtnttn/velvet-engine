// Helpers for the tests of hello-2d. The package build never imports this file. The data files are read from the
// public folder, as the web server would serve them; a canvas gets a stand-in 2D context, because happy-dom has none;
// and small levels are drawn as text, one character per cell, with the same letters as the level's own design.

import { readFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { vi } from "vitest";
import type { SettingsBackend } from "@velvet/core";
import type { SpriteImage } from "@velvet/scene2d";
import { buildCollisionGrid, loadTileMap, type CollisionGrid } from "@velvet/tiles";
import { HERO, LAYERS } from "./config";
import { levelFromMap, type Level, type LevelPoint } from "./level";
import type { Hero } from "./physics";

const HERE = dirname(fileURLToPath(import.meta.url));

/** The public folder of the game: its data files are served from here. */
export const PUBLIC_DIR = resolve(HERE, "..", "public");

/** A file of the public folder, as text. */
export function readPublic(path: string): Promise<string> {
  return readFile(resolve(PUBLIC_DIR, path), "utf8");
}

/** A file of the public folder, as bytes. */
export function readPublicBytes(path: string): Promise<Buffer> {
  return readFile(resolve(PUBLIC_DIR, path));
}

/**
 * Makes `fetch` read the game's files from the public folder, as the web server would. A missing file answers 404.
 */
export function fetchFromPublicFolder(): void {
  vi.stubGlobal("fetch", async (url: string) => {
    try {
      const text = await readFile(resolve(PUBLIC_DIR, url), "utf8");
      return { ok: true, status: 200, text: async () => text };
    } catch {
      return { ok: false, status: 404, text: async () => "" };
    }
  });
}

/** Settings and progress kept in a plain object: the same shape as the bridge's settings, and nothing survives a test. */
export function memoryBackend(): SettingsBackend & { readonly values: Map<string, unknown> } {
  const values = new Map<string, unknown>();
  return {
    values,
    async get(key: string): Promise<unknown> {
      return values.has(key) ? values.get(key) : null;
    },
    async set(key: string, value: unknown): Promise<void> {
      if (value === null) values.delete(key);
      else values.set(key, value);
    },
  };
}

/**
 * A stand-in 2D context: every method exists and does nothing, every property can be written, and the text measure
 * gives a fixed width. It lets the scene draw in a page that has no canvas.
 */
export function fakeContext(): object {
  const target: Record<string, unknown> = {
    globalAlpha: 1,
    imageSmoothingEnabled: true,
    measureText: () => ({ width: 10, fontBoundingBoxAscent: 8, fontBoundingBoxDescent: 2 }),
  };
  return new Proxy(target, {
    get(object, property) {
      if (typeof property === "string" && property in object) return object[property];
      return () => undefined;
    },
    set(object, property, value: unknown) {
      object[String(property)] = value;
      return true;
    },
  });
}

/** A canvas element whose 2D context is the stand-in of {@link fakeContext}. */
export function fakeCanvas(): HTMLCanvasElement {
  const canvas = document.createElement("canvas");
  const context = fakeContext();
  Object.defineProperty(canvas, "getContext", { value: () => context });
  return canvas;
}

/**
 * A picture for the sprites in a test: a canvas that stands for an image. The sprites take its size and never draw it.
 */
export async function fakeImage(): Promise<SpriteImage> {
  const canvas = document.createElement("canvas");
  canvas.width = 1;
  canvas.height = 1;
  return canvas;
}

/** Waits, in real time, until `check` is true. Fails when that takes longer than `timeoutMs`. */
export async function waitUntil(check: () => boolean, timeoutMs = 4000): Promise<void> {
  const deadline = Date.now() + timeoutMs;
  while (!check()) {
    if (Date.now() > deadline) throw new Error("timed out waiting for the page");
    await new Promise((done) => setTimeout(done, 20));
  }
}

/** The first element matching a selector. A test that asks for a missing element fails here. */
export function must<T extends Element>(root: ParentNode, selector: string): T {
  const found = root.querySelector<T>(selector);
  if (found === null) throw new Error(`no element matches ${selector}`);
  return found;
}

/** The tile ids of the test tileset, as the real tileset numbers them. */
const TEST_TILES = { grass: 0, stone: 2, plank: 3, spikes: 4, hill: 7, cloud: 8 } as const;

/** The gid of a test tile: the tileset starts at gid 1. */
const gidOf = (tile: number): number => tile + 1;

/** The tileset that the test maps use: the same picture size and the plank's one-way property. */
function testTileset(): object {
  return {
    name: "tiles",
    firstgid: 1,
    tilewidth: 32,
    tileheight: 32,
    tilecount: 9,
    columns: 5,
    image: "../art/tiles.png",
    imagewidth: 160,
    imageheight: 64,
    margin: 0,
    spacing: 0,
    tiles: [{ id: TEST_TILES.plank, properties: [{ name: "one_way", type: "bool", value: true }] }],
  };
}

/**
 * Draws a Tiled map (as the JSON object that `loadTileMap` reads) from rows of text, one character per 32-pixel cell.
 * `#` is ground, `=` stone, `-` a one-way plank, `^` spikes, `b` a hill and `c` a cloud. The points are `S` (the
 * start), `C` (a checkpoint), `F` (the goal) and `o` (a coin), each at the bottom centre of its cell, or the centre for a
 * coin. Rows are padded to the widest row.
 * @param rows - the rows of the map, top first.
 */
export function mapObjectFromRows(rows: readonly string[]): Record<string, unknown> {
  const width = Math.max(...rows.map((row) => row.length));
  const height = rows.length;
  const cell = (row: number, column: number): string => rows[row]?.[column] ?? ".";
  const data = (pick: (char: string, row: number, column: number) => number | null): number[] => {
    const out: number[] = [];
    for (let row = 0; row < height; row += 1) {
      for (let column = 0; column < width; column += 1) out.push(pick(cell(row, column), row, column) ?? 0);
    }
    return out;
  };
  const objects: Record<string, unknown>[] = [];
  let id = 1;
  rows.forEach((text, row) => {
    for (let column = 0; column < text.length; column += 1) {
      const char = text[column] ?? ".";
      const kinds: Record<string, string> = { S: "spawn", C: "checkpoint", F: "goal", o: "coin" };
      const type = kinds[char];
      if (type === undefined) continue;
      const bottom = char !== "o";
      objects.push({
        id: id++,
        name: type,
        type,
        point: true,
        x: column * 32 + 16,
        y: bottom ? row * 32 + 32 : row * 32 + 16,
        width: 0,
        height: 0,
        rotation: 0,
        visible: true,
      });
    }
  });
  const layer = (name: string, pick: (char: string, row: number, column: number) => number | null) => ({
    id: 0,
    name,
    type: "tilelayer",
    width,
    height,
    x: 0,
    y: 0,
    opacity: 1,
    visible: true,
    data: data(pick),
  });
  return {
    type: "map",
    version: "1.10",
    orientation: "orthogonal",
    infinite: false,
    width,
    height,
    tilewidth: 32,
    tileheight: 32,
    tilesets: [testTileset()],
    layers: [
      layer(LAYERS.background, (char) => (char === "b" ? gidOf(TEST_TILES.hill) : char === "c" ? gidOf(TEST_TILES.cloud) : null)),
      layer(LAYERS.ground, (char, row, column) => {
        if (char === "#") return gidOf(cell(row - 1, column) === "#" ? 1 : TEST_TILES.grass);
        if (char === "=") return gidOf(TEST_TILES.stone);
        return null;
      }),
      layer(LAYERS.platforms, (char) => (char === "-" ? gidOf(TEST_TILES.plank) : null)),
      layer(LAYERS.hazards, (char) => (char === "^" ? gidOf(TEST_TILES.spikes) : null)),
      { id: 0, name: LAYERS.objects, type: "objectgroup", draworder: "topdown", x: 0, y: 0, opacity: 1, visible: true, objects },
    ],
  };
}

/**
 * Loads a small level drawn as text (see {@link mapObjectFromRows}), with the same rules as the real level.
 * @param rows - the rows of the level.
 * @throws LevelError when the rows lack a start, a goal or a coin.
 */
export async function levelFromRows(rows: readonly string[]): Promise<Level> {
  return levelFromMap(await loadTileMap(mapObjectFromRows(rows)));
}

/** The level of the game, read from its map and tileset in the public folder. */
export async function loadShippedLevel(): Promise<Level> {
  const map = await readPublic("maps/level1.tmj");
  const tileset = JSON.parse(await readPublic("maps/tiles.tsj")) as object;
  return levelFromMap(await loadTileMap(map, { loadTileset: async () => tileset }));
}

/** The collision of a text level and its start, for tests of the hero's motion, which need no goal or coins. */
export interface Ground {
  /** The solid cells, built from the ground and the planks as the game builds them. */
  readonly solid: CollisionGrid;
  /** The bottom centre of the `S` cell, or the origin when the rows have none. */
  readonly spawn: LevelPoint;
}

/**
 * Builds the collision of a text level (see {@link mapObjectFromRows}), with no goal and no coins required.
 * @param rows - the rows of the level, top first.
 */
export async function groundFromRows(rows: readonly string[]): Promise<Ground> {
  const map = await loadTileMap(mapObjectFromRows(rows));
  const solid = buildCollisionGrid(map, { layers: [LAYERS.ground, LAYERS.platforms], nonEmpty: true, oneWayProperty: "one_way" });
  let spawn: LevelPoint = { x: 0, y: 0 };
  rows.forEach((text, row) => {
    const column = text.indexOf("S");
    if (column >= 0) spawn = { x: column * 32 + 16, y: row * 32 + 32 };
  });
  return { solid, spawn };
}

/** How close, in pixels, a gap or a wall ahead must be before the bot jumps. */
const BOT_NEAR = 8;
/** How close the spikes ahead must be: the hero needs a longer run-up to rise above them before it reaches them. */
const BOT_SPIKE_NEAR = 26;

/**
 * Whether the scripted bot of the playthrough should jump now: it jumps when a gap, a wall or spikes are just ahead
 * of the hero's front, and the hero is on the ground. It reads the level and the hero, and nothing else.
 * @param level - the level's collision and hazards.
 * @param hero - the hero now.
 */
export function wantsJump(level: Pick<Ground, "solid"> & { readonly hazards: CollisionGrid }, hero: Hero): boolean {
  if (!hero.grounded) return false;
  const tile = level.solid.tileWidth;
  const front = hero.x + HERO.width;
  const footRow = Math.floor((hero.y + HERO.height - 0.5) / tile);
  const column = Math.floor(front / tile) + 1;
  const distance = column * tile - front;
  if (distance <= BOT_NEAR && level.solid.kindAt(column, footRow + 1) === "empty") return true;
  if (distance <= BOT_NEAR && level.solid.kindAt(column, footRow) !== "empty") return true;
  return distance <= BOT_SPIKE_NEAR && level.hazards.kindAt(column, footRow) !== "empty";
}
