// Builds the jobs of a profile: one job per shot, with its composed prompt, its references by art id,
// its seeds and the names of its outputs. A job is data for an external batch runner. Nothing here runs
// a model (docs/formats/ART_PREP.md, "Jobs").

import type { ArtEntry, ArtSize } from "@velvet/assets";
import { composePrompt } from "./compose";
import { addIssue, PHRASE_LIMIT, problemsError, readArray, readText, type Issues } from "./read";
import {
  DEFAULT_SEEDS,
  MAX_SEEDS,
  ARTPREP_FORMAT,
  ARTPREP_VERSION,
  VARIATION_FIELDS,
  type ArtPrepJob,
  type ArtPrepProfile,
  type ArtRef,
  type CharacterIdentity,
  type GenerationSettings,
  type Phrase,
  type Shot,
  type ShotVariation,
  type SizeRule,
  type StyleLayer,
} from "./types";

/** The style of a call that gives none: no phrases and no negatives. */
const NO_STYLE: StyleLayer = { phrases: [], negative: [] };

/** The largest seed: an unsigned 32-bit number. */
const MAX_SEED = 4294967295;

/** The notes that an art entry gives for generation, read from its `meta` block. */
interface EntryNotes {
  /** The subject of the picture, from `meta.prompt`. */
  readonly subject: Phrase | undefined;
  /** Extra style phrases, from `meta.style`. */
  readonly style: readonly Phrase[];
  /** Extra negative phrases, from `meta.negative`, split at commas. */
  readonly negative: readonly string[];
}

/**
 * The name of a picture without its seed: the art id, and the variant after a dot when there is one.
 * @param art - the art id.
 * @param variant - the variant name, or null.
 */
function pictureName(art: string, variant: string | null): string {
  return variant === null ? art : `${art}.${variant}`;
}

/**
 * The name of one output: the picture name, an underscore and the seed, such as
 * `character.ada.portrait.happy_101`. The runner adds the file extension.
 * @param art - the art id.
 * @param variant - the variant name, or null.
 * @param seed - the seed of the output.
 */
export function outputName(art: string, variant: string | null, seed: number): string {
  return `${pictureName(art, variant)}_${seed}`;
}

/**
 * The size of a job in pixels, from the box of the art and the size rule. The long side of the box
 * becomes `longSide` pixels. The short side keeps the box's proportion, rounded down to a multiple of
 * `multipleOf`, and is never less than one multiple.
 * @param size - the box of the art, in logical pixels: width and height.
 * @param rule - the size rule of the profile.
 * @returns the width and the height of the job.
 */
export function jobSize(size: ArtSize, rule: SizeRule): [number, number] {
  const [width, height] = size;
  const long = Math.max(width, height);
  const short = Math.min(width, height);
  const scaled = Math.floor((rule.longSide * short) / long);
  const shortSide = Math.max(rule.multipleOf, Math.floor(scaled / rule.multipleOf) * rule.multipleOf);
  return width >= height ? [rule.longSide, shortSide] : [shortSide, rule.longSide];
}

/**
 * Builds the jobs of one profile from its shots. A shot that breaks a rule of the profile, names an
 * unknown id, or makes a picture that another shot also makes, is refused. Every problem is listed
 * before the call throws.
 *
 * @param manifestEntries - the entries of the art manifest (`parseArtManifest(...).entries`).
 * @param characters - the identity layers of the characters.
 * @param shots - the shots to build, in the order of the output.
 * @param profile - the profile that every job uses.
 * @param seeds - the seeds of each shot. Left out, the six seeds 101 to 106 are used.
 * @param style - the style layer of the project. Left out, the style is empty.
 * @returns one job per shot, in the order of the shots.
 * @throws ArtPrepError with every problem found.
 */
