// One play of a sound: the source node, its gain, and its end. A playback is pending until the engine
// starts it, playing while the source runs, and ended once it finishes, is stopped, or is stolen. A
// playback that has ended never starts again, and the reason is kept in its `ended` promise.

import type { EndReason, PlaybackHandle, PlaybackStatus } from "./api";
import type { AudioBufferLike, AudioBufferSourceLike, AudioContextLike, AudioNodeLike, GainNodeLike, StereoPannerLike } from "./context";
import { checkNonNegative, checkRange } from "./checks";
import { rampTo } from "./params";
import type { AudioLoopPoints } from "./types";
import { volumeToGain } from "./volume";

/** What a playback is when it is made, before it starts. */
export interface PlaybackInit {
  /** The sound id. */
  readonly id: string;
  /** The bus the sound plays on. */
  readonly bus: string;
  /** The sound's own volume, from 0 to 1, from the manifest. */
  readonly baseVolume: number;
  /** The volume of this play, from 0 to 1, from the play options. */
  readonly level: number;
  /** True when the play is a track on the music channel. */
  readonly music: boolean;
}

/** Everything a playback needs to start. The engine works these out from the request. */
export interface StartParams {
  /** The context to build the nodes in. */
  readonly context: AudioContextLike;
  /** The input gain of the bus, which the nodes end in. */
  readonly input: GainNodeLike;
  /** The decoded file. */
  readonly buffer: AudioBufferLike;
  /** Whether the sound repeats. */
  readonly loop: boolean;
  /** The part that repeats, or null for the whole file. */
  readonly loopPoints: AudioLoopPoints | null;
  /** The speed of playback. */
  readonly rate: number;
  /** The pitch shift, in cents. */
  readonly detune: number;
  /** The pan, from -1 to 1, or null for no panner. */
  readonly pan: number | null;
  /** Seconds to wait before the sound starts. */
  readonly delaySeconds: number;
  /** Seconds into the file where it starts. */
  readonly offsetSeconds: number;
  /** Seconds over which the sound fades in from silence. Zero for no fade. */
  readonly fadeInSeconds: number;
  /** The order in which the play started, so that the oldest copy can be found. */
  readonly order: number;
}

/** Keeps a value between two limits. */
function clamp(value: number, low: number, high: number): number {
  return Math.min(high, Math.max(low, value));
}

/** One play of a sound, as the game holds it. Implements the handle that `play` returns. */
export class Playback implements PlaybackHandle {
  readonly id: string;
  readonly bus: string;
  /** True for a track on the music channel. */
  readonly music: boolean;
  /** Resolves with the reason when the play ends. */
  readonly ended: Promise<EndReason>;
  private readonly baseVolume: number;
  private level: number;
  private state: PlaybackStatus = "pending";
  private hasStarted = false;
  private order = 0;
  private resolveEnded: (reason: EndReason) => void = () => undefined;
  private readonly onEnd: (playback: Playback) => void;
  private context: AudioContextLike | null = null;
  private source: AudioBufferSourceLike | null = null;
  private gain: GainNodeLike | null = null;
  private panner: StereoPannerLike | null = null;

  /**
   * @param init - the sound, its volumes and whether it is music.
   * @param onEnd - called once, when the play ends, with the play itself.
   */
  constructor(init: PlaybackInit, onEnd: (playback: Playback) => void) {
    this.id = init.id;
    this.bus = init.bus;
    this.music = init.music;
    this.baseVolume = init.baseVolume;
    this.level = init.level;
    this.onEnd = onEnd;
    this.ended = new Promise<EndReason>((resolve) => {
      this.resolveEnded = resolve;
    });
  }

  /** Where the play is: pending, playing or ended. */
  get status(): PlaybackStatus {
    return this.state;
  }

  /** The volume of this play, from 0 to 1, as last set. */
  get volume(): number {
    return this.level;
  }

  /** True once the play has ended. */
  get isEnded(): boolean {
    return this.state === "ended";
  }

  /** True while the source runs. */
  get isPlaying(): boolean {
    return this.state === "playing";
  }

