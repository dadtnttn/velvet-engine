// Atlases: the frame lists that sprite sheets come with. The readers take the JSON of TexturePacker (hash or
// array form) and of Aseprite (frames, plus the frame tags that name an animation). Both give SpriteFrame
// values that a sprite draws, so a loaded sheet needs no other conversion.
//
// The readers take parsed JSON, or the JSON text. They check the shape of what they read and throw a
// TypeError or a RangeError that names the bad field, so a broken export fails at load time, not on screen.

import type { PlayDirection } from "./animation";
import type { SpriteFrame } from "./sprite";

/** The size of a sheet's image, in pixels, when the export gives it. */
export interface AtlasSize {
  readonly width: number;
  readonly height: number;
}

/** A parsed TexturePacker atlas: its frames by name. */
export class Atlas {
  /** The size of the image, when the export gives it. */
  readonly size: AtlasSize | null;
  private readonly byName: ReadonlyMap<string, SpriteFrame>;

  /**
   * @param size - the image size, or null.
   * @param frames - the frames by name. Use {@link parseTexturePacker} to make an atlas.
   */
  constructor(size: AtlasSize | null, frames: ReadonlyMap<string, SpriteFrame>) {
    this.size = size;
    this.byName = frames;
  }

  /** The number of frames. */
  get count(): number {
    return this.byName.size;
  }

  /** The names of the frames, in the order of the file. */
  get names(): string[] {
    return [...this.byName.keys()];
  }

  /** The frame with a name, or undefined when there is none. */
  get(name: string): SpriteFrame | undefined {
    return this.byName.get(name);
  }

  /** The frame with a name. @throws RangeError when there is no such frame. */
  frame(name: string): SpriteFrame {
    const frame = this.byName.get(name);
    if (frame === undefined) throw new RangeError(`the atlas has no frame named ${name}`);
    return frame;
  }
}

/** A named run of frames in an Aseprite sheet, with the way it plays. */
export interface AsepriteTag {
  /** The tag's name. */
  readonly name: string;
  /** The first and the last frame index of the tag, both inclusive. */
  readonly from: number;
  readonly to: number;
  /** The playing direction. Pass it to `createAnimation` with the frames and durations below. */
  readonly direction: PlayDirection;
  /** The frames of the tag, in their own order. */
  readonly frames: readonly SpriteFrame[];
  /** The time each frame is shown, in seconds, one entry per frame of the tag. */
  readonly durations: readonly number[];
}

/** A parsed Aseprite export: every frame, with its time in seconds, and the tags. */
export class AsepriteSheet {
  /** The size of the image, when the export gives it. */
  readonly size: AtlasSize | null;
  /** Every frame, in the order of the file. */
  readonly frames: readonly SpriteFrame[];
  /** The time of each frame in seconds. */
  readonly durations: readonly number[];
  /** The tags by name. */
  readonly tags: ReadonlyMap<string, AsepriteTag>;

  /**
   * @param size - the image size, or null.
   * @param frames - the frames, in order.
   * @param durations - the time of each frame, in seconds.
   * @param tags - the tags by name.
   */
  constructor(
    size: AtlasSize | null,
    frames: readonly SpriteFrame[],
    durations: readonly number[],
    tags: ReadonlyMap<string, AsepriteTag>,
  ) {
    this.size = size;
    this.frames = frames;
    this.durations = durations;
    this.tags = tags;
  }

  /** The tag with a name. @throws RangeError when there is no such tag. */
  tag(name: string): AsepriteTag {
    const tag = this.tags.get(name);
    if (tag === undefined) throw new RangeError(`the sheet has no tag named ${name}`);
    return tag;
  }
}

/** A JSON object: a value that is not null, not an array and not a primitive. */
type JsonObject = Record<string, unknown>;

/** The playing directions that an Aseprite tag may name. */
const DIRECTIONS: readonly PlayDirection[] = ["forward", "reverse", "pingpong", "pingpong_reverse"];

/**
 * Reads a TexturePacker atlas, in the hash form (`frames` is an object keyed by file name) or the array form
 * (`frames` is a list of entries with a `filename`). Each frame keeps its rotation and its trim.
 * @param source - the parsed JSON, or the JSON text.
 * @throws TypeError when the JSON has a wrong shape, and RangeError for a bad size or a duplicate name.
 */
export function parseTexturePacker(source: unknown): Atlas {
  const root = expectObject(parseSource(source), "atlas");
  const frames = new Map<string, SpriteFrame>();
  for (const [name, entry] of frameEntries(root.frames, "frames")) {
    if (frames.has(name)) throw new RangeError(`the atlas has two frames named ${name}`);
    frames.set(name, readFrame(name, entry, `frames.${name}`));
  }
  return new Atlas(readImageSize(root.meta, "meta"), frames);
}

/**
 * Reads an Aseprite export (JSON with `frames` and `meta.frameTags`), in the hash or the array form. Each
 * frame's `duration` is in milliseconds in the file and in seconds here.
 * @param source - the parsed JSON, or the JSON text.
 * @throws TypeError when the JSON has a wrong shape, and RangeError for a bad tag, index or duration.
 */
