// Reads an audio manifest: JSON text in, a checked AudioManifest out. Every rule of
// docs/formats/AUDIO.md is checked here. A strict read refuses a document that has any problem, and
// reports all of them. A lenient read leaves out the entries that have problems, and reports the same
// issues. Both share one reader, so they agree on every rule.

import { AudioError, type AudioErrorCode, type AudioIssue } from "./error";
import { isRecord } from "./guards";
import {
  AUDIO_FORMAT,
  AUDIO_MAX_CANDIDATES,
  AUDIO_VERSION,
  BUILT_IN_BUSES,
  DEFAULT_BUS,
  MASTER_BUS,
  isBuiltInBus,
  type AudioLoopPoints,
  type AudioManifest,
  type AudioSound,
  type AudioVariant,
} from "./types";

/** A sound id: dotted segments of lowercase letters, digits, `_` and `-`. */
const ID_PATTERN = /^[a-z0-9_-]+(\.[a-z0-9_-]+)*$/;

/** The longest sound id, in characters. */
const ID_LIMIT = 128;

/** A custom bus name: a lowercase letter, then up to 31 lowercase letters, digits, `_` or `-`. */
const BUS_PATTERN = /^[a-z][a-z0-9_-]{0,31}$/;

/** A file path: lowercase folders, then a lowercase name that ends in one of the audio extensions. */
const PATH_PATTERN = /^(?:[a-z0-9_-]+\/)*[a-z0-9_-][a-z0-9_.-]*\.(?:opus|ogg|mp3|wav)$/;

/** The longest file path, in characters. */
const PATH_LIMIT = 256;

/** What a read of a document gives: the manifest without its bad entries, and every issue found. */
export interface LenientAudioManifest {
  /** The manifest in its normal form. The entries that had problems are left out. */
  readonly manifest: AudioManifest;
  /** Every problem found, in the order of the document. Empty for a valid manifest. */
  readonly issues: readonly AudioIssue[];
}

/** The state that a read of the sounds shares. */
interface SoundReader {
  /** The custom bus names that were accepted. */
  readonly buses: readonly string[];
  /** The ids that sounds have used so far, including the sounds that were left out. */
  readonly seen: Set<string>;
  /** Every issue found so far. */
  readonly issues: AudioIssue[];
}

/** Builds the error of a document that fails before its sounds are read. */
function fatal(code: AudioErrorCode, message: string, path?: string): AudioError {
  const issue: AudioIssue = path === undefined ? { code, message } : { code, message, path };
  return new AudioError(code, message, [issue]);
}

/** Reads the custom bus names. A bad name is reported and left out; a list that is not an array is fatal. */
function readCustomBuses(raw: unknown, issues: AudioIssue[]): string[] {
  if (raw === undefined) return [];
  if (!Array.isArray(raw)) throw fatal("structure", "buses must be an array of bus names", "buses");
  const items: readonly unknown[] = raw;
  const names: string[] = [];
  for (const [index, item] of items.entries()) {
    const path = `buses[${index}]`;
    if (typeof item !== "string") {
      issues.push({ code: "structure", message: "a bus name must be a string", path });
    } else if (item === MASTER_BUS) {
      issues.push({ code: "value", message: `"${item}" is reserved for the master bus`, path });
    } else if (isBuiltInBus(item)) {
      issues.push({ code: "duplicate", message: `"${item}" is already a built-in bus`, path });
    } else if (!BUS_PATTERN.test(item)) {
      issues.push({
        code: "value",
        message: `"${item}" is not a bus name: a lowercase letter, then up to 31 lowercase letters, digits, "_" or "-"`,
        path,
      });
    } else if (names.includes(item)) {
      issues.push({ code: "duplicate", message: `"${item}" is declared twice`, path });
    } else {
      names.push(item);
    }
  }
  return names;
}

/** Reads a sound id. Returns undefined, with an issue, when the id is not valid. */
function readSoundId(raw: unknown, path: string, issues: AudioIssue[]): string | undefined {
  if (typeof raw !== "string") {
    issues.push({ code: "structure", message: "id must be a string", path });
    return undefined;
  }
  if (raw.length > ID_LIMIT || !ID_PATTERN.test(raw)) {
    issues.push({
      code: "value",
      message: `"${raw}" is not a sound id: lowercase segments joined by dots, at most ${ID_LIMIT} characters`,
      path,
    });
    return undefined;
  }
  return raw;
}

