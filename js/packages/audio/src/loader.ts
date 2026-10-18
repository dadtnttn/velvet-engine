// Loading and decoding: the files of a sound's variant, tried in their order. Each variant is decoded
// once and kept in a cache. A second request for a variant that is still loading waits for the first
// one, so one file is fetched once. Unloading a sound drops its cache, and a load that was still
// running then does not put its result back.

import type { FetchLike } from "./api";
import type { AudioBufferLike, AudioContextLike } from "./context";
import { describeError } from "./error";
import { AUDIO_MIME, formatOfPath, type AudioFormat, type AudioSound } from "./types";

/** What the loader needs from the engine. */
export interface LoaderOptions {
  /** The address of the audio folder, with a trailing slash. */
  readonly baseUrl: string;
  /** Fetches a file. */
  readonly fetch: FetchLike;
  /** Whether a format can be played. */
  readonly canPlay: (format: AudioFormat) => boolean;
  /** The audio context, or null when there is none yet or at all. */
  readonly context: () => AudioContextLike | null;
  /** Called once each time a variant is decoded. */
  readonly onLoaded: (id: string, variant: number) => void;
  /** Called once each time a variant fails, with the reason. */
  readonly onFailed: (id: string, message: string) => void;
}

/** The key of one variant of one sound in the caches. Sound ids never hold a `#`. */
function keyOf(id: string, variant: number): string {
  return `${id}#${variant}`;
}

/** Caches the decoded variants of sounds, and shares the loads that are running. */
export class Loader {
  private readonly options: LoaderOptions;
  private readonly buffers = new Map<string, AudioBufferLike>();
  private readonly pending = new Map<string, Promise<AudioBufferLike>>();

  /** @param options - what the loader needs from the engine. */
  constructor(options: LoaderOptions) {
    this.options = options;
  }

  /** The decoded buffer of a variant, or null when it is not in the cache. */
  cached(sound: AudioSound, variant: number): AudioBufferLike | null {
    return this.buffers.get(keyOf(sound.id, variant)) ?? null;
  }

  /** True when every variant of the sound is in the cache. */
  isLoaded(sound: AudioSound): boolean {
    return sound.variants.every((_, index) => this.buffers.has(keyOf(sound.id, index)));
  }

  /**
   * Decodes one variant of a sound, or returns it from the cache. A load that is already running for
   * the same variant is shared.
   * @returns the decoded buffer. Rejects when no candidate of the variant can be loaded.
   */
  load(sound: AudioSound, variant: number): Promise<AudioBufferLike> {
    const key = keyOf(sound.id, variant);
    const ready = this.buffers.get(key);
    if (ready !== undefined) return Promise.resolve(ready);
    const running = this.pending.get(key);
    if (running !== undefined) return running;

    const candidates = sound.variants[variant]?.files ?? [];
    const attempt: Promise<AudioBufferLike> = this.decodeFirst(sound.id, candidates).then(
      (buffer) => {
        // A result is kept only when the load was not dropped by an unload in the meantime.
        if (this.pending.get(key) === attempt) {
          this.pending.delete(key);
          this.buffers.set(key, buffer);
        }
        this.options.onLoaded(sound.id, variant);
        return buffer;
      },
      (error: unknown) => {
        if (this.pending.get(key) === attempt) this.pending.delete(key);
        this.options.onFailed(sound.id, describeError(error));
        throw error;
      },
    );
    this.pending.set(key, attempt);
    return attempt;
  }

  /** Drops the cached and the loading variants of a sound. A load that is running is not kept. */
  unload(id: string): void {
    const prefix = `${id}#`;
    for (const key of [...this.buffers.keys()]) {
      if (key.startsWith(prefix)) this.buffers.delete(key);
    }
    for (const key of [...this.pending.keys()]) {
      if (key.startsWith(prefix)) this.pending.delete(key);
    }
  }

  /**
   * Fetches and decodes the first candidate that can be played and read. A candidate is skipped when
   * its format cannot be played, and when it cannot be fetched or decoded.
   * @throws Error, with the reason of each candidate, when none is left.
   */
  private async decodeFirst(id: string, candidates: readonly string[]): Promise<AudioBufferLike> {
    const context = this.options.context();
    if (context === null) throw new Error("audio is not available in this runtime");
    const failures: string[] = [];
    for (const path of candidates) {
      const format = formatOfPath(path);
      if (format === null) {
        failures.push(`${path}: not an audio file name`);
        continue;
      }
      if (!this.options.canPlay(format)) {
        failures.push(`${path}: the runtime cannot play ${AUDIO_MIME[format]}`);
        continue;
      }
      try {
        const response = await this.options.fetch(this.options.baseUrl + path);
        if (!response.ok) {
          failures.push(`${path}: HTTP ${response.status}`);
          continue;
        }
        const bytes = await response.arrayBuffer();
        return await context.decodeAudioData(bytes);
      } catch (error) {
        failures.push(`${path}: ${describeError(error)}`);
      }
    }
    throw new Error(`sound "${id}" could not be loaded: ${failures.join("; ") || "no candidate files"}`);
  }
}
