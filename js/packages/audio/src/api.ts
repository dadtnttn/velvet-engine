// The public types of @velvet/audio: the options of createAudio, the engine, the playback handles,
// the events, and the hooks for the host (the page, the context, the settings). The implementation
// is in engine.ts; these types are what a game and its tests see.

import type { GameClock, Listener, Unsubscribe } from "@velvet/core";
import type { AudioContextLike } from "./context";
import type { AudioFormat, AudioManifest } from "./types";

/** Where a playback is: waiting to start, playing, or ended. */
export type PlaybackStatus = "pending" | "playing" | "ended";

/**
 * Why a playback ended:
 * - "finished": it played to the end (a looped sound never does, until it is stopped);
 * - "stopped": `stop` was called, or the music or every sound was stopped;
 * - "stolen": a newer copy took its place, because the sound has a voice limit;
 * - "skipped": a cooldown refused the play, or it waited too long for the audio to start;
 * - "failed": the file could not be loaded, or there is no audio in this runtime.
 */
export type EndReason = "finished" | "stopped" | "stolen" | "skipped" | "failed";

/** One play of a sound. Returned by `play` and `playMusic`, so the game can stop it or change it. */
export interface PlaybackHandle {
  /** The sound id. */
  readonly id: string;
  /** The bus the sound plays on. */
  readonly bus: string;
  /** Where the play is: "pending" until it starts, "playing", then "ended". */
  readonly status: PlaybackStatus;
  /** The volume of this play, from 0 to 1, as last set. Not the sound's own volume. */
  readonly volume: number;
  /** Resolves with the reason when the play ends. A loop only ends when it is stopped. */
  readonly ended: Promise<EndReason>;
  /**
   * Stops the play. With `fadeMs`, the sound fades out over that many milliseconds first. Stopping
   * a play that has ended does nothing.
   * @throws RangeError when `fadeMs` is negative or not finite.
   */
  stop(fadeMs?: number): void;
  /**
   * Changes the volume of this play to `to`, over `ms` milliseconds.
   * @throws RangeError when `to` is not from 0 to 1, or `ms` is negative.
   */
  fade(to: number, ms: number): void;
}

/** Options of `play`. Every field is optional. */
export interface PlayOptions {
  /** The volume of this play, from 0 to 1, on the dB curve. It multiplies the sound's own volume. Default 1. */
  readonly volume?: number;
  /** The speed of playback: 1 is normal, 2 is an octave up. Above zero. Default 1. */
  readonly rate?: number;
  /** The pitch shift in cents. Default 0. */
  readonly detune?: number;
  /** The position between the speakers: -1 is full left, 1 full right. Default: no panning. */
  readonly pan?: number;
  /** Whether this play repeats. Default: the sound's own `loop`. */
  readonly loop?: boolean;
  /** Milliseconds to wait before the sound starts. Default 0. */
  readonly delay?: number;
}

/** Options of `playMusic`. Every field is optional. */
export interface MusicOptions {
  /** Milliseconds over which the new track fades in while the old one fades out. Default 0. */
  readonly crossfadeMs?: number;
  /** Seconds into the track where it starts, clamped to the length of the track. Default 0. */
  readonly startAt?: number;
}

/**
 * How music ducks other sounds: while a sound of the `trigger` bus plays, the `target` bus is turned
 * down by `decibels`, and it comes back up after the last trigger ends. Every field is optional.
 */
export interface DuckingOptions {
  /** The bus whose sounds cause the ducking. Default "voice". */
  readonly trigger?: string;
  /** The bus that is turned down. Default "music". */
  readonly target?: string;
  /** How far the target goes down, in decibels. Zero or below. Default -12. */
  readonly decibels?: number;
  /** Milliseconds for the target to go down. Default 60. */
  readonly attackMs?: number;
  /** Milliseconds for the target to come back up. Default 400. */
  readonly releaseMs?: number;
}

/** Events of the engine, through the core event system: `on`, `once`, `off`. */
export interface AudioEvents {
  /** A sound's file was decoded and kept. `variant` is the index of the variant that was loaded. */
  loaded: { id: string; variant: number };
  /** A sound could not be loaded from any of its candidates. `message` says why. */
  error: { id: string; message: string };
  /** The music changed: a track started (`id`) or stopped (`id` null). `previous` is the track before. */
  "music-changed": { id: string | null; previous: string | null };
}

