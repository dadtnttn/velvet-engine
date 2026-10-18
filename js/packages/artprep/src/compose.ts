// Composes the prompt of one picture from its three layers, in the way a profile says: the style layer,
// the identity layer of each character, and the variation layer of the shot. The rules are in
// docs/formats/ART_PREP.md, "Composition".

import type {
  ArtPrepProfile,
  CharacterIdentity,
  ComposedPrompt,
  Phrase,
  PromptRules,
  ShotVariation,
  StyleLayer,
} from "./types";
import { VARIATION_FIELDS } from "./types";

/** A phrase with the weight 1. */
function plain(text: string): Phrase {
  return { text, weight: 1 };
}

/** Writes a weight with at most two decimals, without trailing zeros: 1.2 gives "1.2", 0.25 gives "0.25". */
function formatWeight(weight: number): string {
  return String(Number(weight.toFixed(2)));
}

/** Writes one phrase. Only the paren syntax writes weights, and a weight of 1 is never written. */
function renderPhrase(phrase: Phrase, rules: PromptRules): string {
  if (rules.weights === "paren" && phrase.weight !== 1) return `(${phrase.text}:${formatWeight(phrase.weight)})`;
  return phrase.text;
}

/** Makes the first letter a capital. Used for prose, where each layer is a sentence. */
function capitalize(text: string): string {
  return text.charAt(0).toUpperCase() + text.slice(1);
}

/**
 * The phrases of one character's identity layer, in their order: the name, the species, the body, the
 * skin, the hair, the eyes, the features the character has, the outfit, the features it lacks (written
 * as `no <feature>` only when the profile takes no negative prompt), and the fixed phrases.
 */
function identityPhrases(one: CharacterIdentity, negative: boolean): Phrase[] {
  const phrases: Phrase[] = [plain(one.name)];
  for (const text of [one.species, one.body, one.skin, one.hair, one.eyes]) {
    if (text !== null) phrases.push(plain(text));
  }
  phrases.push(...one.has);
  if (one.outfit !== null) phrases.push(plain(one.outfit));
  if (!negative) {
    for (const item of one.lacks) phrases.push(plain(`no ${item}`));
  }
  phrases.push(...one.fixed);
  return phrases;
}

/**
 * The phrases of the variation layer, in the order of VARIATION_FIELDS. In an edit profile, the guard of
 * a field follows the field when the profile gives one.
 */
function variationPhrases(variation: ShotVariation, profile: ArtPrepProfile): Phrase[] {
  const phrases: Phrase[] = [];
  for (const field of VARIATION_FIELDS) {
    const phrase = variation[field];
    if (phrase === undefined) continue;
    phrases.push(phrase);
    const guard = profile.mode === "edit" ? profile.editGuards[field] : undefined;
    if (guard !== undefined) phrases.push(plain(guard));
  }
  return phrases;
}

/**
 * The negative prompt: the style's negative phrases, then the features that the identities lack, in
 * their order and without repeats. A lacked feature is left out when a character in the same picture
 * has a feature (a `has` phrase or a species) that contains its word, so that a negative never removes
 * a feature that the picture asks for. Returns null when the profile takes no negative prompt, or when
 * there is nothing to write.
 */
function negativeText(style: StyleLayer, identities: readonly CharacterIdentity[], profile: ArtPrepProfile): string | null {
  if (!profile.negative) return null;
  const items: string[] = [...style.negative];
  const features = identities
    .flatMap((one) => [one.species ?? "", ...one.has.map((phrase) => phrase.text)])
    .map((text) => text.toLowerCase());
  for (const one of identities) {
    for (const lack of one.lacks) {
      const word = lack.toLowerCase();
      if (features.some((feature) => feature.includes(word))) continue;
      items.push(lack);
    }
  }
  const unique = [...new Set(items)];
  return unique.length === 0 ? null : unique.join(profile.prompt.phraseJoiner);
}

/** True when the identity argument is a list of identities rather than one. */
function isList(value: CharacterIdentity | readonly CharacterIdentity[]): value is readonly CharacterIdentity[] {
  return Array.isArray(value);
}

/**
 * Composes the prompt of one picture.
 *
 * The text is the profile's leading tags (quality tags, then trigger words), then the layers in the order
 * the profile gives. A layer with no phrases is left out. Each layer's phrases are joined with the
 * phrase joiner and closed with the layer end. In a prose profile each layer starts with a capital
 * letter, and the layers are joined with the layer joiner. In a tag profile the weights are written
 * with the paren syntax, when the profile uses it.
 *
 * @param style - the style layer of the project, with any extra phrases of the art entry.
 * @param identity - the identity layer of one character, or the list of the characters in the picture.
 * @param variation - the variation layer of the shot.
 * @param profile - the profile that says how to write the prompt.
 * @returns the positive prompt (it may be empty, when nothing is given) and the negative prompt, or null.
 */
export function composePrompt(
  style: StyleLayer,
  identity: CharacterIdentity | readonly CharacterIdentity[],
  variation: ShotVariation,
  profile: ArtPrepProfile,
): ComposedPrompt {
  const identities: readonly CharacterIdentity[] = isList(identity) ? identity : [identity];
  const rules = profile.prompt;
  const bodies: string[] = [];

  const leading = [...rules.qualityTags, ...profile.triggerWords];
  if (leading.length > 0) bodies.push(leading.join(rules.phraseJoiner));

  const addLayer = (phrases: readonly Phrase[]): void => {
    if (phrases.length === 0) return;
    let text = phrases.map((phrase) => renderPhrase(phrase, rules)).join(rules.phraseJoiner);
    if (rules.style === "prose") text = capitalize(text);
    bodies.push(text + rules.layerEnd);
  };

  for (const layer of rules.order) {
    if (layer === "style") {
      addLayer(style.phrases);
    } else if (layer === "identity") {
      for (const one of identities) addLayer(identityPhrases(one, profile.negative));
    } else {
      addLayer(variationPhrases(variation, profile));
    }
  }

  return {
    prompt: bodies.join(rules.layerJoiner),
    negative: negativeText(style, identities, profile),
  };
}