  /** True once the play has started, even if it has ended since. */
  get started(): boolean {
    return this.hasStarted;
  }

  /** The order in which the play started. Zero before it starts. */
  get startOrder(): number {
    return this.order;
  }

  /** Stops the play, fading out over `fadeMs` milliseconds first. */
  stop(fadeMs = 0): void {
    checkNonNegative(fadeMs, "fadeMs");
    this.finish("stopped", fadeMs);
  }

  /** Changes the volume of the play over `ms` milliseconds. */
  fade(to: number, ms: number): void {
    checkRange(to, 0, 1, "volume");
    checkNonNegative(ms, "ms");
    if (this.state === "ended") return;
    this.level = to;
    const context = this.context;
    const gain = this.gain;
    if (this.state === "playing" && context !== null && gain !== null) {
      rampTo(context, gain.gain, this.gainTarget(), ms / 1000);
    }
  }

  /**
   * Builds the nodes and starts the source. Does nothing unless the play is pending.
   * @param params - the buffer, the bus and the timing.
   */
  start(params: StartParams): void {
    if (this.state !== "pending") return;
    const { context } = params;
    const source = context.createBufferSource();
    source.buffer = params.buffer;
    source.loop = params.loop;
    if (params.loop && params.loopPoints !== null) {
      const start = clamp(params.loopPoints.start, 0, params.buffer.duration);
      const end = clamp(params.loopPoints.end, 0, params.buffer.duration);
      if (end > start) {
        source.loopStart = start;
        source.loopEnd = end;
      }
    }
    source.playbackRate.value = params.rate;
    source.detune.value = params.detune;

    const gain = context.createGain();
    source.connect(gain);
    let tail: AudioNodeLike = gain;
    if (params.pan !== null && typeof context.createStereoPanner === "function") {
      const panner = context.createStereoPanner();
      panner.pan.value = params.pan;
      gain.connect(panner);
      this.panner = panner;
      tail = panner;
    }
    tail.connect(params.input);

    const when = context.currentTime + params.delaySeconds;
    const target = this.gainTarget();
    if (params.fadeInSeconds > 0) {
      gain.gain.setValueAtTime(0, when);
      gain.gain.linearRampToValueAtTime(target, when + params.fadeInSeconds);
    } else {
      gain.gain.setValueAtTime(target, when);
    }

    source.onended = () => this.onSourceEnded();
    source.start(when, clamp(params.offsetSeconds, 0, Math.max(0, params.buffer.duration)));

    this.context = context;
    this.source = source;
    this.gain = gain;
    this.hasStarted = true;
    this.order = params.order;
    this.state = "playing";
  }

  /**
   * Ends the play with a reason. A play that was playing fades out over `fadeMs` first, and the
   * source is stopped then. Calling it on a play that has ended does nothing.
   */
  finish(reason: EndReason, fadeMs = 0): void {
    if (this.state === "ended") return;
    const wasPlaying = this.state === "playing";
    this.state = "ended";
    this.resolveEnded(reason);
    if (wasPlaying && reason !== "finished") this.silence(fadeMs);
    this.onEnd(this);
  }

  /** The gain of the play now: the sound's volume times this play's volume. */
  private gainTarget(): number {
    return volumeToGain(this.baseVolume) * volumeToGain(this.level);
  }

  /** Fades the gain to zero and stops the source once the fade is over. */
  private silence(fadeMs: number): void {
    const context = this.context;
    const gain = this.gain;
    const source = this.source;
    if (context === null || gain === null || source === null) return;
    const seconds = fadeMs / 1000;
    if (seconds > 0) {
      rampTo(context, gain.gain, 0, seconds);
      source.stop(context.currentTime + seconds);
    } else {
      source.stop(context.currentTime);
    }
  }

  /** Called by the source when it has finished, or after a stop. */
  private onSourceEnded(): void {
    this.finish("finished");
    this.source?.disconnect();
    this.gain?.disconnect();
    this.panner?.disconnect();
  }
}