/** Reads a list of candidate files. Returns undefined, with issues, when the list or one of its paths is wrong. */
function readCandidates(raw: unknown, path: string, issues: AudioIssue[]): string[] | undefined {
  if (!Array.isArray(raw)) {
    issues.push({ code: "structure", message: `${path} must be an array of file paths`, path });
    return undefined;
  }
  const items: readonly unknown[] = raw;
  if (items.length === 0 || items.length > AUDIO_MAX_CANDIDATES) {
    issues.push({ code: "value", message: `${path} must list from 1 to ${AUDIO_MAX_CANDIDATES} files`, path });
    return undefined;
  }
  const files: string[] = [];
  let valid = true;
  for (const [index, item] of items.entries()) {
    const itemPath = `${path}[${index}]`;
    if (typeof item !== "string") {
      issues.push({ code: "structure", message: "a file path must be a string", path: itemPath });
      valid = false;
    } else if (item.length > PATH_LIMIT || !PATH_PATTERN.test(item)) {
      issues.push({
        code: "value",
        message: `"${item}" is not a file path: lowercase folders, and a name ending in .opus, .ogg, .mp3 or .wav`,
        path: itemPath,
      });
      valid = false;
    } else if (files.includes(item)) {
      issues.push({ code: "duplicate", message: `"${item}" is listed twice`, path: itemPath });
      valid = false;
    } else {
      files.push(item);
    }
  }
  return valid ? files : undefined;
}

/** Reads the variants of a sound. Returns undefined, with issues, when the list is wrong. */
function readVariantList(raw: unknown, path: string, issues: AudioIssue[]): AudioVariant[] | undefined {
  if (!Array.isArray(raw)) {
    issues.push({ code: "structure", message: `${path} must be an array of variants`, path });
    return undefined;
  }
  const items: readonly unknown[] = raw;
  if (items.length < 2) {
    issues.push({
      code: "value",
      message: `${path} needs at least 2 variants; a sound with one version lists its files directly`,
      path,
    });
    return undefined;
  }
  const variants: AudioVariant[] = [];
  let valid = true;
  for (const [index, item] of items.entries()) {
    const itemPath = `${path}[${index}]`;
    if (!isRecord(item)) {
      issues.push({ code: "structure", message: "a variant must be an object with a files list", path: itemPath });
      valid = false;
      continue;
    }
    const files = readCandidates(item.files, `${itemPath}.files`, issues);
    if (files === undefined) valid = false;
    else variants.push({ files });
  }
  return valid ? variants : undefined;
}

/**
 * Reads where a sound's files come from: a single list in `files`, or a list of variants. A sound
 * has one of the two. Returns the variants in their normal form, where a single list is one variant.
 */
function readSource(raw: Record<string, unknown>, path: string, issues: AudioIssue[]): AudioVariant[] | undefined {
  const hasFiles = raw.files !== undefined;
  const hasVariants = raw.variants !== undefined;
  if (hasFiles && hasVariants) {
    issues.push({ code: "value", message: `${path} has both files and variants; use one of them`, path });
    return undefined;
  }
  if (hasVariants) return readVariantList(raw.variants, `${path}.variants`, issues);
  if (!hasFiles) {
    issues.push({ code: "structure", message: `${path} needs files, or variants`, path });
    return undefined;
  }
  const files = readCandidates(raw.files, `${path}.files`, issues);
  return files === undefined ? undefined : [{ files }];
}

/** Reads a bus name. A missing bus is the default; a name that is neither built in nor declared is an error. */
function readBus(raw: unknown, path: string, buses: readonly string[], issues: AudioIssue[]): string | undefined {
  if (raw === undefined) return DEFAULT_BUS;
  if (typeof raw !== "string") {
    issues.push({ code: "structure", message: "bus must be a bus name", path });
    return undefined;
  }
  if (isBuiltInBus(raw) || buses.includes(raw)) return raw;
  issues.push({
    code: "value",
    message: `"${raw}" is not a bus: use ${BUILT_IN_BUSES.join(", ")}, or a bus declared in buses`,
    path,
  });
  return undefined;
}

