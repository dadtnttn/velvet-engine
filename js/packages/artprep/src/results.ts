// Reads a results document: what a runner reports after it made the candidates of each shot. The
// gallery shows it (docs/formats/ART_PREP.md, "Selection gallery"). A candidate's name must be the name
// that its job gives it, so that the code a person copies is the file the pipeline expects.

import {
  addIssue,
  ART_ID_LIMIT,
  ART_ID_PATTERN,
  checkKeys,
  ENVELOPE_KEYS,
  joinPath,
  problemsError,
  readArray,
  readDocument,
  readFreeText,
  readIdentifier,
  readInteger,
  readObject,
  readOptionalVariant,
  readText,
  SLUG_PATTERN,
  type Issues,
} from "./read";
import { outputName } from "./jobs";
import type { GalleryCandidate, ResultsDocument, ShotResult } from "./types";

/** The most shots one results document may hold. */
const MAX_SHOTS = 500;

/** The most candidates one shot may list. */
const MAX_CANDIDATES = 256;

/** The longest prompt that a result may carry, in characters. */
const PROMPT_LIMIT = 4000;

/** The longest image source, in characters: a data URL can be long. */
const IMAGE_LIMIT = 1_000_000;

/** A data URL of a picture in one of the common formats. */
const DATA_IMAGE = /^data:image\/(png|jpeg|webp|gif|avif);base64,[A-Za-z0-9+/=]+$/;

/** An http or https address with no spaces, quotes or angle brackets. */
const WEB_ADDRESS = /^https?:\/\/[^\s"'<>]+$/i;

/**
 * True when an image source is safe to put in a gallery page: a relative path with `/` separators, an
 * http or https address, or a data URL of a picture. Anything else (a `javascript:` URL, an absolute
 * path, a Windows drive path) is refused.
 * @param source - the image source, as the runner gives it.
 */
export function isSafeImage(source: string): boolean {
  if (DATA_IMAGE.test(source) || WEB_ADDRESS.test(source)) return true;
  if (source.length === 0 || source.startsWith("/") || source.includes("\\")) return false;
  return !/^[a-z][a-z0-9+.-]*:/i.test(source);
}

/**
 * Parses the text of a results document. Throws ArtPrepError, listing every problem found.
 * @param text - the JSON text of the document.
 */
export function parseResults(text: string): ResultsDocument {
  const document = readDocument(text, "results");
  const issues: Issues = [];
  checkKeys(document, [...ENVELOPE_KEYS, "title", "shots"], "", issues);
  const title = document.title === undefined ? null : (readText(document.title, "title", issues, 200) ?? null);
  const list = readArray(document.shots, "shots", issues, MAX_SHOTS) ?? [];
  const shots: ShotResult[] = [];
  const names = new Set<string>();
  list.forEach((item, index) => {
    const path = `shots[${index}]`;
    const shot = readShotResult(item, path, issues);
    if (shot === undefined) return;
    if (names.has(shot.shot)) addIssue(issues, "duplicate", `shot "${shot.shot}" is listed twice`, `${path}.shot`);
    names.add(shot.shot);
    shots.push(shot);
  });
  if (issues.length > 0) throw problemsError(issues, "results document");
  return { title, shots };
}

/** Reads the results of one shot. */
function readShotResult(value: unknown, path: string, issues: Issues): ShotResult | undefined {
  const object = readObject(value, path, issues);
  if (object === undefined) return undefined;
  checkKeys(object, ["shot", "art", "variant", "prompt", "candidates"], path, issues);
  const shot = readIdentifier(object.shot, joinPath(path, "shot"), issues, SLUG_PATTERN, 64, "shot id");
  const art = readIdentifier(object.art, joinPath(path, "art"), issues, ART_ID_PATTERN, ART_ID_LIMIT, "art id");
  const variant = readOptionalVariant(object.variant, joinPath(path, "variant"), issues);
  const prompt =
    object.prompt === undefined || object.prompt === null
      ? null
      : (readFreeText(object.prompt, joinPath(path, "prompt"), issues, PROMPT_LIMIT) ?? null);
  const candidates = readCandidates(object.candidates, joinPath(path, "candidates"), issues, art, variant);
  if (shot === undefined || art === undefined) return undefined;
  return { shot, art, variant, prompt, candidates };
}

/** Reads the candidates of a shot. Each name must be the output name of its seed. */
function readCandidates(
  value: unknown,
  path: string,
  issues: Issues,
  art: string | undefined,
  variant: string | null,
): GalleryCandidate[] {
  if (value === undefined) return [];
  const list = readArray(value, path, issues, MAX_CANDIDATES) ?? [];
  const candidates: GalleryCandidate[] = [];
  list.forEach((item, index) => {
    const at = `${path}[${index}]`;
    const object = readObject(item, at, issues);
    if (object === undefined) return;
    checkKeys(object, ["seed", "name", "image"], at, issues);
    const seed = readInteger(object.seed, joinPath(at, "seed"), issues, 0, 4294967295);
    const name = readText(object.name, joinPath(at, "name"), issues, 300);
    const image = readText(object.image, joinPath(at, "image"), issues, IMAGE_LIMIT);
    if (seed === undefined || name === undefined || image === undefined) return;
    if (art !== undefined && name !== outputName(art, variant, seed)) {
      const expected = outputName(art, variant, seed);
      addIssue(issues, "value", `candidate name "${name}" should be "${expected}"`, joinPath(at, "name"));
    }
    if (!isSafeImage(image)) {
      const message = `image "${image}" must be a relative path, an https address or a data:image URL`;
      addIssue(issues, "value", message, joinPath(at, "image"));
    }
    candidates.push({ seed, name, image });
  });
  return candidates;
}
