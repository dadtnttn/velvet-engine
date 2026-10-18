// The audio engine: plays sounds by id, runs the music channel and keeps the mix. It also holds the
// plays that wait for the browser to let audio run, follows the game clock's pause and the page's
// visibility, and saves the mix. The clock's pause arrives as its events, and the check of the audio
// runs before each frame through the loop's hook, when the options give a loop; `update` runs the same
// check by hand. createAudio is the only entry point; the class is not exported.

import { EventEmitter, type Listener, type SettingsStore, type Unsubscribe } from "@velvet/core";
import type {
  AudioEngine,
  AudioEvents,
  AudioHost,
  AudioOptions,
  DuckingOptions,
  FetchLike,
  MusicOptions,
  PlaybackHandle,
  PlayOptions,
  PreloadProgress,
  PreloadResult,
} from "./api";
import { checkFinite, checkNonNegative, checkPositive, checkRange } from "./checks";
import type { AudioBufferLike, AudioContextLike } from "./context";
import { AudioError, describeError } from "./error";
import { QUIET_HOST, browserHost } from "./host";
import { Loader } from "./loader";
import { Mixer, type DuckingSettings } from "./mixer";
import { Playback } from "./playback";
import { createMixStore, readStoredMix, type MixValues, type StoredMix } from "./settings";
import { AUDIO_MIME, BUILT_IN_BUSES, type AudioFormat, type AudioSound } from "./types";

/** The bus of music. */
const MUSIC_BUS = "music";

/** The folder that files are served from when no base URL is given. */
const DEFAULT_BASE_URL = "audio/";

/** How many plays may wait for the audio to start, by default. */
const DEFAULT_QUEUE_LIMIT = 32;

/** How long a waiting play stays worth starting, by default, in milliseconds. */
const DEFAULT_STALE_MS = 3000;

/** The fade of a stolen copy, so that cutting it short does not click. */
const STEAL_FADE_MS = 8;

/** The ducking that applies when the options do not give one. */
const DEFAULT_DUCKING: DuckingSettings = {
  trigger: "voice",
  target: MUSIC_BUS,
  decibels: -12,
  attackMs: 60,
  releaseMs: 400,
};

/** The play options after their defaults are filled in and their ranges are checked. */
interface ResolvedPlay {
  readonly volume: number;
  readonly rate: number;
  readonly detune: number;
  readonly pan: number | null;
  readonly loop: boolean | null;
  readonly delayMs: number;
}

/** The timing of a play that is a track on the music channel, or a plain play. */
interface Timing {
  readonly music: boolean;
  readonly startAt: number;
  readonly fadeInMs: number;
}

/** One request to play a sound, from the moment it is made until it starts or ends. */
interface PlayRequest {
  readonly playback: Playback;
  readonly sound: AudioSound;
  readonly variant: number;
  readonly play: ResolvedPlay;
  readonly startAt: number;
  readonly fadeInMs: number;
  readonly queuedAt: number;
}

/** The context factory when none is given: the browser's AudioContext, or null without one. */
function defaultContext(): AudioContextLike | null {
  if (typeof AudioContext !== "function") return null;
  return new AudioContext();
}

/** Whether the browser can play a format, asked once per format through an audio element. */
const probes = new Map<AudioFormat, boolean>();
function probeCanPlay(format: AudioFormat): boolean {
  const known = probes.get(format);
  if (known !== undefined) return known;
  const answer = typeof Audio === "function" ? new Audio().canPlayType(AUDIO_MIME[format]) !== "" : true;
  probes.set(format, answer);
  return answer;
}

/** The fetch used when none is given: the global one. */
const defaultFetch: FetchLike = (url) => fetch(url);

/** The clock used when none is given: milliseconds from `performance.now`. */
function defaultNow(): number {
  return performance.now();
}

/** Adds a trailing slash to a base URL, unless it is empty or already ends with one. */
function withSlash(url: string): string {
  return url === "" || url.endsWith("/") ? url : `${url}/`;
}