/** Progress of a preload: how many of the sounds are done. Sent after each sound settles. */
export interface PreloadProgress {
  /** The sounds settled so far, this one included. */
  readonly done: number;
  /** The distinct sounds of the call. */
  readonly total: number;
  /** The id of the sound that just settled. */
  readonly id: string;
}

/** The outcome of a preload. */
export interface PreloadResult {
  /** The ids of the sounds that could not be loaded, sorted. Each one is also reported as an `error` event. */
  readonly failed: readonly string[];
}

/** What the engine needs from the page: visibility, and the user gestures that allow audio to start. */
export interface AudioHost {
  /** True while the page is hidden. */
  readonly hidden: boolean;
  /** Calls `listener` when the page becomes hidden or visible. Returns a function that stops the calls. */
  onVisibilityChange(listener: () => void): () => void;
  /** Calls `listener` on a press, a key or a touch, the gestures that let a browser start audio. Returns a function that stops the calls. */
  onGesture(listener: () => void): () => void;
}

/**
 * Where the settings are kept: the volumes and mutes. It has the same shape as the bridge's
 * `settings` object, so `getBridge()?.settings` can be passed as it is.
 */
export interface AudioSettingsStore {
  /** Reads the value of a key. A missing key gives null. */
  get(key: string): Promise<unknown>;
  /** Writes the value of a key. */
  set(key: string, value: unknown): Promise<void>;
}

/** The part of a fetch response that the engine reads. A real Response has it. */
export interface FetchResponseLike {
  /** True for a status from 200 to 299. */
  readonly ok: boolean;
  /** The HTTP status. */
  readonly status: number;
  /** The body as bytes. */
  arrayBuffer(): Promise<ArrayBuffer>;
}

/** Fetches a file by its address. The global `fetch` fits it. */
export type FetchLike = (url: string) => Promise<FetchResponseLike>;

/** Options of {@link createAudio}. Only `manifest` is required. */
export interface AudioOptions {
  /** The parsed manifest, from `parseAudioManifest`. */
  readonly manifest: AudioManifest;
  /** The address that the audio folder is served from. Defaults to `audio/`. */
  readonly baseUrl?: string;
  /** Fetches the files. Defaults to the global `fetch`. */
  readonly fetch?: FetchLike;
  /**
   * Makes the audio context, when it is first needed. Returns null when there is no audio in this
   * runtime. Defaults to `new AudioContext()`, or null without one.
   */
  readonly createContext?: () => AudioContextLike | null;
  /**
   * Whether the runtime can play a format. Defaults to asking an HTMLAudioElement. Without one, every
   * format counts as playable, and a file that fails to decode moves on to the next candidate.
   */
  readonly canPlay?: (format: AudioFormat) => boolean;
  /** The game clock. While it is paused the game buses go quiet, see `pauseWith`. Read by `update`. */
  readonly clock?: Pick<GameClock, "paused">;
  /**
   * What a paused clock does. "game-buses" (the default) mutes music, sfx and voice and leaves `ui`
   * sounding. "context" suspends the whole audio context, `ui` included.
   */
  readonly pauseWith?: "game-buses" | "context";
  /** Whether the page is hidden, and the gestures. Defaults to the browser's document and window. */
  readonly host?: AudioHost;
  /**
   * Where the volumes and mutes are kept. Defaults to the bridge's settings in Velvet Desktop, then
   * localStorage in a browser, and nowhere else. Pass null to keep them for this session only.
   */
  readonly settings?: AudioSettingsStore | null;
  /** The ducking of music under voice. Defaults to {@link DuckingOptions} defaults. Pass false to turn it off. */
  readonly ducking?: DuckingOptions | false;
  /** Milliseconds on a clock that never goes back, for cooldowns. Defaults to `performance.now`. */
  readonly now?: () => number;
  /** A random number from 0 up to, not including, 1, for the variants. Defaults to `Math.random`. */
  readonly random?: () => number;
  /** How many plays may wait for the audio to start. The oldest is dropped beyond it. Default 32. */
  readonly queueLimit?: number;
  /** Milliseconds a waiting play stays worth starting. Older ones are skipped. Default 3000. */
  readonly staleAfterMs?: number;
  /** Whether a hidden page suspends the audio context. Default true. */
  readonly suspendWhenHidden?: boolean;
}

