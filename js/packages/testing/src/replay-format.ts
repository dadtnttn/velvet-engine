// The replay format, version 1: a JSON document that holds one recorded session, its seeds, its
// starting save and the state checkpoints that a replay must reproduce. docs/formats/REPLAY.md
// describes the format; the fixtures in docs/formats/fixtures/replay are its conformance cases.
//
// A session is counted in frames. A frame is one update step of the game's loop: the inputs of frame
// N happen after N updates have run and before update N runs. A checkpoint at frame N holds the hash
// of the game's state after N updates.

import { SAVE_FORMAT, type MouseButtonName, type SaveEnvelope } from "@velvet/core";
import { TestingError } from "./errors";
import type { PadButtonState } from "./fake-pads";

/** The format tag that every replay document carries. */
export const REPLAY_FORMAT = "velvet-replay";

/** The version this build writes and reads. */
export const REPLAY_VERSION = 1;

/** How many frames pass between checkpoints when the recording names no interval: one second at 60 Hz. */
export const DEFAULT_CHECK_EVERY = 60;

/** The most pads a replay names: the index of a pad, from 0 to 15. */
const MAX_PAD_INDEX = 15;

/** The longest text that a game id, a seed name or a pad name may hold. */
const MAX_TEXT_LENGTH = 128;

/** The fastest loop rate that a replay may name, in steps per second. */
const MAX_RATE = 1000;

/** A KeyboardEvent.code value: letters and digits only, as every standard key name is. */
const KEY_PATTERN = /^[A-Za-z0-9]{1,32}$/;

/** Seed text: decimal digits, for seeds above 2^53. */
const SEED_TEXT_PATTERN = /^\d{1,40}$/;

/** The mouse buttons that an event may name. */
const MOUSE_BUTTONS: readonly MouseButtonName[] = ["left", "middle", "right"];

/** The delta modes that a wheel event may name: pixels, lines and pages. */
const DELTA_MODES: readonly number[] = [0, 1, 2];

/** Most buttons and axes that a pad event may carry. */
const MAX_PAD_BUTTONS = 32;
const MAX_PAD_AXES = 16;

/** A seed for one named random source: a safe whole number, or decimal digits. */
export type ReplaySeed = number | string;

/** The hash of the game's state after a number of frames. */
export interface ReplayCheckpoint {
  /** The number of updates that had run when the hash was taken. */
  readonly frame: number;
  /** The game's hash at that point, as the game's `hash()` returned it. */
  readonly hash: string;
}

/**
 * One recorded input, without its frame. The events are the ones that an action map counts, in
 * logical stage pixels for positions:
 * - a key that went down or up;
 * - a mouse button that went down or up, with the pointer position at that moment;
 * - a pointer move;
 * - a wheel tick;
 * - a pad that connected, changed or disconnected (the whole state of that pad at that moment);
 * - a window blur, which releases every held control.
 *
 * `skipped` is set on a press that the page had already used (a press on a text field or a button, or
 * one whose default was prevented). A replay marks it the same way, so the action map ignores it too.
 */
export type ReplayEvent =
  | { readonly type: "key"; readonly code: string; readonly down: boolean; readonly skipped?: true }
  | {
      readonly type: "button";
      readonly button: MouseButtonName;
      readonly down: boolean;
      readonly x: number;
      readonly y: number;
      readonly skipped?: true;
    }
  | { readonly type: "pointer"; readonly x: number; readonly y: number }
  | {
      readonly type: "wheel";
      readonly deltaX: number;
      readonly deltaY: number;
      readonly deltaMode: 0 | 1 | 2;
      readonly skipped?: true;
    }
  | { readonly type: "pad"; readonly pad: number; readonly connected: false }
  | {
      readonly type: "pad";
      readonly pad: number;
      readonly connected: true;
      readonly id: string;
      readonly buttons: readonly PadButtonState[];
      readonly axes: readonly number[];
    }
  | { readonly type: "blur" };

/** A recorded input, with the frame at which it happened. */
export type ReplayInput = ReplayEvent & { readonly frame: number };