/** Fills in the ducking defaults, and checks the names and numbers that the caller gave. */
function resolveDucking(option: DuckingOptions | false | undefined, busNames: readonly string[]): DuckingSettings | null {
  if (option === false) return null;
  const settings: DuckingSettings = {
    trigger: option?.trigger ?? DEFAULT_DUCKING.trigger,
    target: option?.target ?? DEFAULT_DUCKING.target,
    decibels: option?.decibels ?? DEFAULT_DUCKING.decibels,
    attackMs: option?.attackMs ?? DEFAULT_DUCKING.attackMs,
    releaseMs: option?.releaseMs ?? DEFAULT_DUCKING.releaseMs,
  };
  for (const name of [settings.trigger, settings.target]) {
    if (!busNames.includes(name)) {
      throw new AudioError("unknown-bus", `ducking names the bus "${name}", which the manifest does not have`);
    }
  }
  if (settings.trigger === settings.target) throw new RangeError("ducking needs two different buses");
  checkFinite(settings.decibels, "ducking decibels");
  if (settings.decibels > 0) throw new RangeError(`ducking decibels must be zero or below, got ${settings.decibels}`);
  checkNonNegative(settings.attackMs, "ducking attackMs");
  checkNonNegative(settings.releaseMs, "ducking releaseMs");
  return settings;
}

/** Checks the play options and fills in their defaults. */
function resolvePlay(options: PlayOptions): ResolvedPlay {
  const volume = options.volume ?? 1;
  checkRange(volume, 0, 1, "volume");
  const rate = options.rate ?? 1;
  checkPositive(rate, "rate");
  const detune = options.detune ?? 0;
  checkFinite(detune, "detune");
  const pan = options.pan ?? null;
  if (pan !== null) checkRange(pan, -1, 1, "pan");
  const delayMs = options.delay ?? 0;
  checkNonNegative(delayMs, "delay");
  return { volume, rate, detune, pan, loop: options.loop ?? null, delayMs };
}

/** The engine. Created by {@link createAudio}; the game only sees the {@link AudioEngine} interface. */
class Engine implements AudioEngine {
  readonly ready: Promise<void>;
  private readonly sounds = new Map<string, AudioSound>();
  private readonly mixer: Mixer;
  private readonly loader: Loader;
  private readonly events = new EventEmitter<AudioEvents>();
  private readonly live = new Set<Playback>();
  private readonly lastStart = new Map<string, number>();
  private readonly lastVariant = new Map<string, number>();
  private readonly touched = new Set<string>();
  private readonly createContext: () => AudioContextLike | null;
  private readonly host: AudioHost;
  private readonly clock: AudioOptions["clock"];
  private readonly pauseWith: "game-buses" | "context";
  private readonly suspendWhenHidden: boolean;
  private readonly queueLimit: number;
  private readonly staleAfterMs: number;
  private readonly now: () => number;
  private readonly random: () => number;
  private readonly mix: SettingsStore<MixValues>;
  /** The subscriptions to the clock's events and the loop's hook, released by close. */
  private readonly stopFollowing: Array<() => void> = [];
  private queue: PlayRequest[] = [];
  private context: AudioContextLike | null = null;
  private contextError: string | null = null;
  private hasUnlocked = false;
  private closed = false;
  private music: Playback | null = null;
  private lastPaused = false;
  private lastHidden = false;
  private startCounter = 0;
  private releaseGesture: (() => void) | null = null;
  private releaseVisibility: (() => void) | null = null;
  private resumeInFlight = false;
  private persistQueued = false;

