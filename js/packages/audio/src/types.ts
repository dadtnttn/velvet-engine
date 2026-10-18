// The shared vocabulary of velvet-audio: the format tag and version, the built-in buses, the file
// formats and their browser types, and the shape of a parsed manifest. The rules themselves are in
// docs/formats/AUDIO.md.

/** The format tag that every audio manifest carries. */
export const AUDIO_FORMAT = "velvet-audio";

/** The format version this package reads. Any other version is refused. */
export const AUDIO_VERSION = 1 as const;

/** The buses that every manifest can use, in the order the format lists them. */
export const BUILT_IN_BUSES = ["music", "sfx", "voice", "ui"] as const;

/** One of the built-in bus names. */
export type BuiltInBus = (typeof BUILT_IN_BUSES)[number];

/** The reserved name of the master bus, which every sound's output passes through. */
export const MASTER_BUS = "master";

/** The bus of a sound that does not name one. */
export const DEFAULT_BUS: BuiltInBus = "sfx";

/** The file formats of sounds, in the order of the format's extension list. */
export const AUDIO_FORMATS = ["opus", "ogg", "mp3", "wav"] as const;

/** One file format of sounds. */
export type AudioFormat = (typeof AUDIO_FORMATS)[number];

/** The MIME type that a browser is asked about for each format, through `canPlayType`. */
export const AUDIO_MIME: Readonly<Record<AudioFormat, string>> = {
  opus: 'audio/ogg; codecs="opus"',
  ogg: 'audio/ogg; codecs="vorbis"',
  mp3: "audio/mpeg",
  wav: 'audio/wav; codecs="1"',
};

/** The most candidate files that one list may hold. */
export const AUDIO_MAX_CANDIDATES = 8;

/** True when `name` is one of the built-in bus names. */
export function isBuiltInBus(name: string): name is BuiltInBus {
  return BUILT_IN_BUSES.some((bus) => bus === name);
}

/** The format of a file path, from its extension; null when the extension is not one of the formats. */
export function formatOfPath(path: string): AudioFormat | null {
  for (const format of AUDIO_FORMATS) {
    if (path.endsWith(`.${format}`)) return format;
  }
  return null;
}

/** The part of a file that repeats, in seconds from the start of the file. */
export interface AudioLoopPoints {
  /** Where the repeated part starts. */
  readonly start: number;
  /** Where the repeated part ends. After `start`. */
  readonly end: number;
}

/** One alternative version of a sound: its candidate files, best format first. */
export interface AudioVariant {
  /** The candidate paths of this variant, relative to the audio folder. */
  readonly files: readonly string[];
}

/** One sound as the parsed manifest holds it. Optional fields are filled with their defaults. */
export interface AudioSound {
  /** The sound id, unique in the manifest, such as `sfx.door.open`. */
  readonly id: string;
  /** The bus the sound plays on: a built-in bus or a custom one. Defaults to `sfx`. */
  readonly bus: string;
  /** The sound's own level, from 0 to 1. Defaults to 1. */
  readonly volume: number;
  /** True when the sound repeats until it is stopped. Defaults to false. */
  readonly loop: boolean;
  /** The part that repeats, or null when the whole file repeats. */
  readonly loop_points: AudioLoopPoints | null;
  /** How many copies may play at once, or null for no limit. */
  readonly max_voices: number | null;
  /** The least time between two plays, in milliseconds. Defaults to 0. */
  readonly cooldown_ms: number;
  /** True when a preload that names no ids decodes the sound. Defaults to false. */
  readonly preload: boolean;
  /** The variants of the sound. A sound without variants has exactly one. */
  readonly variants: readonly AudioVariant[];
  /** Notes for tools. The runtime never reads them. Defaults to an empty object. */
  readonly meta: Readonly<Record<string, unknown>>;
}

/** A parsed audio manifest in its normal form: defaults filled in, sounds in the order of the file. */
export interface AudioManifest {
  /** The format tag: always `velvet-audio`. */
  readonly format: typeof AUDIO_FORMAT;
  /** The format version: always 1. */
  readonly version: typeof AUDIO_VERSION;
  /** The custom bus names that the manifest declares, in the order of the file. */
  readonly buses: readonly string[];
  /** The sounds, in the order of the file. */
  readonly sounds: readonly AudioSound[];
}
