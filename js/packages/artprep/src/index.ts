// @velvet/artprep: tooling that prepares images for a diffusion pipeline, from data
// (docs/formats/ART_PREP.md). It composes prompts in three layers (style, identity and variation) for
// each generation profile, writes the jobs of an external batch runner, and writes the selection
// gallery. It runs no model and writes no art file.
//
// Import from the package root, for example:
//   import { parseProfiles, parseIdentity, buildJobs, galleryHtml } from "@velvet/artprep";

export {
  ARTPREP_FORMAT,
  ARTPREP_KINDS,
  ARTPREP_VERSION,
  DEFAULT_SEEDS,
  MAX_SEEDS,
  PROMPT_LAYERS,
  VARIATION_FIELDS,
} from "./types";
export type {
  ArtPrepJob,
  ArtPrepKind,
  ArtPrepMode,
  ArtPrepProfile,
  ArtRef,
  CharacterIdentity,
  ComposedPrompt,
  GalleryCandidate,
  GalleryOptions,
  GenerationSettings,
  JobOutput,
  LoraSetting,
  Phrase,
  PromptLayer,
  PromptRules,
  PromptStyle,
  ResultsDocument,
  Shot,
  ShotEdit,
  ShotResult,
  ShotVariation,
  SizeRule,
  StyleLayer,
  VariationField,
  WeightSyntax,
} from "./types";
export { ArtPrepError } from "./error";
export type { ArtPrepErrorCode, ArtPrepIssue } from "./error";
export { parseProfiles } from "./profiles";
export { parseIdentity, parseStyle } from "./layers";
export { parseShots } from "./shots";
export { parseResults } from "./results";
export { composePrompt } from "./compose";
export { buildJobs, jobSize, outputName } from "./jobs";
export { galleryHtml } from "./gallery";