/** A replay document: one recorded session, ready to be written as JSON and to be replayed. */
export interface Replay {
  readonly format: typeof REPLAY_FORMAT;
  readonly version: typeof REPLAY_VERSION;
  /** The id of the game that was recorded. A replay is played by a build of the same game. */
  readonly game: string;
  /** The game's own version, when the recording names one. For reading only: replays do not compare it. */
  readonly gameVersion?: string;
  /** The update steps per second of the loop that ran the session. */
  readonly rate: number;
  /** The number of updates in the session. */
  readonly frames: number;
  /** The named seeds that the game read at the start, such as the board's seed. */
  readonly seeds: Readonly<Record<string, ReplaySeed>>;
  /** The save that the session started from, in the save file format, or null for a new game. */
  readonly save: SaveEnvelope | null;
  /** How many frames pass between checkpoints. */
  readonly checkEvery: number;
  /** The state hashes, in frame order, that a replay must reproduce. */
  readonly checkpoints: readonly ReplayCheckpoint[];
  /** The recorded inputs, in frame order; inputs of one frame keep the order in which they happened. */
  readonly inputs: readonly ReplayInput[];
}

/**
 * Reads a replay from a value that came from JSON.
 * @throws TestingError with code "bad-replay" for the first field that does not match the format. The
 * message names the field, for example `inputs[3].frame must be ...`.
 */
export function parseReplay(value: unknown): Replay {
  const document = objectAt(value, "replay");
  if (document.format !== REPLAY_FORMAT) fail("format", `must be "${REPLAY_FORMAT}"`);
  if (document.version !== REPLAY_VERSION) {
    fail("version", `must be ${REPLAY_VERSION}; this build reads version ${REPLAY_VERSION} only`);
  }
  const game = textAt(document.game, "game");
  const gameVersion = document.gameVersion === undefined ? undefined : textAt(document.gameVersion, "gameVersion");
  const rate = numberAt(document.rate, "rate", 0, MAX_RATE, true);
  const frames = integerAt(document.frames, "frames", 0);
  const seeds = parseSeeds(document.seeds);
  const save = parseSave(document.save);
  const checkEvery = integerAt(document.checkEvery, "checkEvery", 1);
  const checkpoints = parseCheckpoints(document.checkpoints, frames, checkEvery);
  const inputs = parseInputs(document.inputs, frames);
  return {
    format: REPLAY_FORMAT,
    version: REPLAY_VERSION,
    game,
    ...(gameVersion === undefined ? {} : { gameVersion }),
    rate,
    frames,
    seeds,
    save,
    checkEvery,
    checkpoints,
    inputs,
  };
}

/**
 * Reads a replay from JSON text.
 * @throws TestingError with code "bad-replay" when the text is not JSON or does not match the format.
 */
export function parseReplayText(text: string): Replay {
  let value: unknown;
  try {
    value = JSON.parse(text);
  } catch (cause) {
    throw new TestingError("bad-replay", "the replay is not valid JSON", { cause });
  }
  return parseReplay(value);
}

/**
 * Writes a replay as JSON text, with two-space indents and a final line break. The seed names are
 * written in sorted order, and every other field in the order of the format, so the same replay always
 * gives the same file.
 */
export function serializeReplay(replay: Replay): string {
  const seeds = Object.fromEntries(Object.entries(replay.seeds).sort(([left], [right]) => (left < right ? -1 : 1)));
  const ordered = {
    format: replay.format,
    version: replay.version,
    game: replay.game,
    ...(replay.gameVersion === undefined ? {} : { gameVersion: replay.gameVersion }),
    rate: replay.rate,
    frames: replay.frames,
    seeds,
    save: replay.save,
    checkEvery: replay.checkEvery,
    checkpoints: replay.checkpoints,
    inputs: replay.inputs,
  };
  return `${JSON.stringify(ordered, null, 2)}\n`;
}

/** Throws the bad-replay error for one field. */
function fail(path: string, message: string): never {
  throw new TestingError("bad-replay", `${path} ${message}`);
}

/** The value as a plain object, or a bad-replay error. */
function objectAt(value: unknown, path: string): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) fail(path, "must be a JSON object");
  return value as Record<string, unknown>;
}

/** A non-empty text of at most the longest allowed length. */
function textAt(value: unknown, path: string): string {
  if (typeof value !== "string" || value.length === 0 || value.length > MAX_TEXT_LENGTH) {
    fail(path, `must be a text of 1 to ${MAX_TEXT_LENGTH} characters`);
  }
  return value;
}

