// @velvet/assets: the art of Velvet JS games, in the velvet-art format (docs/formats/ART_MANIFEST.md).
// A game asks for an art id, never a file path. Files are found through the manifest, and an id
// without a file shows a generated placeholder until its art arrives.
//
// Import from the package root, for example:
//   import { ArtResolver, parseArtManifest, artElement } from "@velvet/assets";

export {
  ART_FORMAT,
  ART_FORMATS,
  ART_KINDS,
  ART_MAX_SIDE,
  ART_REPORT_FORMAT,
  ART_VERSION,
  isArtKind,
} from "./types";
export type {
  ArtAnchor,
  ArtEntry,
  ArtFormat,
  ArtKind,
  ArtManifest,
  ArtRequest,
  ArtSize,
  ArtSource,
} from "./types";
export { ArtError } from "./error";
export type { ArtErrorCode, ArtIssue } from "./error";
export { parseArtManifest } from "./manifest";
export { placeholderColor, placeholderSvg, shortLabel } from "./placeholder";
export { ArtResolver } from "./resolver";
export type { ArtMissingReport, ArtMissingRow, ArtResolverOptions, ResolvedArt } from "./resolver";
export type { PreloadProgress, PreloadResult } from "./preload";
export { artElement } from "./dom";