  /** @param options - the options of createAudio, already checked for their types. */
  constructor(options: AudioOptions) {
    const busNames = [...BUILT_IN_BUSES, ...options.manifest.buses];
    for (const sound of options.manifest.sounds) {
      // A manifest from the parser always passes this; a hand-made one is checked here.
      if (!busNames.includes(sound.bus)) {
        throw new AudioError("unknown-bus", `sound "${sound.id}" is on the bus "${sound.bus}", which the manifest does not have`);
      }
      this.sounds.set(sound.id, sound);
    }
    this.mixer = new Mixer(busNames, resolveDucking(options.ducking, busNames));
    this.createContext = options.createContext ?? defaultContext;
    this.host = options.host ?? browserHost() ?? QUIET_HOST;
    this.clock = options.clock;
    this.pauseWith = options.pauseWith ?? "game-buses";
    this.suspendWhenHidden = options.suspendWhenHidden ?? true;
    this.queueLimit = checkQueueLimit(options.queueLimit ?? DEFAULT_QUEUE_LIMIT);
    this.staleAfterMs = options.staleAfterMs ?? DEFAULT_STALE_MS;
    checkNonNegative(this.staleAfterMs, "staleAfterMs");
    this.now = options.now ?? defaultNow;
    this.random = options.random ?? Math.random;
    this.mix = createMixStore(options.settings);
    this.loader = new Loader({
      baseUrl: withSlash(options.baseUrl ?? DEFAULT_BASE_URL),
      fetch: options.fetch ?? defaultFetch,
      canPlay: options.canPlay ?? probeCanPlay,
      context: () => this.context,
      onLoaded: (id, variant) => this.events.emit("loaded", { id, variant }),
      onFailed: (id, message) => this.events.emit("error", { id, message }),
    });

    this.lastHidden = this.host.hidden;
    this.releaseGesture = this.host.onGesture(() => {
      void this.unlock();
    });
    this.releaseVisibility = this.host.onVisibilityChange(() => {
      this.lastHidden = this.host.hidden;
      this.syncRunning();
    });
    this.followClock(options.clock);
    // A clock that is already paused when the engine starts mutes the game buses from the start.
    this.followPause();
    if (options.loop !== undefined) {
      this.stopFollowing.push(options.loop.on("beforeFrame", () => this.update()));
    }
    this.ready = this.readStored();
  }

  /** True once the browser has let the audio context run. */
  get unlocked(): boolean {
    return this.hasUnlocked;
  }

  /** The id of the music that plays or waits to play, or null. */
  get currentMusic(): string | null {
    return this.music?.id ?? null;
  }

  on<Type extends keyof AudioEvents>(type: Type, listener: Listener<AudioEvents[Type]>): Unsubscribe {
    return this.events.on(type, listener);
  }

  once<Type extends keyof AudioEvents>(type: Type, listener: Listener<AudioEvents[Type]>): Unsubscribe {
    return this.events.once(type, listener);
  }

  off<Type extends keyof AudioEvents>(type: Type, listener: Listener<AudioEvents[Type]>): void {
    this.events.off(type, listener);
  }

  async preload(ids?: readonly string[], onProgress?: (progress: PreloadProgress) => void): Promise<PreloadResult> {
    const targets = [...new Set(ids ?? this.preloadIds())];
    const sounds = targets.map((id) => this.soundOf(id));
    this.ensureContext();
    const failed: string[] = [];
    let done = 0;
    await Promise.all(
      sounds.map(async (sound) => {
        const outcomes = await Promise.all(
          sound.variants.map((_, variant) => this.loader.load(sound, variant).then(() => true, () => false)),
        );
        if (outcomes.includes(false)) failed.push(sound.id);
        done += 1;
        onProgress?.({ done, total: sounds.length, id: sound.id });
      }),
    );
    return { failed: failed.sort() };
  }

  isLoaded(id: string): boolean {
    const sound = this.sounds.get(id);
    return sound !== undefined && this.loader.isLoaded(sound);
  }

  unload(ids?: readonly string[]): void {
    const targets = ids ?? [...this.sounds.keys()];
    for (const id of targets) this.soundOf(id);
    for (const id of targets) this.loader.unload(id);
  }

