// The resolver: maps art ids and variants to the files that show them, and falls back to a
// placeholder when no file exists. It is built from a manifest and a list of the files that exist,
// so one code path serves a file list from the desktop bridge, a fetched index or a Vite glob.

import { ArtError } from "./error";
import { entryProblems } from "./manifest";
import { placeholderSvg } from "./placeholder";
import { preloadArt, type PreloadProgress, type PreloadResult } from "./preload";
import {
  ART_FORMATS,
  ART_REPORT_FORMAT,
  ART_VERSION,
  type ArtAnchor,
  type ArtEntry,
  type ArtFormat,
  type ArtKind,
  type ArtManifest,
  type ArtRequest,
  type ArtSize,
  type ArtSource,
} from "./types";

/** Options of an ArtResolver. Both are optional. */
export interface ArtResolverOptions {
  /** The address that the art folder is served from, such as `art/` or `velvet://game/art/`. Defaults to `art/`. */
  readonly baseUrl?: string;
  /** The file formats the reader can show. Defaults to all four. The order is always AVIF, WebP, PNG, SVG. */
  readonly accept?: readonly ArtFormat[];
  /**
   * True: an id or a variant that the manifest does not declare throws an ArtError (for tools and
   * tests). False, the default: it resolves to a placeholder and onUnknown is told once, so a typo
   * shows on screen instead of breaking the frame that draws it.
   */
  readonly strict?: boolean;
  /** Told once per unknown id or variant when strict is false. Defaults to console.warn. */
  readonly onUnknown?: (error: ArtError) => void;
}

/** The box of a placeholder for an id that the manifest does not declare. */
const UNKNOWN_SIZE: ArtSize = [128, 128];

/** The picture that a request resolved to. */
export interface ResolvedArt {
  /** The art id. */
  readonly id: string;
  /** The variant whose own file answered. Null when the base id's file or a placeholder answered. */
  readonly variant: string | null;
  /** The kind of the id. */
  readonly kind: ArtKind;
  /** The box in logical pixels. */
  readonly size: ArtSize;
  /** The pivot inside the box, as fractions of the size. */
  readonly anchor: ArtAnchor;
  /** The file format that answered, or "placeholder". */
  readonly source: ArtSource;
  /** Where to load the picture from: the file's URL, or a data URL for a placeholder. */
  readonly url: string;
  /** The file's path inside the art folder, such as `portrait/character.lyra.portrait.webp`. Null for a placeholder. */
  readonly path: string | null;
}

/** One id in the missing-art report. */
export interface ArtMissingRow {
  /** The art id. */
  readonly id: string;
  /** The kind of the id. */
  readonly kind: ArtKind;
  /** The box in logical pixels. */
  readonly size: ArtSize;
  /** True when the base id has a file. False when the id shows a placeholder. */
  readonly base: boolean;
  /** The declared variants that have no file of their own, in the order of the manifest. */
  readonly missing_variants: readonly string[];
}

/** The missing-art report, a JSON document. It lists every id that still needs art. */
export interface ArtMissingReport {
  readonly format: typeof ART_REPORT_FORMAT;
  readonly version: typeof ART_VERSION;
  /** The ids that have no base file, or a variant with no file of its own. Sorted by id. */
  readonly missing: readonly ArtMissingRow[];
}

/**
 * Reads a file path the way the resolver compares paths: `/` separators, and no leading `./` or `/`.
 * @param path - a path from the file list.
 */
function normalizePath(path: string): string {
  return path.replace(/\\/g, "/").replace(/^(\.\/|\/)+/, "");
}

/** Adds a trailing slash to a base URL, unless it is empty or already ends with one. */
function withSlash(url: string): string {
  return url === "" || url.endsWith("/") ? url : `${url}/`;
}

/** Orders two ids. Ids are ASCII, so code unit order is the byte order of the format. */
function byId(a: ArtEntry, b: ArtEntry): number {
  if (a.id === b.id) return 0;
  return a.id < b.id ? -1 : 1;
}

