// The shared vocabulary of velvet-artprep: the format tag and version, the kinds of document, the
// fields of a shot's variation, and the shapes of profiles, layers, shots, jobs and results. The rules
// themselves are in docs/formats/ART_PREP.md.

/** The format tag that every artprep document carries. */
export const ARTPREP_FORMAT = "velvet-artprep";

/** The format version this package reads. Any other version is refused. */
export const ARTPREP_VERSION = 1 as const;

/** The kinds of document. Each file holds one of them. */
export const ARTPREP_KINDS = ["profiles", "style", "identity", "shots", "job", "results"] as const;

/** One kind of document. */
export type ArtPrepKind = (typeof ARTPREP_KINDS)[number];

/**
 * The fields of a shot's variation, in the order a prompt writes them. `subject` is the first
 * sentence of the picture; a shot that leaves it out takes the `meta.prompt` of its art entry.
 */
export const VARIATION_FIELDS = [
  "subject",
  "framing",
  "pose",
  "expression",
  "outfit",
  "setting",
  "light",
  "palette",
] as const;

/** One field of the variation layer. */
export type VariationField = (typeof VARIATION_FIELDS)[number];

/** The three layers of a prompt. A profile lists them in the order it writes them. */
export const PROMPT_LAYERS = ["style", "identity", "variation"] as const;

/** One layer of a prompt. */
export type PromptLayer = (typeof PROMPT_LAYERS)[number];

/** The candidate seeds of a shot when the caller gives none: six per shot, 101 to 106. */
export const DEFAULT_SEEDS: readonly number[] = [101, 102, 103, 104, 105, 106];

/** The most seeds one shot may list. */
export const MAX_SEEDS = 64;

/** One phrase of a layer, with its weight. The weight is 1 unless the author sets another. */
export interface Phrase {
  /** The text: trimmed, on one line. */
  readonly text: string;
  /** The emphasis: above 0 and at most 2. Written only by tag profiles that use the paren syntax. */
  readonly weight: number;
}

/** `generate` makes new pictures; `edit` changes a picture that already exists. */
export type ArtPrepMode = "generate" | "edit";

/** `prose` writes sentences; `tags` writes a list of tags. */
export type PromptStyle = "prose" | "tags";

/** `paren` writes a weight as `(text:1.2)`; `none` writes no weights. */
export type WeightSyntax = "paren" | "none";

/** One LoRA of a profile: its name and its strength. */
export interface LoraSetting {
  readonly name: string;
  readonly weight: number;
}

/** The sampling settings of a profile. Every job copies them, so a runner needs no other file. */
export interface GenerationSettings {
  readonly model: string;
  readonly loras: readonly LoraSetting[];
  readonly sampler: string;
  /** The scheduler, or null when the profile leaves it to the runner. */
  readonly scheduler: string | null;
  readonly steps: number;
  readonly cfg: number;
  /** The shift of a flow model, or null when the profile leaves it to the runner. */
  readonly shift: number | null;
}

/** How a profile sizes its jobs from an art box: the long side in pixels, and the rounding step. */
export interface SizeRule {
  readonly longSide: number;
  readonly multipleOf: number;
}

/** How a profile writes a prompt: its grammar, the order of its layers and its joiners. */
export interface PromptRules {
  readonly style: PromptStyle;
  readonly order: readonly PromptLayer[];
  /** Tags that lead the tag list. Only tag profiles have them. */
  readonly qualityTags: readonly string[];
  readonly weights: WeightSyntax;
  /** The text between two phrases of one layer. */
  readonly phraseJoiner: string;
  /** The text between two layers. */
  readonly layerJoiner: string;
  /** The text that closes each layer. */
  readonly layerEnd: string;
}

/** One generation profile: a named setup for one model family. */
export interface ArtPrepProfile {
  readonly id: string;
  readonly label: string;
  readonly mode: ArtPrepMode;
  readonly generation: GenerationSettings;
  readonly size: SizeRule;
  /** The most input images a job may use. In an edit, the picture being edited counts as one. */
  readonly maxReferences: number;
  /** The most characters that one picture may show: 1 or 2. */
  readonly maxIdentities: number;
  /** True when the profile takes a negative prompt. */
  readonly negative: boolean;
  /** Keywords that the profile's LoRAs need. They lead the prompt. */
  readonly triggerWords: readonly string[];
  readonly prompt: PromptRules;
  /** Extra text for an edited field, keyed by the field. Only edit profiles have them. */
  readonly editGuards: Readonly<Partial<Record<VariationField, string>>>;
}