  play(id: string, options: PlayOptions = {}): PlaybackHandle {
    const sound = this.soundOf(id);
    return this.schedule(sound, resolvePlay(options), { music: false, startAt: 0, fadeInMs: 0 });
  }

  playMusic(id: string, options: MusicOptions = {}): PlaybackHandle {
    const sound = this.soundOf(id);
    if (sound.bus !== MUSIC_BUS) {
      throw new AudioError("wrong-bus", `"${id}" is on the "${sound.bus}" bus; music must be on the "${MUSIC_BUS}" bus`);
    }
    const crossfadeMs = options.crossfadeMs ?? 0;
    const startAt = options.startAt ?? 0;
    checkNonNegative(crossfadeMs, "crossfadeMs");
    checkNonNegative(startAt, "startAt");
    const current = this.music;
    if (current !== null && current.id === id && !current.isEnded) return current;

    const playback = this.schedule(sound, resolvePlay({}), { music: true, startAt, fadeInMs: crossfadeMs });
    if (playback.isEnded) return playback;
    const previous = this.music;
    this.music = playback;
    if (previous !== null) previous.finish("stopped", crossfadeMs);
    this.events.emit("music-changed", { id, previous: previous?.id ?? null });
    return playback;
  }

  stopMusic(fadeMs = 0): void {
    checkNonNegative(fadeMs, "fadeMs");
    const current = this.music;
    if (current === null) return;
    this.music = null;
    current.finish("stopped", fadeMs);
    this.events.emit("music-changed", { id: null, previous: current.id });
  }

  stopAll(fadeMs = 0): void {
    checkNonNegative(fadeMs, "fadeMs");
    const previous = this.music;
    this.music = null;
    for (const playback of [...this.live]) playback.finish("stopped", fadeMs);
    this.queue = [];
    if (previous !== null) this.events.emit("music-changed", { id: null, previous: previous.id });
  }

  setVolume(bus: string, volume: number): void {
    this.requireBus(bus);
    checkRange(volume, 0, 1, "volume");
    this.mixer.setVolume(bus, volume);
    this.touched.add(bus);
    this.persist();
  }

  getVolume(bus: string): number {
    this.requireBus(bus);
    return this.mixer.volumeOf(bus);
  }

  setMuted(bus: string, muted: boolean): void {
    this.requireBus(bus);
    this.mixer.setMuted(bus, muted);
    this.touched.add(bus);
    this.persist();
  }

  isMuted(bus: string): boolean {
    this.requireBus(bus);
    return this.mixer.isMuted(bus);
  }

  async unlock(): Promise<boolean> {
    if (this.closed) return false;
    const context = this.ensureContext();
    if (context === null) return false;
    if (context.state !== "running") {
      try {
        await context.resume();
      } catch {
        // The browser refused the request. The next gesture asks again.
      }
    }
    if (context.state !== "running") return false;
    this.markUnlocked();
    return true;
  }

  /**
   * Checks the clock's pause, the page's visibility and the start of the audio, and starts the plays that
   * were waiting. Optional: the clock's events and the loop's hook already do this. Cheap when nothing changed.
   */
  update(): void {
    if (this.closed) return;
    this.followPause();
    this.lastHidden = this.host.hidden;
    const context = this.context;
    if (context !== null && !this.hasUnlocked && context.state === "running") {
      this.markUnlocked();
    }
    this.syncRunning();
  }

  async close(): Promise<void> {
    if (this.closed) return;
    this.stopAll(0);
    this.closed = true;
    for (const stop of this.stopFollowing.splice(0)) stop();
    this.releaseGesture?.();
    this.releaseGesture = null;
    this.releaseVisibility?.();
    this.releaseVisibility = null;
    // Keep the mix that was changed in the last moment before the audio goes away. The changes made before
    // the saved mix was read are written by the `ready` callbacks, which run before this continuation.
    await this.ready;
    await this.mix.save();
    const context = this.context;
    if (context !== null) {
      try {
        await context.close();
      } catch {
        // The context was already closed.
      }
    }
  }