/**
 * Finds the picture of each art id. The resolver is read-only apart from markBroken, and it keeps the
 * results it has found, so calling it every frame is cheap.
 */
export class ArtResolver {
  private readonly entries: ReadonlyMap<string, ArtEntry>;
  private readonly files: ReadonlySet<string>;
  private readonly formats: readonly ArtFormat[];
  private readonly baseUrl: string;
  private readonly broken = new Set<string>();
  private readonly cache = new Map<string, ResolvedArt>();
  private readonly strict: boolean;
  private readonly onUnknown: (error: ArtError) => void;
  private readonly reported = new Set<string>();

  /**
   * @param manifest - a manifest from parseArtManifest, or one built by hand.
   * @param files - the paths of the files that exist, relative to the art folder, such as
   *   `portrait/character.lyra.portrait.webp`. A `\` is read as `/`, and a leading `./` or `/` is ignored.
   * @param options - the base URL of the art folder and the formats the reader can show.
   * @throws ArtError with code "duplicate" when the manifest repeats an id or a file name.
   */
  constructor(manifest: ArtManifest, files: Iterable<string>, options: ArtResolverOptions = {}) {
    const problems = entryProblems(manifest.entries);
    const [first] = problems;
    if (first !== undefined) {
      throw new ArtError("duplicate", `art manifest repeats a name: ${first.message}`, problems);
    }
    this.entries = new Map<string, ArtEntry>(manifest.entries.map((entry): [string, ArtEntry] => [entry.id, entry]));
    this.files = new Set(Array.from(files, normalizePath));
    const accepted = new Set<ArtFormat>(options.accept ?? ART_FORMATS);
    this.formats = ART_FORMATS.filter((format) => accepted.has(format));
    this.baseUrl = withSlash(options.baseUrl ?? "art/");
    this.strict = options.strict ?? false;
    this.onUnknown = options.onUnknown ?? ((error) => console.warn(`@velvet/assets: ${error.message}`));
  }

  /**
   * The picture of an art id, or of one of its variants.
   *
   * The lookup tries, in this order: the variant's own file in AVIF, WebP, PNG and SVG; the base
   * id's file in the same formats; a placeholder. The first file found wins. A variant therefore
   * falls back to the base id, and never the other way round.
   * @param id - an id of the manifest.
   * @param variant - a variant that the id declares. Left out, the base picture is wanted.
   * @throws ArtError with code "unknown-id" or "unknown-variant" when the manifest does not declare
   *   them and the resolver is strict. Otherwise an unknown id gets a placeholder and an unknown
   *   variant falls back to the base picture, and onUnknown is told once.
   */
  resolve(id: string, variant?: string): ResolvedArt {
    const known = this.entries.get(id);
    if (known === undefined && !this.strict) {
      this.report(new ArtError("unknown-id", `art id "${id}" is not in the manifest`));
      const stand: ArtEntry = { id, kind: "prop", size: UNKNOWN_SIZE, anchor: [0, 0], variants: [], meta: {} };
      return this.placeholder(stand);
    }
    const entry = this.entryFor(id);
    if (variant !== undefined && !entry.variants.includes(variant)) {
      const error = new ArtError("unknown-variant", `art id "${id}" has no variant "${variant}"`);
      if (this.strict) throw error;
      this.report(error);
      return this.resolve(id);
    }
    const key = `${id}#${variant ?? ""}`;
    const cached = this.cache.get(key);
    if (cached !== undefined) return cached;
    const found = this.findFile(entry, variant) ?? this.placeholder(entry);
    this.cache.set(key, found);
    return found;
  }

  /**
   * Tells the resolver that a file cannot be shown, for example because the browser cannot decode it.
   * Later resolutions skip that file and go on to the next format, or to a placeholder. The report
   * counts it as missing.
   * @param path - the file's path inside the art folder, as ResolvedArt.path gives it.
   */
  markBroken(path: string): void {
    this.broken.add(normalizePath(path));
    this.cache.clear();
  }

