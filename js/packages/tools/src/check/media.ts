// The formats whose files name other files of the game. An audio manifest names its sound files, and a Tiled map
// or tileset names the pictures of its tilesets and the external tilesets of a map. Each named file must exist in
// the game, spelled as it is named (see game/files.ts). The formats themselves are read by their own packages.

import { readFile } from "node:fs/promises";
import { join, posix, resolve } from "node:path";
import type { AudioManifest } from "@velvet/audio";
import { loadTileMap, type TileMap, type Tileset } from "@velvet/tiles";
import { makeIssue, WHOLE_FILE, type Issue } from "../issues";
import type { GameDocument } from "../game/discover";
import type { FileIndex } from "../game/files";
import { failureIssues } from "./failures";

/**
 * The items of a list field of a parsed JSON value.
 * @param value - the parsed value (any JSON value).
 * @param key - the field's name.
 * @returns the items, or an empty list when the value has no list under that name.
 */
function listField(value: unknown, key: string): unknown[] {
  const list = typeof value === "object" && value !== null ? Reflect.get(value, key) : undefined;
  return Array.isArray(list) ? list : [];
}

/**
 * A folder as a message names it: the game folder for the top of the game, else its path.
 * @param folder - the folder, relative to the game folder.
 */
function folderLabel(folder: string): string {
  return folder === "." ? "the game folder" : folder;
}

/**
 * Checks that every sound file of an audio manifest exists. The files are relative to the audio folder, which is the
 * folder of the manifest: the game serves that folder as `audio/`, and the loader fetches each file from it.
 * @param files - the game's file index.
 * @param folder - the game folder.
 * @param doc - the manifest's document. Its JSON gives the paths of the sounds as the file writes them.
 * @param manifest - the manifest, as the audio reader gives it back.
 */
export async function checkAudioFiles(
  files: FileIndex,
  folder: string,
  doc: GameDocument,
  manifest: AudioManifest,
): Promise<Issue[]> {
  const issues: Issue[] = [];
  const audioFolder = resolve(folder, doc.folder);
  const rawSounds = listField(doc.json, "sounds");
  for (const [soundIndex, sound] of manifest.sounds.entries()) {
    const usesVariants = listField(rawSounds[soundIndex], "variants").length > 0;
    for (const [variantIndex, variant] of sound.variants.entries()) {
      for (const [fileIndex, file] of variant.files.entries()) {
        const path = usesVariants
          ? `sounds[${soundIndex}].variants[${variantIndex}].files[${fileIndex}]`
          : `sounds[${soundIndex}].files[${fileIndex}]`;
        if (!(await files.isFile(join(audioFolder, file)))) {
          const message = `"${file}" is not in the audio folder, ${folderLabel(doc.folder)}`;
          issues.push(makeIssue("error", doc.file, path, "missing-file", message));
        }
      }
    }
  }
  return issues;
}

/**
 * Reads a tileset file of the game as a JSON value, for the loader of a map. A file that is not in the game, in its
 * exact spelling, is an error: the loader reports it against the map.
 * @param files - the game's file index.
 * @param folder - the game folder.
 * @param file - the file, relative to the game folder, with forward slashes.
 */
async function readTilesetFile(files: FileIndex, folder: string, file: string): Promise<object> {
  const path = join(folder, file);
  if (!(await files.isFile(path))) throw new Error(`the file "${file}" is not in the game`);
  const text = (await readFile(path, "utf8")).replace(/^\uFEFF/u, "");
  return JSON.parse(text) as object;
}

/**
 * The path of a problem with the `[name]` prefix that the reader gives to the problems of a tileset, without it.
 * @param path - the path as the reader gives it.
 * @param name - the tileset's file name.
 */
function withoutPrefix(path: string, name: string): string {
  const prefix = `[${name}]`;
  if (!path.startsWith(prefix)) return path;
  const rest = path.slice(prefix.length).replace(/^\./u, "");
  return rest === "" ? WHOLE_FILE : rest;
}