export function parseAseprite(source: unknown): AsepriteSheet {
  const root = expectObject(parseSource(source), "sheet");
  const frames: SpriteFrame[] = [];
  const durations: number[] = [];
  for (const [name, entry] of frameEntries(root.frames, "frames")) {
    const path = `frames.${name}`;
    frames.push(readFrame(name, entry, path));
    const milliseconds = readNumber(expectObject(entry, path), "duration", path);
    if (!isNonNegativeNumber(milliseconds)) {
      throw new RangeError(`${path}.duration must be a finite number from 0, got ${milliseconds}`);
    }
    durations.push(milliseconds / 1000);
  }

  const meta: JsonObject = root.meta === undefined ? {} : expectObject(root.meta, "meta");
  const tags = new Map<string, AsepriteTag>();
  const tagList = meta.frameTags === undefined ? [] : expectArray(meta.frameTags, "meta.frameTags");
  for (const [position, raw] of tagList.entries()) {
    const path = `meta.frameTags[${position}]`;
    const tag = expectObject(raw, path);
    const name = readString(tag, "name", path);
    const from = readInteger(tag, "from", path);
    const to = readInteger(tag, "to", path);
    const direction = readString(tag, "direction", path);
    if (from < 0 || to >= frames.length || from > to) {
      throw new RangeError(`${path} has frames ${from} to ${to}, outside the ${frames.length} frames of the sheet`);
    }
    if (!isDirection(direction)) {
      throw new TypeError(`${path}.direction must be one of ${DIRECTIONS.join(", ")}, got ${direction}`);
    }
    if (tags.has(name)) throw new RangeError(`the sheet has two tags named ${name}`);
    tags.set(name, {
      name,
      from,
      to,
      direction,
      frames: frames.slice(from, to + 1),
      durations: durations.slice(from, to + 1),
    });
  }
  return new AsepriteSheet(readImageSize(root.meta, "meta"), frames, durations, tags);
}

/** Parses JSON text. A value that is not text is returned as it is. */
function parseSource(source: unknown): unknown {
  return typeof source === "string" ? JSON.parse(source) : source;
}

/** The entries of a `frames` value: `[name, entry]` pairs, from the hash form or the array form. */
function frameEntries(value: unknown, path: string): [string, unknown][] {
  if (Array.isArray(value)) {
    return value.map((entry, position): [string, unknown] => {
      const object = expectObject(entry, `${path}[${position}]`);
      return [readString(object, "filename", `${path}[${position}]`), entry];
    });
  }
  if (isObject(value)) return Object.entries(value);
  throw new TypeError(`${path} must be an object or a list`);
}

/** Reads one frame entry: its stored rectangle, its rotation, and its trim. */
function readFrame(name: string, entry: unknown, path: string): SpriteFrame {
  const object = expectObject(entry, path);
  const rect = expectObject(object.frame, `${path}.frame`);
  const width = readSize(rect, "w", `${path}.frame`);
  const height = readSize(rect, "h", `${path}.frame`);
  const trim = object.spriteSourceSize === undefined ? null : expectObject(object.spriteSourceSize, `${path}.spriteSourceSize`);
  const source = object.sourceSize === undefined ? null : expectObject(object.sourceSize, `${path}.sourceSize`);
  return {
    name,
    x: readNumber(rect, "x", `${path}.frame`),
    y: readNumber(rect, "y", `${path}.frame`),
    width,
    height,
    rotated: object.rotated === true,
    trimX: trim === null ? 0 : readNumber(trim, "x", `${path}.spriteSourceSize`),
    trimY: trim === null ? 0 : readNumber(trim, "y", `${path}.spriteSourceSize`),
    sourceWidth: source === null ? undefined : readSize(source, "w", `${path}.sourceSize`),
    sourceHeight: source === null ? undefined : readSize(source, "h", `${path}.sourceSize`),
  };
}

/** The image size of a `meta` object, or null when it has none. */
function readImageSize(meta: unknown, path: string): AtlasSize | null {
  if (!isObject(meta) || meta.size === undefined) return null;
  const size = expectObject(meta.size, `${path}.size`);
  return { width: readSize(size, "w", `${path}.size`), height: readSize(size, "h", `${path}.size`) };
}

/** Reads a field that must be a finite number. */
function readNumber(object: JsonObject, key: string, path: string): number {
  const value = object[key];
  if (typeof value !== "number" || !Number.isFinite(value)) {
    throw new TypeError(`${path}.${key} must be a finite number`);
  }
  return value;
}

/** Reads a size field: a finite number of zero or more. */
function readSize(object: JsonObject, key: string, path: string): number {
  const value = readNumber(object, key, path);
  if (!isNonNegativeNumber(value)) throw new RangeError(`${path}.${key} must be zero or more, got ${value}`);
  return value;
}

/** Reads a field that must be a whole number. */
function readInteger(object: JsonObject, key: string, path: string): number {
  const value = readNumber(object, key, path);
  if (!Number.isInteger(value)) throw new TypeError(`${path}.${key} must be a whole number`);
  return value;
}

/** Reads a field that must be a string. */
function readString(object: JsonObject, key: string, path: string): string {
  const value = object[key];
  if (typeof value !== "string") throw new TypeError(`${path}.${key} must be a string`);
  return value;
}

/** Checks that a value is a JSON object, and returns it typed as one. */
function expectObject(value: unknown, path: string): JsonObject {
  if (!isObject(value)) throw new TypeError(`${path} must be an object`);
  return value;
}

/** Checks that a value is a JSON array, and returns it typed as one. */
function expectArray(value: unknown, path: string): unknown[] {
  if (!Array.isArray(value)) throw new TypeError(`${path} must be a list`);
  return value;
}

/** True for a JSON object, not an array and not null. */
function isObject(value: unknown): value is JsonObject {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

/** True for a finite number of zero or more. */
function isNonNegativeNumber(value: number): boolean {
  return Number.isFinite(value) && value >= 0;
}

/** True when a value is one of the Aseprite playing directions. */
function isDirection(value: string): value is PlayDirection {
  return (DIRECTIONS as readonly string[]).includes(value);
}
