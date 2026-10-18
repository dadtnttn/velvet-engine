// The pictures of the game: the tileset, the hero's sheet and the coin's sheet, loaded once. The sheets are read
// with the atlas readers of @velvet/scene2d, so their frames are the same rectangles the files name.

import { parseAseprite, parseTexturePacker, type Atlas, type AsepriteSheet, type SpriteFrame, type SpriteImage } from "@velvet/scene2d";
import type { Tileset } from "@velvet/tiles";
import { ART_URLS } from "./config";

/** The pictures and sheets of the game, ready to draw. */
export interface Art {
  /** The tileset's picture: the ground, the planks, the spikes, the coin and the flag. */
  readonly tiles: SpriteImage;
  /** The hero's picture, with its frames in the TexturePacker atlas. */
  readonly hero: SpriteImage;
  readonly heroAtlas: Atlas;
  /** The coin's picture, with the Aseprite export whose tag "spin" lists its frames. */
  readonly coins: SpriteImage;
  readonly coinSheet: AsepriteSheet;
}

/** Loads one picture, by its URL. */
export type ImageLoader = (url: string) => Promise<SpriteImage>;

/**
 * Loads a picture in the browser: an image element that is decoded before it is used.
 * @param url - the address of the picture.
 */
export async function loadImageElement(url: string): Promise<HTMLImageElement> {
  const image = new Image();
  image.src = url;
  await image.decode();
  return image;
}

/**
 * Loads every picture and sheet of the game.
 * @param options - how to load a picture and how to read a text file.
 * @throws when a file cannot be loaded, or a sheet has a wrong shape.
 */
export async function loadArt(options: {
  readonly loadImage: ImageLoader;
  readonly fetchText: (url: string) => Promise<string>;
}): Promise<Art> {
  const [tiles, hero, coins, heroText, coinText] = await Promise.all([
    options.loadImage(ART_URLS.tiles),
    options.loadImage(ART_URLS.hero),
    options.loadImage(ART_URLS.coins),
    options.fetchText(ART_URLS.heroAtlas),
    options.fetchText(ART_URLS.coinSheet),
  ]);
  return {
    tiles,
    hero,
    coins,
    heroAtlas: parseTexturePacker(heroText),
    coinSheet: parseAseprite(coinText),
  };
}

/**
 * The part of a tileset's picture that one tile occupies.
 * @param tileset - the tileset, as the map loaded it.
 * @param tile - the tile's local id: its index in the tileset.
 */
export function tileFrame(tileset: Tileset, tile: number): SpriteFrame {
  const column = tile % tileset.columns;
  const row = Math.floor(tile / tileset.columns);
  return {
    x: tileset.margin + column * (tileset.tileWidth + tileset.spacing),
    y: tileset.margin + row * (tileset.tileHeight + tileset.spacing),
    width: tileset.tileWidth,
    height: tileset.tileHeight,
  };
}
