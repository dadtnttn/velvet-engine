// Reads a profiles document: the generation setups of a project (docs/formats/ART_PREP.md, "Profiles").
// Every rule of a profile is checked here, and all the problems of a document are reported together.

import {
  addIssue,
  checkKeys,
  ENVELOPE_KEYS,
  joinPath,
  PHRASE_LIMIT,
  problemsError,
  readArray,
  readBoolean,
  readChoice,
  readDocument,
  readIdentifier,
  readInteger,
  readJoiner,
  readNumber,
  readObject,
  readText,
  readTextList,
  SLUG_PATTERN,
  type Issues,
} from "./read";
import {
  PROMPT_LAYERS,
  VARIATION_FIELDS,
  type ArtPrepProfile,
  type GenerationSettings,
  type LoraSetting,
  type PromptLayer,
  type PromptRules,
  type SizeRule,
  type VariationField,
} from "./types";

/** The most profiles one document may hold. */
const MAX_PROFILES = 64;

/** The names of a profile. */
const PROFILE_KEYS = [
  "id",
  "label",
  "mode",
  "generation",
  "size",
  "max_references",
  "max_identities",
  "negative",
  "trigger_words",
  "prompt",
  "edit_guards",
];

/** The names of the generation settings. */
const GENERATION_KEYS = ["model", "loras", "sampler", "scheduler", "steps", "cfg", "shift"];

/** The names of one LoRA. */
const LORA_KEYS = ["name", "weight"];

/** The names of the size rule. */
const SIZE_KEYS = ["long_side", "multiple_of"];

/** The names of the prompt rules. */
const PROMPT_KEYS = ["style", "order", "quality_tags", "weights", "phrase_joiner", "layer_joiner", "layer_end"];

/**
 * Parses the text of a profiles document. The profiles come in the order of the file, with every
 * default filled in. Throws ArtPrepError, listing every problem found.
 * @param text - the JSON text of the document.
 */
export function parseProfiles(text: string): readonly ArtPrepProfile[] {
  const document = readDocument(text, "profiles");
  const issues: Issues = [];
  checkKeys(document, [...ENVELOPE_KEYS, "profiles"], "", issues);
  const list = readArray(document.profiles, "profiles", issues, MAX_PROFILES) ?? [];
  const profiles: ArtPrepProfile[] = [];
  const ids = new Set<string>();
  list.forEach((item, index) => {
    const path = `profiles[${index}]`;
    const profile = readProfile(item, path, issues);
    if (profile === undefined) return;
    if (ids.has(profile.id)) addIssue(issues, "duplicate", `profile "${profile.id}" is listed twice`, `${path}.id`);
    ids.add(profile.id);
    profiles.push(profile);
  });
  if (issues.length > 0) throw problemsError(issues, "profiles document");
  return profiles;
}

