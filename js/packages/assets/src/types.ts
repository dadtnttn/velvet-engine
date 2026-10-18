// The shared vocabulary of velvet-art: the format tags and version, the kinds and file formats of
// art, the shapes of a manifest and the requests that a game makes. The rules themselves are in
// docs/formats/ART_MANIFEST.md.

/** The format tag that every art manifest carries. */
export const ART_FORMAT = "velvet-art";

/** The format version this package reads. Any other version is refused. */
export const ART_VERSION = 1 as const;

/** The format tag of the missing-art report. */
export const ART_REPORT_FORMAT = "velvet-art-report";

/** The largest width or height an art box may have, in logical pixels (an 8K width). */
export const ART_MAX_SIDE = 7680;

/**
 * The kinds of art, in the order the format lists them. A kind decides the folder that holds the
 * files and the silhouette of the placeholder.
 */
export const ART_KINDS = ["portrait", "fullbody", "card", "icon", "background", "prop"] as const;

/** One kind of art. */
export type ArtKind = (typeof ART_KINDS)[number];

/**
 * The file formats of art, in resolution order: the first one that exists wins. The order is
 * AVIF, WebP, PNG, then a hand-made SVG.
 */
export const ART_FORMATS = ["avif", "webp", "png", "svg"] as const;

/** One file format of art. */
export type ArtFormat = (typeof ART_FORMATS)[number];

/** Where a resolved picture comes from: a file in one of the formats, or a generated placeholder. */
export type ArtSource = ArtFormat | "placeholder";

/** A box's width and height, in logical pixels of the 1920×1080 stage. */
export type ArtSize = readonly [width: number, height: number];

/** A pivot inside the box, as fractions of its size from the top-left corner. `[0.5, 1]` is the bottom middle. */
export type ArtAnchor = readonly [x: number, y: number];

/** One art id as the parsed manifest holds it. Optional fields are filled with their defaults. */
export interface ArtEntry {
  /** The id, unique in the manifest, such as `character.lyra.portrait`. */
  readonly id: string;
  /** The kind: the folder of the files and the shape of the placeholder. */
  readonly kind: ArtKind;
  /** The box the art is drawn in, in logical pixels. */
  readonly size: ArtSize;
  /** The pivot inside the box. Defaults to the top-left corner. */
  readonly anchor: ArtAnchor;
  /** The variant names, unique within the entry, such as `happy`. Defaults to none. */
  readonly variants: readonly string[];
  /** Notes for later art generation. The runtime never reads them. Defaults to an empty object. */
  readonly meta: Readonly<Record<string, unknown>>;
}

/** A parsed art manifest in its normal form: defaults filled in, entries in the order of the file. */
export interface ArtManifest {
  readonly format: typeof ART_FORMAT;
  readonly version: typeof ART_VERSION;
  readonly entries: readonly ArtEntry[];
}

/** A request for one picture: an id, and optionally one of its variants. */
export interface ArtRequest {
  /** The art id. */
  readonly id: string;
  /** A variant that the id declares. Left out, the request is for the base picture. */
  readonly variant?: string;
}

/** True when `value` is one of the kinds in ART_KINDS. */
export function isArtKind(value: string): value is ArtKind {
  return ART_KINDS.some((kind) => kind === value);
}