  /** Subscribes to the pause events of a clock that has them. A plain clock is followed by `update` alone. */
  private followClock(clock: AudioOptions["clock"]): void {
    if (clock?.on === undefined) return;
    this.stopFollowing.push(
      clock.on("paused", () => this.onPauseChanged()),
      clock.on("resumed", () => this.onPauseChanged()),
    );
  }

  /** A pause event: sets the game buses' gate, and starts or stops the context when the policy needs it. */
  private onPauseChanged(): void {
    if (this.closed) return;
    if (this.followPause()) this.syncRunning();
  }

  /**
   * Follows the clock's pause: the game buses are gated while it is paused (with the "game-buses" policy).
   * Returns true when the pause changed since the last check.
   */
  private followPause(): boolean {
    const paused = this.clock?.paused ?? false;
    if (paused === this.lastPaused) return false;
    this.lastPaused = paused;
    this.mixer.setGate(paused && this.pauseWith === "game-buses");
    return true;
  }

  /**
   * Reads the saved mix, and applies the buses that the game has not changed yet. The promise resolves once
   * the mix is in place, and never rejects: the store gives the defaults when it cannot read.
   */
  private readStored(): Promise<void> {
    return this.mix.load().then(() => {
      this.applyStored(readStoredMix({ volumes: this.mix.get("volumes"), muted: this.mix.get("muted") }));
    });
  }

  /** Applies a saved mix. A bus that the game has already set keeps the game's value. */
  private applyStored(stored: StoredMix): void {
    for (const [name, volume] of Object.entries(stored.volumes)) {
      if (this.mixer.hasBus(name) && !this.touched.has(name)) this.mixer.setVolume(name, volume);
    }
    for (const name of stored.muted) {
      if (this.mixer.hasBus(name) && !this.touched.has(name)) this.mixer.setMuted(name, true);
    }
  }

  /**
   * Saves the mix after a change. The store debounces the writes, so a burst of changes is one write. A call
   * made before the saved mix has been read waits for the read, so that it does not replace stored values.
   */
  private persist(): void {
    if (this.persistQueued) return;
    this.persistQueued = true;
    void this.ready.then(() => {
      this.persistQueued = false;
      const mix = this.snapshot();
      this.mix.set("volumes", mix.volumes);
      this.mix.set("muted", mix.muted);
    });
  }

  /** The mix as it is saved: every bus volume, and the muted buses. */
  private snapshot(): StoredMix {
    const volumes: Record<string, number> = {};
    for (const name of this.mixer.names()) volumes[name] = this.mixer.volumeOf(name);
    return { volumes, muted: this.mixer.mutedNames() };
  }

  /** The ids of the sounds that the manifest marks for preloading. */
  private preloadIds(): string[] {
    return [...this.sounds.values()].filter((sound) => sound.preload).map((sound) => sound.id);
  }

  /** The sound with an id. @throws AudioError with code "unknown-id" when there is none. */
  private soundOf(id: string): AudioSound {
    const sound = this.sounds.get(id);
    if (sound === undefined) throw new AudioError("unknown-id", `no sound has the id "${id}"`);
    return sound;
  }

  /** Checks that a bus, or the master, exists. @throws AudioError with code "unknown-bus" when it does not. */
  private requireBus(name: string): void {
    if (!this.mixer.hasBus(name)) throw new AudioError("unknown-bus", `there is no bus named "${name}"`);
  }

