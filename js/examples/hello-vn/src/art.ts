// The art of the game, by art id: the backgrounds, the title and the portraits. The manifest is the one in
// public/art. The game ships with no image files, so every id shows its generated placeholder, which is the
// intended look of provisional art; a file added to art/ later replaces it without a change here.

import { ArtResolver, artElement, parseArtManifest, type ArtManifest } from "@velvet/assets";
import { ART } from "./config";
import { logLine } from "./platform";

/** The art of the game. */
export interface Art {
  /** The parsed manifest, for the checks of the content test. */
  readonly manifest: ArtManifest;
  /** The picture of an id: its file, or its placeholder when it has no file yet. Sized in logical pixels. */
  picture(id: string): HTMLImageElement | SVGElement;
  /** The portrait of a speaker in a mood, or null when the speaker has no portrait in the manifest. */
  portrait(who: string, mood: string | null): HTMLImageElement | SVGElement | null;
  /** True when the manifest declares the id. */
  has(id: string): boolean;
}

/**
 * Reads the art manifest and makes the art of the game.
 * @param manifestText - the text of art/manifest.json.
 * @param files - the art files that exist, as paths relative to the art folder (none yet).
 * @throws ArtError when the manifest is not valid.
 */
export function createArt(manifestText: string, files: readonly string[] = []): Art {
  const manifest = parseArtManifest(manifestText);
  const resolver = new ArtResolver(manifest, files, {
    baseUrl: "art/",
    onUnknown: (error) => logLine("warn", `art: ${error.message}`),
  });
  const entries = new Map(manifest.entries.map((entry) => [entry.id, entry]));
  return {
    manifest,
    picture: (id) => artElement(resolver, id),
    portrait(who, mood) {
      const id = `${ART.portraitPrefix}${who}`;
      const entry = entries.get(id);
      if (entry === undefined) return null;
      // A mood the entry declares shows its own picture; any other mood shows the base portrait.
      const variant = mood !== null && entry.variants.includes(mood) ? mood : undefined;
      return artElement(resolver, id, variant);
    },
    has: (id) => entries.has(id),
  };
}