/**
 * The issue of a tileset's picture that is not in the game, if it is not. The picture is relative to the tileset's own
 * file: the .tsj file of an external tileset, or the map for a tileset embedded in it.
 * @param files - the game's file index.
 * @param folder - the game folder.
 * @param doc - the map or the tileset file that names the tileset.
 * @param tileset - the tileset, as the reader gives it back.
 * @param reported - the pictures already reported, by file and path, so that a shared tileset is reported once.
 */
async function pictureIssues(
  files: FileIndex,
  folder: string,
  doc: GameDocument,
  tileset: Tileset,
  reported: Set<string>,
): Promise<Issue[]> {
  const sourceFile = tileset.source === undefined ? undefined : posix.normalize(posix.join(doc.folder, tileset.source));
  const ownerFolder = sourceFile === undefined ? doc.folder : posix.dirname(sourceFile);
  const picture = posix.normalize(posix.join(ownerFolder, tileset.image));
  const inGame = picture !== ".." && !picture.startsWith("../");
  if (inGame && (await files.isFile(join(folder, picture)))) return [];

  const file = sourceFile ?? doc.file;
  const path = sourceFile === undefined ? `tilesets["${tileset.name}"].image` : "image";
  const key = `${file} ${path}`;
  if (reported.has(key)) return [];
  reported.add(key);
  const message = `the picture "${tileset.image}" is not in the game: it is looked for at "${picture}"`;
  return [makeIssue("error", file, path, "missing-file", message)];
}

/**
 * Checks a tileset file that no map names. It is loaded through a one-cell map that names it as its external tileset,
 * so that the same reader and the same rules apply. Its own problems are reported at its own file.
 * @param files - the game's file index.
 * @param folder - the game folder.
 * @param doc - the tileset's document.
 * @param reported - the pictures already reported, by file and path.
 */
async function checkOwnTileset(
  files: FileIndex,
  folder: string,
  doc: GameDocument,
  reported: Set<string>,
): Promise<Issue[]> {
  let parsed: unknown;
  try {
    parsed = JSON.parse(doc.text);
  } catch {
    return [makeIssue("error", doc.file, WHOLE_FILE, "json", "the tileset text is not JSON")];
  }
  const wrapper = {
    type: "map",
    orientation: "orthogonal",
    infinite: false,
    width: 1,
    height: 1,
    tilewidth: 1,
    tileheight: 1,
    layers: [],
    tilesets: [{ firstgid: 1, source: doc.name }],
  };
  let map: TileMap;
  try {
    map = await loadTileMap(wrapper, { loadTileset: async () => parsed as object });
  } catch (error) {
    return failureIssues(doc.file, error).map((issue) => ({ ...issue, path: withoutPrefix(issue.path, doc.name) }));
  }
  const tileset = map.tilesets[0];
  return tileset === undefined ? [] : pictureIssues(files, folder, doc, tileset, reported);
}

/**
 * Checks the Tiled files of a game. Each map is loaded with its tilesets, as the game loads it: an external tileset
 * is read from the file that the map names, relative to the map's folder. Every problem the reader finds is reported
 * with its path, and every tileset's picture must exist. A tileset file that no map names is checked on its own.
 * @param files - the game's file index.
 * @param folder - the game folder.
 * @param documents - the tile documents of the game: its maps and its tileset files.
 */
export async function checkTiledFiles(
  files: FileIndex,
  folder: string,
  documents: readonly GameDocument[],
): Promise<Issue[]> {
  const issues: Issue[] = [];
  const named = new Set<string>();
  const reported = new Set<string>();
  for (const doc of documents) {
    if (doc.kind !== "tile-map") continue;
    let map: TileMap;
    try {
      map = await loadTileMap(doc.text, {
        loadTileset: (source) => {
          const file = posix.normalize(posix.join(doc.folder, source));
          named.add(file);
          return readTilesetFile(files, folder, file);
        },
      });
    } catch (error) {
      issues.push(...failureIssues(doc.file, error));
      continue;
    }
    for (const tileset of map.tilesets) {
      issues.push(...(await pictureIssues(files, folder, doc, tileset, reported)));
    }
  }
  for (const doc of documents) {
    if (doc.kind !== "tileset" || named.has(posix.normalize(doc.file))) continue;
    issues.push(...(await checkOwnTileset(files, folder, doc, reported)));
  }
  return issues;
}
