// Test doubles for @velvet/audio. A fake Web Audio context records the graph and every value that
// is scheduled, and it plays its sources on a clock that the tests move with `advance`. A fake fetch
// serves files by address, a fake host stands in for the page, and an in-memory store stands in for
// the saved settings. Only the tests import this file; the package index does not export it.

import type { AudioEngine, AudioHost, AudioOptions, AudioSettingsStore, DuckingOptions, FetchLike, FetchResponseLike } from "./api";
import type {
  AudioBufferLike,
  AudioBufferSourceLike,
  AudioContextLike,
  AudioNodeLike,
  AudioParamLike,
  GainNodeLike,
  StereoPannerLike,
} from "./context";
import { createAudio } from "./engine";
import { parseAudioManifest } from "./manifest";
import type { AudioManifest } from "./types";

/** The first byte of a file that the fake context refuses to decode. */
export const DECODE_FAILS = 0xff;

/** One change that was scheduled on a parameter, in the order it was made. */
export type ParamEvent =
  | { readonly kind: "set"; readonly value: number; readonly time: number }
  | { readonly kind: "ramp"; readonly value: number; readonly time: number }
  | { readonly kind: "cancel"; readonly time: number };

/** A parameter that records what is scheduled on it. */
export class FakeParam implements AudioParamLike {
  value: number;
  readonly events: ParamEvent[] = [];

  constructor(value: number) {
    this.value = value;
  }

  setValueAtTime(value: number, time: number): this {
    this.events.push({ kind: "set", value, time });
    this.value = value;
    return this;
  }

  linearRampToValueAtTime(value: number, time: number): this {
    this.events.push({ kind: "ramp", value, time });
    this.value = value;
    return this;
  }

  cancelScheduledValues(time: number): this {
    this.events.push({ kind: "cancel", time });
    return this;
  }

  /** The last ramp that was scheduled, or null when there was none. */
  lastRamp(): { value: number; time: number } | null {
    for (let index = this.events.length - 1; index >= 0; index -= 1) {
      const event = this.events[index];
      if (event?.kind === "ramp") return { value: event.value, time: event.time };
    }
    return null;
  }

  /** The last value that was set at a time, or null when nothing was set. */
  lastSet(): { value: number; time: number } | null {
    for (let index = this.events.length - 1; index >= 0; index -= 1) {
      const event = this.events[index];
      if (event?.kind === "set") return { value: event.value, time: event.time };
    }
    return null;
  }
}

/** A node that records what it is connected to. */
export class FakeNode implements AudioNodeLike {
  readonly outputs: AudioNodeLike[] = [];
  disconnected = false;

  connect(destination: AudioNodeLike): void {
    this.outputs.push(destination);
  }

  disconnect(): void {
    this.outputs.length = 0;
    this.disconnected = true;
  }
}

/** A gain node. */
export class FakeGain extends FakeNode implements GainNodeLike {
  readonly gain = new FakeParam(1);
}

/** A stereo panner. */
export class FakePanner extends FakeNode implements StereoPannerLike {
  readonly pan = new FakeParam(0);
}

/** Decoded audio of a known length. */
export class FakeBuffer implements AudioBufferLike {
  readonly duration: number;

  constructor(duration: number) {
    this.duration = duration;
  }
}

/** A source node. It records when it starts and stops, and reports its end once the clock passes it. */
export class FakeSource extends FakeNode implements AudioBufferSourceLike {
  buffer: AudioBufferLike | null = null;
  loop = false;
  loopStart = 0;
  loopEnd = 0;
  readonly playbackRate = new FakeParam(1);
  readonly detune = new FakeParam(0);
  onended: ((event: Event) => void) | null = null;
  startedAt: number | null = null;
  offset = 0;
  stopAt: number | null = null;
  reported = false;

  start(when = 0, offset = 0): void {
    if (this.startedAt !== null) throw new Error("start called twice");
    this.startedAt = when;
    this.offset = offset;
  }

  stop(when = 0): void {
    if (this.startedAt === null) throw new Error("stop called before start");
    this.stopAt = when;
  }

  /** When the source stops: the end of the buffer, or the stop time, whichever comes first. Null before it starts. */
  endTime(): number | null {
    if (this.startedAt === null) return null;
    const duration = this.buffer?.duration ?? 0;
    const natural = this.loop ? Number.POSITIVE_INFINITY : this.startedAt + (duration - this.offset) / this.playbackRate.value;
    return Math.min(natural, this.stopAt ?? Number.POSITIVE_INFINITY);
  }
}

/** A fake audio context. Its clock moves only with `advance`, and it records every node it makes. */
export class FakeAudioContext implements AudioContextLike {
  readonly destination = new FakeNode();
  currentTime = 0;
  state: "suspended" | "running" | "closed" | "interrupted" = "running";
  readonly gains: FakeGain[] = [];
  readonly sources: FakeSource[] = [];
  readonly panners: FakePanner[] = [];
  /** When false, `resume` leaves the context suspended, as a browser does without a user gesture. */
  resumeAllowed = true;
  resumeCalls = 0;
  suspendCalls = 0;
  decodeCount = 0;