/** Reads one profile. Returns undefined when a required part is wrong; the problem is recorded. */
function readProfile(value: unknown, path: string, issues: Issues): ArtPrepProfile | undefined {
  const object = readObject(value, path, issues);
  if (object === undefined) return undefined;
  checkKeys(object, PROFILE_KEYS, path, issues);
  const id = readIdentifier(object.id, joinPath(path, "id"), issues, SLUG_PATTERN, 64, "profile id");
  const label = object.label === undefined ? undefined : readText(object.label, joinPath(path, "label"), issues, 80);
  const mode = readChoice(object.mode, joinPath(path, "mode"), issues, ["generate", "edit"] as const);
  const generation = readGeneration(object.generation, joinPath(path, "generation"), issues);
  const size = readSize(object.size, joinPath(path, "size"), issues);
  const maxReferences =
    object.max_references === undefined
      ? 0
      : readInteger(object.max_references, joinPath(path, "max_references"), issues, 0, 8);
  const maxIdentities =
    object.max_identities === undefined
      ? 2
      : readInteger(object.max_identities, joinPath(path, "max_identities"), issues, 1, 2);
  const negative = readBoolean(object.negative, joinPath(path, "negative"), issues);
  const triggerWords =
    object.trigger_words === undefined
      ? []
      : readTextList(object.trigger_words, joinPath(path, "trigger_words"), issues, 8, 64);
  const prompt = readPrompt(object.prompt, joinPath(path, "prompt"), issues);
  const editGuards = readEditGuards(object.edit_guards, joinPath(path, "edit_guards"), issues);

  if (mode === "edit" && maxReferences !== undefined && maxReferences < 1) {
    const at = joinPath(path, "max_references");
    addIssue(issues, "value", `${at} must be at least 1 for an edit profile: the picture being edited`, at);
  }
  if (mode !== undefined && mode !== "edit" && Object.keys(editGuards).length > 0) {
    const at = joinPath(path, "edit_guards");
    addIssue(issues, "value", `${at} is only for edit profiles`, at);
  }

  if (
    id === undefined ||
    mode === undefined ||
    generation === undefined ||
    size === undefined ||
    maxReferences === undefined ||
    maxIdentities === undefined ||
    negative === undefined ||
    prompt === undefined
  ) {
    return undefined;
  }
  return {
    id,
    label: label ?? id,
    mode,
    generation,
    size,
    maxReferences,
    maxIdentities,
    negative,
    triggerWords,
    prompt,
    editGuards,
  };
}

/** Reads the generation settings of a profile. */
function readGeneration(value: unknown, path: string, issues: Issues): GenerationSettings | undefined {
  const object = readObject(value, path, issues);
  if (object === undefined) return undefined;
  checkKeys(object, GENERATION_KEYS, path, issues);
  const model = readText(object.model, joinPath(path, "model"), issues, 120);
  const loras = object.loras === undefined ? [] : readLoras(object.loras, joinPath(path, "loras"), issues);
  const sampler = readText(object.sampler, joinPath(path, "sampler"), issues, 60);
  const scheduler =
    object.scheduler === undefined ? null : (readText(object.scheduler, joinPath(path, "scheduler"), issues, 60) ?? null);
  const steps = readInteger(object.steps, joinPath(path, "steps"), issues, 1, 200);
  const cfg = readNumber(object.cfg, joinPath(path, "cfg"), issues, 0, 30);
  const shift =
    object.shift === undefined ? null : (readNumber(object.shift, joinPath(path, "shift"), issues, 0, 20) ?? null);
  if (model === undefined || sampler === undefined || steps === undefined || cfg === undefined) return undefined;
  return { model, loras, sampler, scheduler, steps, cfg, shift };
}

/** Reads the LoRAs of a profile. A LoRA's strength is 1 unless it says otherwise. */
function readLoras(value: unknown, path: string, issues: Issues): LoraSetting[] {
  const list = readArray(value, path, issues, 8);
  const loras: LoraSetting[] = [];
  list?.forEach((item, index) => {
    const at = `${path}[${index}]`;
    const object = readObject(item, at, issues);
    if (object === undefined) return;
    checkKeys(object, LORA_KEYS, at, issues);
    const name = readText(object.name, joinPath(at, "name"), issues, 120);
    const weight =
      object.weight === undefined ? 1 : readNumber(object.weight, joinPath(at, "weight"), issues, 0, 2);
    if (name !== undefined && weight !== undefined) loras.push({ name, weight });
  });
  return loras;
}

/** Reads the size rule: the long side in pixels, which must be a multiple of the rounding step. */
function readSize(value: unknown, path: string, issues: Issues): SizeRule | undefined {
  const object = readObject(value, path, issues);
  if (object === undefined) return undefined;
  checkKeys(object, SIZE_KEYS, path, issues);
  const longSide = readInteger(object.long_side, joinPath(path, "long_side"), issues, 64, 4096);
  const multipleOf = readInteger(object.multiple_of, joinPath(path, "multiple_of"), issues, 8, 256);
  if (longSide === undefined || multipleOf === undefined) return undefined;
  if (longSide % multipleOf !== 0) {
    const at = joinPath(path, "long_side");
    addIssue(issues, "value", `${at} must be a multiple of multiple_of (${multipleOf})`, at);
    return undefined;
  }
  return { longSide, multipleOf };
}