  /**
   * Makes a playback for a sound, and routes it: it starts now, or waits. A play that a cooldown
   * refuses ends at once with "skipped".
   */
  private schedule(sound: AudioSound, play: ResolvedPlay, timing: Timing): Playback {
    const playback = new Playback(
      { id: sound.id, bus: sound.bus, baseVolume: sound.volume, level: play.volume, music: timing.music },
      (ended) => this.onPlaybackEnd(ended),
    );
    if (this.closed) {
      playback.finish("skipped");
      return playback;
    }
    const now = this.now();
    const last = this.lastStart.get(sound.id);
    if (sound.cooldown_ms > 0 && last !== undefined && now - last < sound.cooldown_ms) {
      playback.finish("skipped");
      return playback;
    }
    this.lastStart.set(sound.id, now);
    this.live.add(playback);
    this.route({
      playback,
      sound,
      variant: this.pickVariant(sound),
      play,
      startAt: timing.startAt,
      fadeInMs: timing.fadeInMs,
      queuedAt: now,
    });
    return playback;
  }

  /** Sends a request on: it fails when there is no audio at all, starts when the audio runs, else waits. */
  private route(request: PlayRequest): void {
    if (this.ensureContext() === null) {
      request.playback.finish("failed");
      this.events.emit("error", {
        id: request.sound.id,
        message: this.contextError ?? "audio is not available in this runtime",
      });
      return;
    }
    if (this.runningContext() !== null) this.begin(request);
    else this.enqueue(request);
  }

  /** Starts a request whose file is decoded, or waits for the file first. */
  private begin(request: PlayRequest): void {
    const cached = this.loader.cached(request.sound, request.variant);
    if (cached !== null) {
      this.afterLoad(request, cached);
      return;
    }
    void this.loader.load(request.sound, request.variant).then(
      (buffer) => this.afterLoad(request, buffer),
      () => request.playback.finish("failed"),
    );
  }

  /** Starts a request once its file is ready, if the audio runs; otherwise the request waits. */
  private afterLoad(request: PlayRequest, buffer: AudioBufferLike): void {
    if (request.playback.isEnded) return;
    const context = this.runningContext();
    if (context === null) {
      this.enqueue(request);
      return;
    }
    this.startNow(request, buffer, context);
  }

  /** Starts a copy of a sound. A sound with a voice limit stops its oldest copies first. */
  private startNow(request: PlayRequest, buffer: AudioBufferLike, context: AudioContextLike): void {
    const { playback, sound, play } = request;
    const limit = sound.max_voices;
    if (limit !== null) {
      const copies = [...this.live]
        .filter((other) => other.isPlaying && other.id === sound.id)
        .sort((a, b) => a.startOrder - b.startOrder);
      const excess = copies.length - limit + 1;
      for (const oldest of copies.slice(0, Math.max(0, excess))) oldest.finish("stolen", STEAL_FADE_MS);
    }
    this.startCounter += 1;
    try {
      playback.start({
        context,
        input: this.mixer.inputOf(sound.bus),
        buffer,
        loop: play.loop ?? sound.loop,
        loopPoints: sound.loop_points,
        rate: play.rate,
        detune: play.detune,
        pan: play.pan,
        delaySeconds: play.delayMs / 1000,
        offsetSeconds: request.startAt,
        fadeInSeconds: request.fadeInMs / 1000,
        order: this.startCounter,
      });
    } catch (error) {
      // The browser refused to build the nodes: the play fails, and the game is told why.
      playback.finish("failed");
      this.events.emit("error", { id: sound.id, message: describeError(error) });
      return;
    }
    this.mixer.voiceStarted(sound.bus);
  }

  /** Adds a request to the queue. The oldest waiting request is dropped when the queue is full. */
  private enqueue(request: PlayRequest): void {
    const waiting = this.queue.filter((item) => !item.playback.isEnded);
    waiting.push(request);
    while (waiting.length > this.queueLimit) {
      const dropped = waiting.shift();
      dropped?.playback.finish("skipped");
    }
    this.queue = waiting;
  }

  /** Starts the waiting requests, in order, when the audio runs. Requests that waited too long are skipped. */
  private flush(): void {
    if (this.queue.length === 0 || this.runningContext() === null) return;
    const waiting = this.queue;
    this.queue = [];
    const now = this.now();
    for (const request of waiting) {
      if (request.playback.isEnded) continue;
      if (now - request.queuedAt > this.staleAfterMs) {
        request.playback.finish("skipped");
        continue;
      }
      this.begin(request);
    }
  }

