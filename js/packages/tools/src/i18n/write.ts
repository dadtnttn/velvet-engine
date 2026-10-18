// Writes a pseudo-locale into a game's locale folder. It refuses to write over a language that is not a
// pseudo-locale, so a real translation is never replaced.

import { mkdir, readdir, readFile, writeFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import type { EntryValue } from "@velvet/i18n";
import { pseudoText, pseudoValue, PSEUDO_NOTE } from "./pseudo";
import type { LoadedLocale } from "../game/locales";

/** The file name of the pseudo-locale's catalog. */
export const PSEUDO_FILE = "00_pseudo.json";

/** What a pseudo-locale write produced. */
export interface PseudoWrite {
  /** The new language's tag. */
  readonly tag: string;
  /** The folder that was written, relative to the game folder. */
  readonly folder: string;
  /** How many texts the file holds. */
  readonly keys: number;
  /** The keys whose text is not a valid template, which were left out. */
  readonly skipped: readonly string[];
}

/** Why a pseudo-locale cannot be written. */
export class PseudoError extends Error {
  /** @param message - what is wrong, for the user. */
  constructor(message: string) {
    super(message);
    this.name = "PseudoError";
  }
}

/**
 * Writes `<locales folder>/<tag>/00_pseudo.json` with a pseudo-text of every text of the source language,
 * and of the text keys that the code shows. The folder of the source language's parent is the locales folder.
 * @param gameFolder - the game folder.
 * @param source - the source language, as loaded.
 * @param tag - the pseudo-locale's tag, which the caller has checked.
 * @param textKeys - the text keys that the code shows, which the source does not hold.
 * @throws PseudoError when the folder holds a language that is not a pseudo-locale, or when a text is not valid.
 */
export async function writePseudoLocale(
  gameFolder: string,
  source: LoadedLocale,
  tag: string,
  textKeys: readonly string[],
): Promise<PseudoWrite> {
  const folder = join(dirname(source.folder), tag).split("\\").join("/");
  const target = join(gameFolder, dirname(source.folder), tag);
  await refuseRealLocale(target, folder);

  const entries: Record<string, EntryValue> = {};
  const skipped: string[] = [];
  for (const key of source.locale.keys()) {
    const value = source.locale.values.get(key);
    if (value === undefined) continue;
    const pseudo = pseudoValue(value);
    if (pseudo === undefined) skipped.push(key);
    else entries[key] = pseudo;
  }
  for (const key of textKeys) {
    if (source.locale.has(key)) continue;
    const pseudo = pseudoText(key);
    if (pseudo === undefined) skipped.push(key);
    else entries[key] = pseudo;
  }

  const meta: { name: string; script?: string; direction: "ltr" } = { name: `Pseudo (${tag})`, direction: "ltr" };
  if (source.locale.meta.script !== undefined) meta.script = source.locale.meta.script;
  const catalog = {
    format: "velvet-locale",
    version: 1,
    language: tag,
    note: PSEUDO_NOTE,
    meta,
    entries,
  };
  await mkdir(target, { recursive: true });
  await writeFile(join(target, PSEUDO_FILE), `${JSON.stringify(catalog, null, 2)}\n`, "utf8");
  return { tag, folder, keys: Object.keys(entries).length, skipped };
}

/**
 * Refuses to write into a folder that holds anything other than an earlier pseudo-locale of its own.
 * @param target - the folder, absolute.
 * @param shown - the folder as the user sees it.
 */
async function refuseRealLocale(target: string, shown: string): Promise<void> {
  let names: string[];
  try {
    names = await readdir(target);
  } catch {
    return;
  }
  for (const name of names) {
    if (name !== PSEUDO_FILE) {
      throw new PseudoError(`${shown} already holds ${name}, which is not a pseudo-locale: choose another tag`);
    }
    const text = await readFile(join(target, name), "utf8");
    let note: unknown;
    try {
      note = Reflect.get(JSON.parse(text) as object, "note");
    } catch {
      note = undefined;
    }
    if (note !== PSEUDO_NOTE) {
      throw new PseudoError(`${shown}/${name} is not a pseudo-locale written by velvet-js: choose another tag`);
    }
  }
}