/** A finite number within a range. `exclusiveMin` makes the minimum itself not allowed. */
function numberAt(value: unknown, path: string, min: number, max: number, exclusiveMin = false): number {
  if (typeof value !== "number" || !Number.isFinite(value)) fail(path, "must be a finite number");
  if (exclusiveMin ? value <= min : value < min) fail(path, `must be ${exclusiveMin ? "above" : "at least"} ${min}`);
  if (value > max) fail(path, `must be at most ${max}`);
  return value;
}

/** A whole number from `min` up, and up to `max` when it is given. */
function integerAt(value: unknown, path: string, min: number, max?: number): number {
  if (typeof value !== "number" || !Number.isInteger(value)) fail(path, "must be a whole number");
  if (value < min) fail(path, `must be at least ${min}`);
  if (max !== undefined && value > max) fail(path, `must be at most ${max}`);
  return value;
}

/** A boolean. */
function booleanAt(value: unknown, path: string): boolean {
  if (typeof value !== "boolean") fail(path, "must be true or false");
  return value;
}

/** The optional `skipped` flag: absent (false), or true. */
function skippedAt(value: unknown, path: string): true | undefined {
  if (value === undefined) return undefined;
  if (value !== true) fail(path, "must be true when it is written");
  return true;
}

/** The seeds object: names mapped to seeds. */
function parseSeeds(value: unknown): Record<string, ReplaySeed> {
  const seeds = objectAt(value, "seeds");
  const result: Record<string, ReplaySeed> = {};
  for (const name of Object.keys(seeds)) {
    if (name.length === 0 || name.length > MAX_TEXT_LENGTH) fail("seeds", `has a name of 1 to ${MAX_TEXT_LENGTH} characters`);
    const seed = seeds[name];
    if (typeof seed === "number") {
      if (!Number.isSafeInteger(seed) || seed < 0) fail(`seeds.${name}`, "must be a safe whole number from 0 up");
      result[name] = seed;
    } else if (typeof seed === "string" && SEED_TEXT_PATTERN.test(seed)) {
      result[name] = seed;
    } else {
      fail(`seeds.${name}`, "must be a safe whole number, or a text of 1 to 40 decimal digits");
    }
  }
  return result;
}

/** The starting save: null for a new game, else an envelope in the save file format. */
function parseSave(value: unknown): SaveEnvelope | null {
  if (value === null) return null;
  const save = objectAt(value, "save");
  if (save.format !== SAVE_FORMAT) fail("save.format", `must be "${SAVE_FORMAT}"`);
  const version = integerAt(save.version, "save.version", 1);
  const game = textAt(save.game, "save.game");
  if (typeof save.savedAt !== "string") fail("save.savedAt", "must be a text");
  if (!("data" in save)) fail("save.data", "is required");
  return { format: SAVE_FORMAT, version, game, savedAt: save.savedAt, data: save.data };
}

/** The checkpoints: ascending frames that are multiples of the interval, each with a hash. */
function parseCheckpoints(value: unknown, frames: number, checkEvery: number): ReplayCheckpoint[] {
  if (!Array.isArray(value)) fail("checkpoints", "must be an array");
  const checkpoints: ReplayCheckpoint[] = [];
  let previous = 0;
  for (let index = 0; index < value.length; index += 1) {
    const path = `checkpoints[${index}]`;
    const item = objectAt(value[index], path);
    const frame = integerAt(item.frame, `${path}.frame`, 1, frames);
    if (frame % checkEvery !== 0) fail(`${path}.frame`, `must be a multiple of checkEvery (${checkEvery})`);
    if (frame <= previous) fail(`${path}.frame`, `must be above the previous checkpoint (${previous})`);
    if (typeof item.hash !== "string" || item.hash.length === 0) fail(`${path}.hash`, "must be a text");
    checkpoints.push({ frame, hash: item.hash });
    previous = frame;
  }
  return checkpoints;
}

/** The inputs: frames from 0 up to the frame count, in order, each with a known type. */
function parseInputs(value: unknown, frames: number): ReplayInput[] {
  if (!Array.isArray(value)) fail("inputs", "must be an array");
  const inputs: ReplayInput[] = [];
  let previous = 0;
  for (let index = 0; index < value.length; index += 1) {
    const path = `inputs[${index}]`;
    const item = objectAt(value[index], path);
    const frame = integerAt(item.frame, `${path}.frame`, 0, frames);
    if (frame < previous) fail(`${path}.frame`, `must not be below ${previous}: the inputs are in frame order`);
    previous = frame;
    inputs.push({ frame, ...parseEvent(item, path) });
  }
  return inputs;
}