  createGain(): FakeGain {
    const gain = new FakeGain();
    this.gains.push(gain);
    return gain;
  }

  createBufferSource(): FakeSource {
    const source = new FakeSource();
    this.sources.push(source);
    return source;
  }

  createStereoPanner(): FakePanner {
    const panner = new FakePanner();
    this.panners.push(panner);
    return panner;
  }

  async decodeAudioData(data: ArrayBuffer): Promise<FakeBuffer> {
    this.decodeCount += 1;
    if (new Uint8Array(data)[0] === DECODE_FAILS) throw new Error("the file is not audio");
    return new FakeBuffer(data.byteLength / 1000);
  }

  async resume(): Promise<void> {
    this.resumeCalls += 1;
    if (this.resumeAllowed && this.state !== "closed") this.state = "running";
  }

  async suspend(): Promise<void> {
    this.suspendCalls += 1;
    if (this.state === "running") this.state = "suspended";
  }

  async close(): Promise<void> {
    this.state = "closed";
  }

  /** Moves the clock on by `seconds`, and reports each source that ends on the way, in order of its end. */
  advance(seconds: number): void {
    const target = this.currentTime + seconds;
    for (;;) {
      let next: FakeSource | null = null;
      let nextTime = Number.POSITIVE_INFINITY;
      for (const source of this.sources) {
        if (source.reported) continue;
        const end = source.endTime();
        if (end !== null && end <= target && end < nextTime) {
          next = source;
          nextTime = end;
        }
      }
      if (next === null) break;
      this.currentTime = nextTime;
      next.reported = true;
      next.onended?.(new Event("ended"));
    }
    this.currentTime = target;
  }

  /** The sources that are still running, which have started and have not reported their end. */
  runningSources(): FakeSource[] {
    return this.sources.filter((source) => source.startedAt !== null && !source.reported);
  }
}

/** How a fake file answers: its size and status, or a network failure. */
export type FakeFile =
  | { readonly bytes: number; readonly status?: number; readonly decodeFails?: boolean }
  | { readonly networkError: true };

/** A fake fetch, with the addresses that were asked for. */
export interface FakeNetwork {
  /** The fetch function to give the engine. */
  readonly fetchFile: FetchLike;
  /** Every address that was requested, in order. */
  readonly requests: string[];
}

/** Builds a fetch that serves the given files by address. An address that is not listed is a 404. */
export function createFakeNetwork(files: Readonly<Record<string, FakeFile>>): FakeNetwork {
  const requests: string[] = [];
  const fetchFile: FetchLike = async (url: string): Promise<FetchResponseLike> => {
    requests.push(url);
    const file = files[url];
    if (file === undefined) return { ok: false, status: 404, arrayBuffer: async () => new ArrayBuffer(0) };
    if ("networkError" in file) throw new Error(`network down for ${url}`);
    const status = file.status ?? 200;
    const buffer = new ArrayBuffer(file.bytes);
    if (file.decodeFails === true) new Uint8Array(buffer)[0] = DECODE_FAILS;
    return { ok: status >= 200 && status < 300, status, arrayBuffer: async () => buffer };
  };
  return { fetchFile, requests };
}

/** A fake page: its hidden flag, and the listeners of its visibility and gestures. */
export class FakeHost implements AudioHost {
  hidden = false;
  private readonly visibilityListeners = new Set<() => void>();
  private readonly gestureListeners = new Set<() => void>();

  onVisibilityChange(listener: () => void): () => void {
    this.visibilityListeners.add(listener);
    return () => {
      this.visibilityListeners.delete(listener);
    };
  }

  onGesture(listener: () => void): () => void {
    this.gestureListeners.add(listener);
    return () => {
      this.gestureListeners.delete(listener);
    };
  }

  /** Hides or shows the page, and tells the listeners. */
  setHidden(hidden: boolean): void {
    this.hidden = hidden;
    for (const listener of [...this.visibilityListeners]) listener();
  }

  /** A user gesture: a press, a key or a touch. */
  gesture(): void {
    for (const listener of [...this.gestureListeners]) listener();
  }

  /** How many listeners are still attached. */
  get listenerCount(): number {
    return this.visibilityListeners.size + this.gestureListeners.size;
  }
}

/** An in-memory settings store, with the values it holds and the number of writes. */
export interface MemorySettings {
  readonly store: AudioSettingsStore;
  readonly values: Map<string, unknown>;
  writes: number;
}

/** Builds a settings store in memory, with optional starting values. */
export function createMemorySettings(initial: Readonly<Record<string, unknown>> = {}): MemorySettings {
  const values = new Map<string, unknown>(Object.entries(initial));
  const memory: MemorySettings = {
    values,
    writes: 0,
    store: {
      get: async (key: string) => values.get(key) ?? null,
      set: async (key: string, value: unknown) => {
        memory.writes += 1;
        values.set(key, value);
      },
    },
  };
  return memory;
}

/** Parses a manifest from sounds written as plain objects, as a game would write them in JSON. */
export function manifestOf(sounds: readonly unknown[], buses: readonly string[] = []): AudioManifest {
  return parseAudioManifest(JSON.stringify({ format: "velvet-audio", version: 1, buses, sounds }));
}