/** Reads a volume from 0 to 1. A missing volume is 1. */
function readVolume(raw: unknown, path: string, issues: AudioIssue[]): number | undefined {
  if (raw === undefined) return 1;
  if (typeof raw !== "number") {
    issues.push({ code: "structure", message: "volume must be a number", path });
    return undefined;
  }
  if (!Number.isFinite(raw) || raw < 0 || raw > 1) {
    issues.push({ code: "value", message: `${raw} is not a volume: use a number from 0 to 1`, path });
    return undefined;
  }
  return raw;
}

/** Reads a true-or-false field. A missing field is the fallback. */
function readFlag(raw: unknown, path: string, fallback: boolean, issues: AudioIssue[]): boolean | undefined {
  if (raw === undefined) return fallback;
  if (typeof raw !== "boolean") {
    issues.push({ code: "structure", message: `${path} must be true or false`, path });
    return undefined;
  }
  return raw;
}

/**
 * Reads the loop points. A missing field gives null. `loop` is the sound's loop flag, or undefined
 * when that field was already reported; then the points are not checked, because the sound is left
 * out anyway.
 */
function readLoopPoints(
  raw: unknown,
  loop: boolean | undefined,
  path: string,
  issues: AudioIssue[],
): AudioLoopPoints | null | undefined {
  if (raw === undefined) return null;
  if (!isRecord(raw)) {
    issues.push({ code: "structure", message: `${path} must be an object with start and end`, path });
    return undefined;
  }
  const start = raw.start;
  const end = raw.end;
  if (typeof start !== "number" || typeof end !== "number") {
    issues.push({ code: "structure", message: `${path} needs numeric start and end`, path });
    return undefined;
  }
  if (loop === undefined) return undefined;
  if (loop === false) {
    issues.push({ code: "value", message: `${path} needs loop: true`, path });
    return undefined;
  }
  if (!Number.isFinite(start) || !Number.isFinite(end) || start < 0 || end <= start) {
    issues.push({ code: "value", message: `${path}: start must be zero or more, and end must be after start`, path });
    return undefined;
  }
  return { start, end };
}

/** Reads the voice limit. A missing limit is null, which means no limit. */
function readMaxVoices(raw: unknown, path: string, issues: AudioIssue[]): number | null | undefined {
  if (raw === undefined) return null;
  if (typeof raw !== "number") {
    issues.push({ code: "structure", message: "max_voices must be a number", path });
    return undefined;
  }
  if (!Number.isInteger(raw) || raw < 1) {
    issues.push({ code: "value", message: "max_voices must be a whole number of at least 1", path });
    return undefined;
  }
  return raw;
}

/** Reads the cooldown in milliseconds. A missing cooldown is 0. */
function readCooldown(raw: unknown, path: string, issues: AudioIssue[]): number | undefined {
  if (raw === undefined) return 0;
  if (typeof raw !== "number") {
    issues.push({ code: "structure", message: "cooldown_ms must be a number", path });
    return undefined;
  }
  if (!Number.isInteger(raw) || raw < 0) {
    issues.push({ code: "value", message: "cooldown_ms must be a whole number of 0 or more", path });
    return undefined;
  }
  return raw;
}

/** Reads the notes of a sound. A missing block is an empty object. */
function readMeta(raw: unknown, path: string, issues: AudioIssue[]): Readonly<Record<string, unknown>> | undefined {
  if (raw === undefined) return {};
  if (!isRecord(raw)) {
    issues.push({ code: "structure", message: "meta must be an object", path });
    return undefined;
  }
  return raw;
}

/**
 * Reads one sound. Returns undefined when any of its fields is wrong; the issues say which. The id
 * is checked for repeats first: a sound whose id an earlier sound already used is left out, even
 * when that earlier sound was left out too.
 */