/**
 * The audio engine: plays sounds by id, runs the music channel, and keeps the mix. Created by
 * {@link createAudio}. Call `update()` once per frame, next to the game clock's `advance`.
 */
export interface AudioEngine {
  /** Resolves when the saved settings have been read and applied. Never rejects. */
  readonly ready: Promise<void>;
  /** True once the browser has let the audio context run. */
  readonly unlocked: boolean;
  /** The id of the music that is playing or waiting to play, or null. */
  readonly currentMusic: string | null;
  /** Adds a listener for an event. Returns a function that removes it. */
  on<Type extends keyof AudioEvents>(type: Type, listener: Listener<AudioEvents[Type]>): Unsubscribe;
  /** Adds a listener that runs once, at the next event of that type. */
  once<Type extends keyof AudioEvents>(type: Type, listener: Listener<AudioEvents[Type]>): Unsubscribe;
  /** Removes a listener. */
  off<Type extends keyof AudioEvents>(type: Type, listener: Listener<AudioEvents[Type]>): void;
  /**
   * Decodes the files of some sounds, so that they play at once. With no ids, the sounds whose
   * manifest says `preload: true`. Repeats are decoded once.
   * @throws AudioError with code "unknown-id" for an id that the manifest does not have.
   */
  preload(ids?: readonly string[], onProgress?: (progress: PreloadProgress) => void): Promise<PreloadResult>;
  /** True when every variant of the sound is decoded. False for an unknown id. */
  isLoaded(id: string): boolean;
  /**
   * Drops the decoded files of some sounds, all of them when no ids are given. A play that is
   * already running keeps its audio.
   * @throws AudioError with code "unknown-id" for an id that the manifest does not have.
   */
  unload(ids?: readonly string[]): void;
  /**
   * Plays a sound. It starts at once when its file is decoded and the audio runs; otherwise it waits.
   * A play refused by a cooldown is returned already ended with the reason "skipped".
   * @throws AudioError with code "unknown-id", or RangeError for an option out of range.
   */
  play(id: string, options?: PlayOptions): PlaybackHandle;
  /**
   * Starts a track on the music channel. The sound must be on the `music` bus. The old track fades
   * out over `crossfadeMs` while the new one fades in. Asking for the track that already plays
   * changes nothing and returns its handle.
   * @throws AudioError with code "unknown-id" or "wrong-bus", or RangeError for an option out of range.
   */
  playMusic(id: string, options?: MusicOptions): PlaybackHandle;
  /** Stops the music, fading out over `fadeMs` milliseconds. Does nothing when no music plays. */
  stopMusic(fadeMs?: number): void;
  /** Stops every play, music included, fading out over `fadeMs` milliseconds. */
  stopAll(fadeMs?: number): void;
  /**
   * Sets the volume of a bus, or of "master", from 0 to 1, on the dB curve.
   * @throws AudioError with code "unknown-bus", or RangeError when the volume is out of range.
   */
  setVolume(bus: string, volume: number): void;
  /** The volume of a bus or of "master". @throws AudioError with code "unknown-bus". */
  getVolume(bus: string): number;
  /** Mutes or unmutes a bus, or "master". @throws AudioError with code "unknown-bus". */
  setMuted(bus: string, muted: boolean): void;
  /** True while a bus, or "master", is muted. @throws AudioError with code "unknown-bus". */
  isMuted(bus: string): boolean;
  /**
   * Asks the browser to start the audio. Call it from a user gesture. Resolves true when the audio
   * runs. The engine also does this on the first press, key or touch, when it is not yet unlocked.
   */
  unlock(): Promise<boolean>;
  /**
   * Follows the game clock's pause, the page's visibility and the start of the audio, and starts
   * the plays that were waiting. Call it once per frame. It is cheap when nothing has changed.
   */
  update(): void;
  /** Stops every play, releases the page's listeners and closes the audio context. */
  close(): Promise<void>;
}