export function buildJobs(
  manifestEntries: readonly ArtEntry[],
  characters: readonly CharacterIdentity[],
  shots: readonly Shot[],
  profile: ArtPrepProfile,
  seeds: readonly number[] = DEFAULT_SEEDS,
  style: StyleLayer = NO_STYLE,
): ArtPrepJob[] {
  const issues: Issues = [];
  checkSeeds(seeds, issues);
  const entries = new Map<string, ArtEntry>(manifestEntries.map((entry) => [entry.id, entry]));
  const identities = new Map<string, CharacterIdentity>();
  characters.forEach((one, index) => {
    if (identities.has(one.id)) {
      addIssue(issues, "duplicate", `character "${one.id}" has two identity layers`, `characters[${index}].id`);
    }
    identities.set(one.id, one);
  });
  const claimed = new Set<string>();
  const jobs: ArtPrepJob[] = [];

  shots.forEach((shot, index) => {
    const path = `shots[${index}]`;
    const before = issues.length;
    const entry = entries.get(shot.art);
    if (entry === undefined) {
      addIssue(issues, "unknown-id", `art id "${shot.art}" is not in the manifest`, `${path}.art`);
    } else if (shot.variant !== null && !entry.variants.includes(shot.variant)) {
      addIssue(issues, "unknown-variant", `"${shot.variant}" is not a variant of ${shot.art}`, `${path}.variant`);
    }

    const picture = JSON.stringify([shot.art, shot.variant]);
    if (claimed.has(picture)) {
      addIssue(issues, "duplicate", `another shot already makes ${pictureName(shot.art, shot.variant)}`, path);
    }
    claimed.add(picture);

    const shown: CharacterIdentity[] = [];
    shot.characters.forEach((id, position) => {
      const one = identities.get(id);
      if (one === undefined) {
        addIssue(issues, "unknown-character", `no identity layer for character "${id}"`, `${path}.characters[${position}]`);
      } else {
        shown.push(one);
      }
    });
    if (shot.characters.length > profile.maxIdentities) {
      addIssue(
        issues,
        "identities",
        `the shot shows ${shot.characters.length} characters; profile "${profile.id}" allows ${profile.maxIdentities}`,
        `${path}.characters`,
      );
    }

    shot.references.forEach((ref, position) => {
      checkRef(ref, entries, `${path}.references[${position}]`, issues);
    });
    if (shot.edit !== null) checkRef(shot.edit.base, entries, `${path}.edit.base`, issues);
    const inputs = shot.references.length + (shot.edit === null ? 0 : 1);
    if (inputs > profile.maxReferences) {
      addIssue(
        issues,
        "references",
        `the shot uses ${inputs} input images; profile "${profile.id}" allows ${profile.maxReferences}`,
        `${path}.references`,
      );
    }

    checkEdit(shot, profile, path, issues);
    if (entry === undefined) return;
    const notes = readNotes(entry, issues);
    if (issues.length > before) return;

    const effective: StyleLayer = {
      phrases: [...style.phrases, ...notes.style],
      negative: [...style.negative, ...notes.negative],
    };
    const variation: ShotVariation = { ...shot.variation, subject: shot.variation.subject ?? notes.subject };
    const composed = composePrompt(effective, shown, variation, profile);
    if (composed.prompt.length === 0) {
      addIssue(issues, "value", "the shot composes an empty prompt", path);
      return;
    }

    jobs.push({
      format: ARTPREP_FORMAT,
      version: ARTPREP_VERSION,
      kind: "job",
      shot: shot.id,
      profile: profile.id,
      art: shot.art,
      variant: shot.variant,
      characters: [...shot.characters],
      prompt: composed.prompt,
      negative: composed.negative,
      references: shot.references.map(copyRef),
      edit: shot.edit === null ? null : { base: copyRef(shot.edit.base), change: shot.edit.change },
      size: jobSize(entry.size, profile.size),
      generation: copyGeneration(profile.generation),
      seeds: [...seeds],
      outputs: seeds.map((seed) => ({ seed, name: outputName(shot.art, shot.variant, seed) })),
    });
  });

  if (issues.length > 0) throw problemsError(issues, "the batch of shots");
  return jobs;
}