/** The style layer of a project: the phrases that every picture carries, and its negative phrases. */
export interface StyleLayer {
  readonly phrases: readonly Phrase[];
  readonly negative: readonly string[];
}

/** The identity layer of one character: what stays the same in every picture of that character. */
export interface CharacterIdentity {
  /** The character's id, the same as its id in the character sheet. */
  readonly id: string;
  /** The name that the prompt uses. */
  readonly name: string;
  readonly species: string | null;
  readonly body: string | null;
  readonly skin: string | null;
  readonly hair: string | null;
  readonly eyes: string | null;
  /** The character's usual clothes. */
  readonly outfit: string | null;
  /** Features that the character has, such as horns. */
  readonly has: readonly Phrase[];
  /** Features that the character does not have. Each one is written as `no <feature>` or as a negative. */
  readonly lacks: readonly string[];
  /** Phrases that must appear in every picture of the character, in this order. */
  readonly fixed: readonly Phrase[];
}

/** The variation layer of a shot: the fields that change from one picture to the next. */
export type ShotVariation = Readonly<Partial<Record<VariationField, Phrase>>>;

/** A reference to an art id, optionally to one of its variants. */
export interface ArtRef {
  readonly id: string;
  readonly variant: string | null;
}

/** The edit of an edit shot: the picture it starts from, and the one field that it changes. */
export interface ShotEdit {
  readonly base: ArtRef;
  readonly change: VariationField;
}

/** One shot: a picture that an art id (and variant) needs, described by its layers. */
export interface Shot {
  readonly id: string;
  readonly art: string;
  readonly variant: string | null;
  /** The identities that the picture shows, in the order of the prompt. */
  readonly characters: readonly string[];
  /** The input images of a generation, by art id. */
  readonly references: readonly ArtRef[];
  readonly variation: ShotVariation;
  /** Set for an edit shot, and null for a shot that makes a new picture. */
  readonly edit: ShotEdit | null;
}

/** One output of a job: its seed, and the name of its file without the extension. */
export interface JobOutput {
  readonly seed: number;
  readonly name: string;
}

/** One job: everything an external batch runner needs to make the candidates of one shot. */
export interface ArtPrepJob {
  readonly format: typeof ARTPREP_FORMAT;
  readonly version: typeof ARTPREP_VERSION;
  readonly kind: "job";
  readonly shot: string;
  readonly profile: string;
  readonly art: string;
  readonly variant: string | null;
  readonly characters: readonly string[];
  readonly prompt: string;
  /** The negative prompt, or null when the profile takes none. */
  readonly negative: string | null;
  readonly references: readonly ArtRef[];
  readonly edit: ShotEdit | null;
  /** The size in pixels: width, then height. */
  readonly size: readonly [number, number];
  readonly generation: GenerationSettings;
  readonly seeds: readonly number[];
  readonly outputs: readonly JobOutput[];
}

/** The prompt of one picture: the positive text, and the negative text when the profile uses one. */
export interface ComposedPrompt {
  readonly prompt: string;
  readonly negative: string | null;
}

/** One candidate picture of a shot, as the runner reports it. */
export interface GalleryCandidate {
  readonly seed: number;
  /** The code of the candidate: its output name, without the extension. */
  readonly name: string;
  /** Where the picture is, relative to the gallery page, or an https or data:image URL. */
  readonly image: string;
}

/** The results of one shot: its candidates, and the prompt that made them. */
export interface ShotResult {
  readonly shot: string;
  readonly art: string;
  readonly variant: string | null;
  readonly prompt: string | null;
  readonly candidates: readonly GalleryCandidate[];
}

/** A results document: a title, and the results of its shots. */
export interface ResultsDocument {
  readonly title: string | null;
  readonly shots: readonly ShotResult[];
}

/** The options of the gallery page. */
export interface GalleryOptions {
  /** The page title. The default is "Art gallery". */
  readonly title?: string;
}