/** A file of the given size for every candidate of every variant in a manifest, under the base address. */
export function filesFor(manifest: AudioManifest, base = "audio/"): Record<string, FakeFile> {
  const files: Record<string, FakeFile> = {};
  for (const sound of manifest.sounds) {
    for (const variant of sound.variants) {
      for (const path of variant.files) files[base + path] = { bytes: 1000 };
    }
  }
  return files;
}

/** Everything a test needs to drive the engine. */
export interface Rig {
  /** The engine under test. */
  readonly audio: AudioEngine;
  /** The fake context the engine made (it is made on first use). */
  readonly context: FakeAudioContext;
  /** The fake network, with the requests that were made. */
  readonly network: FakeNetwork;
  /** The fake page. */
  readonly host: FakeHost;
  /** The game clock: set `paused` to pause it. */
  readonly clock: { paused: boolean };
  /** The clock of cooldowns, in milliseconds. Move it with `ms`. */
  readonly time: { ms: number };
}

/** The options of {@link createRig}. Only `sounds` is required. */
export interface RigOptions {
  /** The sounds, as plain objects. */
  readonly sounds: readonly unknown[];
  /** Custom buses of the manifest. */
  readonly buses?: readonly string[];
  /** The files that exist. Defaults to a 1-second file for every candidate. */
  readonly files?: Readonly<Record<string, FakeFile>>;
  /** Whether the fake context starts running. Defaults to true. */
  readonly running?: boolean;
  /** The pause policy. */
  readonly pauseWith?: "game-buses" | "context";
  /** The settings store. Defaults to none. */
  readonly settings?: AudioSettingsStore | null;
  /** The ducking. Defaults to the engine's defaults. */
  readonly ducking?: DuckingOptions | false;
  /** The random source for the variants. */
  readonly random?: () => number;
  /** The queue limit. */
  readonly queueLimit?: number;
  /** The stale time of waiting plays, in milliseconds. */
  readonly staleAfterMs?: number;
  /** Whether a hidden page suspends the context. */
  readonly suspendWhenHidden?: boolean;
  /** A context factory, to test a runtime with no audio. Defaults to the fake context. */
  readonly createContext?: AudioOptions["createContext"];
  /** Which formats the runtime can play. Defaults to all of them. */
  readonly canPlay?: AudioOptions["canPlay"];
}

/** Builds an engine on a fake context, a fake network and a fake page. */
export function createRig(options: RigOptions): Rig {
  const manifest = manifestOf(options.sounds, options.buses ?? []);
  const context = new FakeAudioContext();
  context.state = options.running === false ? "suspended" : "running";
  const network = createFakeNetwork(options.files ?? filesFor(manifest));
  const host = new FakeHost();
  const clock = { paused: false };
  const time = { ms: 0 };
  const audio = createAudio({
    manifest,
    baseUrl: "audio/",
    fetch: network.fetchFile,
    createContext: options.createContext ?? (() => context),
    canPlay: options.canPlay ?? (() => true),
    host,
    clock,
    pauseWith: options.pauseWith,
    settings: options.settings === undefined ? null : options.settings,
    ducking: options.ducking,
    now: () => time.ms,
    random: options.random,
    queueLimit: options.queueLimit,
    staleAfterMs: options.staleAfterMs,
    suspendWhenHidden: options.suspendWhenHidden,
  });
  return { audio, context, network, host, clock, time };
}

/** The gain node that a graph node is, or a failure when it is something else. */
export function asGain(node: AudioNodeLike | undefined): FakeGain {
  if (node instanceof FakeGain) return node;
  throw new Error("expected a gain node in the graph");
}

/** The nodes that a played source is wired through, from the source to the master. */
export interface Chain {
  /** The source itself. */
  readonly source: FakeSource;
  /** The gain of the play: its volume. */
  readonly gain: FakeGain;
  /** The bus input: the bus volume, mute and pause gate. */
  readonly input: FakeGain;
  /** The bus duck: the ducking of the bus. */
  readonly duck: FakeGain;
  /** The master gain, which feeds the destination. */
  readonly master: FakeGain;
}

/** Walks the graph from a source to the master. Panners are passed through. */
export function chainOf(source: FakeSource): Chain {
  const gain = asGain(source.outputs[0]);
  let next: AudioNodeLike | undefined = gain.outputs[0];
  if (next instanceof FakePanner) next = next.outputs[0];
  const input = asGain(next);
  const duck = asGain(input.outputs[0]);
  const master = asGain(duck.outputs[0]);
  return { source, gain, input, duck, master };
}

/** Lets every pending promise settle: a macrotask, so all microtasks have run. */
export function settle(): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, 0));
}

/**
 * Runs `action` and returns the error that it throws. Fails the test when nothing is thrown.
 * @param action - the call to make.
 */
export function thrownBy(action: () => unknown): unknown {
  try {
    action();
  } catch (error) {
    return error;
  }
  throw new Error("expected the call to throw, but it returned");
}