/** Checks the seeds of a call: 1 to 64 whole numbers from 0 to 2^32 - 1, each one once. */
function checkSeeds(seeds: readonly number[], issues: Issues): void {
  if (seeds.length === 0 || seeds.length > MAX_SEEDS) {
    addIssue(issues, "value", `seeds must list 1 to ${MAX_SEEDS} seeds`, "seeds");
    return;
  }
  const seen = new Set<number>();
  seeds.forEach((seed, index) => {
    const at = `seeds[${index}]`;
    if (!Number.isInteger(seed) || seed < 0 || seed > MAX_SEED) {
      addIssue(issues, "value", `seed ${seed} must be a whole number from 0 to ${MAX_SEED}`, at);
    }
    if (seen.has(seed)) addIssue(issues, "duplicate", `seed ${seed} is listed twice`, at);
    seen.add(seed);
  });
}

/** Checks that an input image names an art id, and a variant that the id declares. */
function checkRef(ref: ArtRef, entries: ReadonlyMap<string, ArtEntry>, path: string, issues: Issues): void {
  const entry = entries.get(ref.id);
  if (entry === undefined) {
    addIssue(issues, "unknown-id", `reference "${ref.id}" is not in the manifest`, path);
    return;
  }
  if (ref.variant !== null && !entry.variants.includes(ref.variant)) {
    addIssue(issues, "unknown-variant", `"${ref.variant}" is not a variant of ${ref.id}`, path);
  }
}

/**
 * Checks the edit of a shot against its profile. A generate profile takes no edit. An edit profile needs
 * an edit, and the shot's own variation must set exactly the one field that the edit names.
 */
function checkEdit(shot: Shot, profile: ArtPrepProfile, path: string, issues: Issues): void {
  if (profile.mode === "generate") {
    if (shot.edit !== null) {
      const message = `profile "${profile.id}" makes new pictures; the shot must not have an edit`;
      addIssue(issues, "edit", message, `${path}.edit`);
    }
    return;
  }
  if (shot.edit === null) {
    const message = `profile "${profile.id}" edits a picture; the shot needs an edit with a base and a change`;
    addIssue(issues, "edit", message, path);
    return;
  }
  const set = VARIATION_FIELDS.filter((field) => shot.variation[field] !== undefined);
  if (set.length !== 1 || set[0] !== shot.edit.change) {
    const message = `an edit changes one thing: the variation must set only "${shot.edit.change}"`;
    addIssue(issues, "edit", message, `${path}.variation`);
  }
}

/**
 * Reads the generation notes of an art entry from its `meta` block. The runtime never reads `meta`, but
 * a generation tool does: `prompt` is the subject, `style` lists extra style phrases, and `negative` is
 * extra negative text. Other keys, such as `notes`, are for people and are not read.
 */
function readNotes(entry: ArtEntry, issues: Issues): EntryNotes {
  const meta = entry.meta;
  const path = (key: string): string => `${entry.id}.meta.${key}`;

  const subjectText = meta.prompt === undefined ? undefined : readText(meta.prompt, path("prompt"), issues, PHRASE_LIMIT);
  const subject = subjectText === undefined ? undefined : { text: subjectText, weight: 1 };

  const style: Phrase[] = [];
  if (meta.style !== undefined) {
    const list = readArray(meta.style, path("style"), issues, 32) ?? [];
    for (const item of list) {
      const text = readText(item, path("style"), issues, PHRASE_LIMIT);
      if (text !== undefined) style.push({ text, weight: 1 });
    }
  }

  const negative: string[] = [];
  if (meta.negative !== undefined) {
    const text = readText(meta.negative, path("negative"), issues, 1000);
    if (text !== undefined) {
      for (const piece of text.split(",")) {
        const trimmed = piece.trim();
        if (trimmed.length > 0) negative.push(trimmed);
      }
    }
  }
  return { subject, style, negative };
}

/** Copies an input image reference. */
function copyRef(ref: ArtRef): ArtRef {
  return { id: ref.id, variant: ref.variant };
}

/** Copies the generation settings of a profile, so that a job does not share them with the profile. */
function copyGeneration(settings: GenerationSettings): GenerationSettings {
  return {
    model: settings.model,
    loras: settings.loras.map((lora) => ({ name: lora.name, weight: lora.weight })),
    sampler: settings.sampler,
    scheduler: settings.scheduler,
    steps: settings.steps,
    cfg: settings.cfg,
    shift: settings.shift,
  };
}