/** Reads the prompt rules of a profile. Each default depends on whether the profile writes prose or tags. */
function readPrompt(value: unknown, path: string, issues: Issues): PromptRules | undefined {
  const object = readObject(value, path, issues);
  if (object === undefined) return undefined;
  checkKeys(object, PROMPT_KEYS, path, issues);
  const style = readChoice(object.style, joinPath(path, "style"), issues, ["prose", "tags"] as const);
  const order =
    object.order === undefined ? [...PROMPT_LAYERS] : readOrder(object.order, joinPath(path, "order"), issues);
  const qualityTags =
    object.quality_tags === undefined
      ? []
      : readTextList(object.quality_tags, joinPath(path, "quality_tags"), issues, 16, 64);
  const weights =
    object.weights === undefined
      ? style === "tags"
        ? "paren"
        : "none"
      : readChoice(object.weights, joinPath(path, "weights"), issues, ["paren", "none"] as const);
  const phraseJoiner =
    object.phrase_joiner === undefined ? ", " : readJoiner(object.phrase_joiner, joinPath(path, "phrase_joiner"), issues);
  const layerJoiner =
    object.layer_joiner === undefined
      ? style === "prose"
        ? " "
        : ", "
      : readJoiner(object.layer_joiner, joinPath(path, "layer_joiner"), issues);
  const layerEnd =
    object.layer_end === undefined
      ? style === "prose"
        ? "."
        : ""
      : readJoiner(object.layer_end, joinPath(path, "layer_end"), issues);

  if (style === "prose" && weights === "paren") {
    const at = joinPath(path, "weights");
    addIssue(issues, "value", `${at} "paren" is for tag prompts; prose prompts have no weight syntax`, at);
  }
  if (style === "prose" && qualityTags.length > 0) {
    const at = joinPath(path, "quality_tags");
    addIssue(issues, "value", `${at} is for tag prompts; prose prompts take no quality tags`, at);
  }
  if (
    style === undefined ||
    order === undefined ||
    weights === undefined ||
    phraseJoiner === undefined ||
    layerJoiner === undefined ||
    layerEnd === undefined
  ) {
    return undefined;
  }
  return { style, order, qualityTags, weights, phraseJoiner, layerJoiner, layerEnd };
}

/** Reads the layer order: the three layers, each named once. */
function readOrder(value: unknown, path: string, issues: Issues): PromptLayer[] | undefined {
  const list = readArray(value, path, issues, PROMPT_LAYERS.length);
  if (list === undefined) return undefined;
  const layers: PromptLayer[] = [];
  list.forEach((item, index) => {
    const layer = readChoice(item, `${path}[${index}]`, issues, PROMPT_LAYERS);
    if (layer !== undefined) layers.push(layer);
  });
  const complete = layers.length === PROMPT_LAYERS.length && PROMPT_LAYERS.every((name) => layers.includes(name));
  if (!complete) {
    addIssue(issues, "value", `${path} must name each layer once: ${PROMPT_LAYERS.join(", ")}`, path);
    return undefined;
  }
  return layers;
}

/** Reads the edit guards: extra text for the variation fields that an edit may change. */
function readEditGuards(value: unknown, path: string, issues: Issues): Partial<Record<VariationField, string>> {
  const guards: Partial<Record<VariationField, string>> = {};
  if (value === undefined) return guards;
  const object = readObject(value, path, issues);
  if (object === undefined) return guards;
  for (const key of Object.keys(object)) {
    const at = joinPath(path, key);
    const field = VARIATION_FIELDS.find((name) => name === key);
    if (field === undefined) {
      addIssue(issues, "unknown-field", `"${at}" is not a variation field`, at);
      continue;
    }
    const text = readText(object[key], at, issues, PHRASE_LIMIT);
    if (text !== undefined) guards[field] = text;
  }
  return guards;
}