  /**
   * Decodes the pictures of some ids before they are drawn, with a progress callback. See
   * preloadArt in preload.ts for the details. Placeholders need no decoding. Outside a browser,
   * where there is no Image, nothing is decoded and every request counts as done at once.
   * @param requests - art ids, or requests with a variant.
   * @param onProgress - called after each request is done.
   * @returns the paths of the files that could not be decoded.
   * @throws ArtError (asynchronously) with code "unknown-id" or "unknown-variant" for a bad request.
   */
  preload(
    requests: readonly (string | ArtRequest)[],
    onProgress?: (progress: PreloadProgress) => void,
  ): Promise<PreloadResult> {
    return preloadArt(this, requests, onProgress);
  }

  /**
   * The missing-art report: every id whose base has no file, and every id with a variant that has no
   * file of its own. A file counts when its format is accepted and no markBroken call names it.
   */
  missingReport(): ArtMissingReport {
    const missing: ArtMissingRow[] = [];
    const sorted = [...this.entries.values()].sort(byId);
    for (const entry of sorted) {
      const base = this.hasOwnFile(entry, entry.id);
      const missingVariants = entry.variants.filter((variant) => !this.hasOwnFile(entry, `${entry.id}.${variant}`));
      if (!base || missingVariants.length > 0) {
        missing.push({
          id: entry.id,
          kind: entry.kind,
          size: [entry.size[0], entry.size[1]],
          base,
          missing_variants: missingVariants,
        });
      }
    }
    return { format: ART_REPORT_FORMAT, version: ART_VERSION, missing };
  }

  /** Tells onUnknown about a bad request, once per message. */
  private report(error: ArtError): void {
    if (this.reported.has(error.message)) return;
    this.reported.add(error.message);
    this.onUnknown(error);
  }

  /** The entry of an id. Throws when the manifest does not have the id. */
  private entryFor(id: string): ArtEntry {
    const entry = this.entries.get(id);
    if (entry === undefined) {
      throw new ArtError("unknown-id", `art id "${id}" is not in the manifest`);
    }
    return entry;
  }

  /** True when the file exists, its format is accepted and no markBroken call names it. */
  private hasFile(path: string): boolean {
    return this.files.has(path) && !this.broken.has(path);
  }

  /** True when the name (an id, or an id with a variant) has a file in any accepted format. */
  private hasOwnFile(entry: ArtEntry, name: string): boolean {
    return this.formats.some((format) => this.hasFile(`${entry.kind}/${name}.${format}`));
  }

  /** The first existing file of the variant, then of the base id, in format order. */
  private findFile(entry: ArtEntry, variant: string | undefined): ResolvedArt | undefined {
    const candidates: { readonly name: string; readonly variant: string | null }[] = [];
    if (variant !== undefined) candidates.push({ name: `${entry.id}.${variant}`, variant });
    candidates.push({ name: entry.id, variant: null });
    for (const candidate of candidates) {
      for (const format of this.formats) {
        const path = `${entry.kind}/${candidate.name}.${format}`;
        if (this.hasFile(path)) {
          return this.answer(entry, candidate.variant, format, `${this.baseUrl}${path}`, path);
        }
      }
    }
    return undefined;
  }

  /** The placeholder of an id: a data URL of its generated SVG. */
  private placeholder(entry: ArtEntry): ResolvedArt {
    const svg = placeholderSvg(entry.id, entry.kind, entry.size);
    const url = `data:image/svg+xml;charset=utf-8,${encodeURIComponent(svg)}`;
    return this.answer(entry, null, "placeholder", url, null);
  }

  /** Builds a frozen result, with its own copies of the size and the anchor. */
  private answer(
    entry: ArtEntry,
    variant: string | null,
    source: ArtSource,
    url: string,
    path: string | null,
  ): ResolvedArt {
    const size: ArtSize = [entry.size[0], entry.size[1]];
    const anchor: ArtAnchor = [entry.anchor[0], entry.anchor[1]];
    return Object.freeze({ id: entry.id, variant, kind: entry.kind, size, anchor, source, url, path });
  }
}