function readSound(raw: unknown, index: number, reader: SoundReader): AudioSound | undefined {
  const path = `sounds[${index}]`;
  if (!isRecord(raw)) {
    reader.issues.push({ code: "structure", message: "a sound must be an object", path });
    return undefined;
  }
  const before = reader.issues.length;
  const id = readSoundId(raw.id, `${path}.id`, reader.issues);
  if (id !== undefined) {
    if (reader.seen.has(id)) {
      reader.issues.push({ code: "duplicate", message: `"${id}" repeats the id of an earlier sound`, path: `${path}.id` });
      return undefined;
    }
    reader.seen.add(id);
  }
  const variants = readSource(raw, path, reader.issues);
  const bus = readBus(raw.bus, `${path}.bus`, reader.buses, reader.issues);
  const volume = readVolume(raw.volume, `${path}.volume`, reader.issues);
  const loop = readFlag(raw.loop, `${path}.loop`, false, reader.issues);
  const loopPoints = readLoopPoints(raw.loop_points, loop, `${path}.loop_points`, reader.issues);
  const maxVoices = readMaxVoices(raw.max_voices, `${path}.max_voices`, reader.issues);
  const cooldown = readCooldown(raw.cooldown_ms, `${path}.cooldown_ms`, reader.issues);
  const preload = readFlag(raw.preload, `${path}.preload`, false, reader.issues);
  const meta = readMeta(raw.meta, `${path}.meta`, reader.issues);
  if (
    reader.issues.length > before ||
    id === undefined ||
    variants === undefined ||
    bus === undefined ||
    volume === undefined ||
    loop === undefined ||
    loopPoints === undefined ||
    maxVoices === undefined ||
    cooldown === undefined ||
    preload === undefined ||
    meta === undefined
  ) {
    return undefined;
  }
  return {
    id,
    bus,
    volume,
    loop,
    loop_points: loopPoints,
    max_voices: maxVoices,
    cooldown_ms: cooldown,
    preload,
    variants,
    meta,
  };
}

/**
 * Reads a whole document. Only the envelope and the two lists are fatal; every other problem is
 * reported and its entry is left out.
 */
function readDocument(json: string): LenientAudioManifest {
  let parsed: unknown;
  try {
    parsed = JSON.parse(json);
  } catch (cause) {
    throw new AudioError("json", "audio manifest is not valid JSON", [{ code: "json", message: "the text is not valid JSON" }], {
      cause,
    });
  }
  if (!isRecord(parsed)) throw fatal("structure", "the manifest is not a JSON object");
  if (parsed.format !== AUDIO_FORMAT) throw fatal("format", `the format tag is not "${AUDIO_FORMAT}"`, "format");
  if (parsed.version !== AUDIO_VERSION) throw fatal("version", `the version is not ${AUDIO_VERSION}`, "version");

  const issues: AudioIssue[] = [];
  const buses = readCustomBuses(parsed.buses, issues);
  const rawSounds: unknown = parsed.sounds;
  if (!Array.isArray(rawSounds)) throw fatal("structure", "sounds must be an array of sounds", "sounds");
  const items: readonly unknown[] = rawSounds;

  const reader: SoundReader = { buses, seen: new Set<string>(), issues };
  const sounds: AudioSound[] = [];
  for (const [index, raw] of items.entries()) {
    const sound = readSound(raw, index, reader);
    if (sound !== undefined) sounds.push(sound);
  }
  return { manifest: { format: AUDIO_FORMAT, version: AUDIO_VERSION, buses, sounds }, issues };
}

/**
 * Reads an audio manifest strictly.
 * @param json - the text of the manifest file.
 * @returns the manifest in its normal form: defaults filled in, sounds in the order of the file.
 * @throws AudioError. Its `code` is the code of the first problem: "json", "format", "version",
 *   "structure", "value" or "duplicate". Its `issues` list every problem that was found.
 */
export function parseAudioManifest(json: string): AudioManifest {
  const { manifest, issues } = readDocument(json);
  const first = issues[0];
  if (first !== undefined) {
    const count = issues.length === 1 ? "1 problem" : `${issues.length} problems`;
    throw new AudioError(first.code, `audio manifest has ${count}; the first: ${first.message}`, issues);
  }
  return manifest;
}

/**
 * Reads an audio manifest leniently: the envelope must be right, but a bad custom bus or a bad
 * sound is left out and reported, and the other entries are kept.
 * @param json - the text of the manifest file.
 * @throws AudioError for the envelope only: text that is not JSON, a wrong format or version, or a
 *   `sounds` or `buses` that is not an array.
 */
export function parseAudioManifestLenient(json: string): LenientAudioManifest {
  return readDocument(json);
}