  /** The variant to play: random, and never the one that played last when there is a choice. */
  private pickVariant(sound: AudioSound): number {
    const count = sound.variants.length;
    if (count <= 1) return 0;
    const last = this.lastVariant.get(sound.id);
    let index: number;
    if (last === undefined) {
      index = Math.min(count - 1, Math.floor(this.random() * count));
    } else {
      index = Math.min(count - 2, Math.floor(this.random() * (count - 1)));
      if (index >= last) index += 1;
    }
    this.lastVariant.set(sound.id, index);
    return index;
  }

  /** Called when a play ends, for whatever reason. Keeps the counts and the music channel right. */
  private onPlaybackEnd(playback: Playback): void {
    this.live.delete(playback);
    if (playback.started) this.mixer.voiceEnded(playback.bus);
    if (this.music === playback) {
      this.music = null;
      this.events.emit("music-changed", { id: null, previous: playback.id });
    }
  }

  /** The context, once it is made. Makes it on first use; a factory that fails makes audio unavailable. */
  private ensureContext(): AudioContextLike | null {
    if (this.context !== null) return this.context;
    if (this.closed || this.contextError !== null) return null;
    let created: AudioContextLike | null;
    try {
      created = this.createContext();
    } catch (error) {
      this.contextError = `the audio context could not be created: ${describeError(error)}`;
      return null;
    }
    if (created === null) {
      this.contextError = "audio is not available in this runtime";
      return null;
    }
    this.context = created;
    this.mixer.attach(created);
    if (created.state === "running") this.setUnlocked();
    return created;
  }

  /** The context when it runs now, else null. */
  private runningContext(): AudioContextLike | null {
    const context = this.context;
    return context !== null && context.state === "running" ? context : null;
  }

  /** Records that the audio has run, and stops listening for the first gesture. */
  private setUnlocked(): void {
    this.hasUnlocked = true;
    const release = this.releaseGesture;
    this.releaseGesture = null;
    release?.();
  }

  /** Records that the audio has run, and starts the plays that were waiting. */
  private markUnlocked(): void {
    this.setUnlocked();
    this.syncRunning();
  }

  /**
   * Brings the context in line with the page and the clock: it runs unless the clock is paused (in
   * "context" mode) or the page is hidden. Then it starts the waiting plays if it runs.
   */
  private syncRunning(): void {
    const context = this.context;
    if (context === null || this.closed) return;
    if (this.hasUnlocked) {
      const wantRunning =
        !(this.pauseWith === "context" && this.lastPaused) && !(this.suspendWhenHidden && this.lastHidden);
      if (wantRunning && context.state !== "running" && context.state !== "closed" && !this.resumeInFlight) {
        this.resumeInFlight = true;
        void context.resume().then(
          () => {
            this.resumeInFlight = false;
            this.flush();
          },
          () => {
            this.resumeInFlight = false;
          },
        );
      } else if (!wantRunning && context.state === "running") {
        void context.suspend().catch(() => undefined);
      }
    }
    this.flush();
  }
}

/** Checks the queue limit: a whole number of 1 or more. */
function checkQueueLimit(limit: number): number {
  if (!Number.isInteger(limit) || limit < 1) throw new RangeError(`queueLimit must be a whole number of 1 or more, got ${limit}`);
  return limit;
}

/**
 * Creates the audio engine for a manifest. The engine loads files when they are first needed, and
 * the browser's rule for starting audio is followed: plays made before the first gesture wait for it.
 * @param options - the manifest and the hooks. Only `manifest` is required.
 * @throws RangeError or AudioError for an option out of range, or a ducking bus that is not a bus.
 */
export function createAudio(options: AudioOptions): AudioEngine {
  return new Engine(options);
}