/** One input without its frame, checked by its type. */
function parseEvent(item: Record<string, unknown>, path: string): ReplayEvent {
  switch (item.type) {
    case "key":
      if (typeof item.code !== "string" || !KEY_PATTERN.test(item.code)) {
        fail(`${path}.code`, "must be 1 to 32 letters or digits");
      }
      return withSkipped(
        { type: "key", code: item.code, down: booleanAt(item.down, `${path}.down`) },
        skippedAt(item.skipped, `${path}.skipped`),
      );
    case "button": {
      if (typeof item.button !== "string" || !MOUSE_BUTTONS.includes(item.button as MouseButtonName)) {
        fail(`${path}.button`, "must be left, middle or right");
      }
      return withSkipped(
        {
          type: "button",
          button: item.button as MouseButtonName,
          down: booleanAt(item.down, `${path}.down`),
          x: numberAt(item.x, `${path}.x`, -Infinity, Infinity),
          y: numberAt(item.y, `${path}.y`, -Infinity, Infinity),
        },
        skippedAt(item.skipped, `${path}.skipped`),
      );
    }
    case "pointer":
      return {
        type: "pointer",
        x: numberAt(item.x, `${path}.x`, -Infinity, Infinity),
        y: numberAt(item.y, `${path}.y`, -Infinity, Infinity),
      };
    case "wheel": {
      const deltaMode = integerAt(item.deltaMode, `${path}.deltaMode`, 0, 2);
      if (!DELTA_MODES.includes(deltaMode)) fail(`${path}.deltaMode`, "must be 0, 1 or 2");
      return withSkipped(
        {
          type: "wheel",
          deltaX: numberAt(item.deltaX, `${path}.deltaX`, -Infinity, Infinity),
          deltaY: numberAt(item.deltaY, `${path}.deltaY`, -Infinity, Infinity),
          deltaMode: deltaMode as 0 | 1 | 2,
        },
        skippedAt(item.skipped, `${path}.skipped`),
      );
    }
    case "pad":
      return parsePadEvent(item, path);
    case "blur":
      return { type: "blur" };
    default:
      fail(`${path}.type`, "must be key, button, pointer, wheel, pad or blur");
  }
}

/** A pad event: a disconnection, or the whole state of a connected pad. */
function parsePadEvent(item: Record<string, unknown>, path: string): ReplayEvent {
  const pad = integerAt(item.pad, `${path}.pad`, 0, MAX_PAD_INDEX);
  if (!booleanAt(item.connected, `${path}.connected`)) return { type: "pad", pad, connected: false };
  const id = item.id;
  if (typeof id !== "string" || id.length > MAX_TEXT_LENGTH) fail(`${path}.id`, "must be a text");
  if (!Array.isArray(item.buttons) || item.buttons.length > MAX_PAD_BUTTONS) {
    fail(`${path}.buttons`, `must be an array of at most ${MAX_PAD_BUTTONS} buttons`);
  }
  if (!Array.isArray(item.axes) || item.axes.length > MAX_PAD_AXES) {
    fail(`${path}.axes`, `must be an array of at most ${MAX_PAD_AXES} axes`);
  }
  const buttons = (item.buttons as unknown[]).map((entry, button) => {
    const at = `${path}.buttons[${button}]`;
    const record = objectAt(entry, at);
    return {
      pressed: booleanAt(record.pressed, `${at}.pressed`),
      value: numberAt(record.value, `${at}.value`, 0, 1),
    };
  });
  const axes = (item.axes as unknown[]).map((entry, axis) => numberAt(entry, `${path}.axes[${axis}]`, -1, 1));
  return { type: "pad", pad, connected: true, id, buttons, axes };
}

/** Adds the `skipped` flag to an event when it is set, so that an event without it has no such field. */
function withSkipped<Event extends ReplayEvent>(event: Event, skipped: true | undefined): ReplayEvent {
  return skipped === undefined ? event : ({ ...event, skipped: true } as ReplayEvent);
}
