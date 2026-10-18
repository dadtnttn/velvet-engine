// The mix: the gain of each bus, the master gain, the pause gate and the ducking of music.
//
//     voice -> bus input -> bus duck -> master -> speakers
//
// The bus input holds the bus volume, the mute and the pause gate. The bus duck is the ducking,
// and only the target of the ducking has a duck that moves. The mixer keeps its settings before a
// context is attached, so a volume that the game set earlier is applied when the nodes are built.
// Every later change is a short ramp, so it does not click.

import type { AudioContextLike, GainNodeLike } from "./context";
import { rampTo } from "./params";
import { decibelsToGain, volumeToGain } from "./volume";
import { MASTER_BUS } from "./types";

/** The bus that the game's pause does not mute: interface sounds keep going. */
const UI_BUS = "ui";

/** How long a change of volume or mute takes, in seconds. Short enough to be instant, long enough not to click. */
const RAMP_SECONDS = 0.02;

/** The ducking settings in their resolved form: every field set. */
export interface DuckingSettings {
  /** The bus whose sounds cause the ducking. */
  readonly trigger: string;
  /** The bus that is turned down. */
  readonly target: string;
  /** How far the target goes down, in decibels. */
  readonly decibels: number;
  /** Milliseconds for the target to go down. */
  readonly attackMs: number;
  /** Milliseconds for the target to come back up. */
  readonly releaseMs: number;
}

/** The two gains of a bus: its input, which voices connect to, and its duck. */
interface Channel {
  readonly input: GainNodeLike;
  readonly duck: GainNodeLike;
}

/** The buses of one mix. Each bus is a name; the master is the reserved name "master". */
export class Mixer {
  private readonly busNames: readonly string[];
  private readonly volumes = new Map<string, number>();
  private readonly muted = new Set<string>();
  private readonly ducking: DuckingSettings | null;
  private gated = false;
  private activeTriggers = 0;
  private context: AudioContextLike | null = null;
  private masterGain: GainNodeLike | null = null;
  private readonly channels = new Map<string, Channel>();

  /**
   * @param busNames - every bus of the manifest, built-in and custom. The master is added here.
   * @param ducking - the ducking to run, or null for none.
   */
  constructor(busNames: readonly string[], ducking: DuckingSettings | null) {
    this.busNames = busNames;
    this.ducking = ducking;
    for (const name of [MASTER_BUS, ...busNames]) this.volumes.set(name, 1);
  }

  /** The names of the master and of every bus, the master first. */
  names(): string[] {
    return [...this.volumes.keys()];
  }

  /** True when `name` is the master or one of the buses. */
  hasBus(name: string): boolean {
    return this.volumes.has(name);
  }

  /** The volume of a bus or of the master, from 0 to 1. */
  volumeOf(name: string): number {
    return this.volumes.get(name) ?? 1;
  }

  /** True while a bus, or the master, is muted. */
  isMuted(name: string): boolean {
    return this.muted.has(name);
  }

  /** The names of the buses that are muted, the master included. */
  mutedNames(): string[] {
    return [...this.muted];
  }

  /**
   * Builds the gain nodes in `context` and applies the settings at once. Nothing plays yet, so there
   * is nothing to ramp from. A second call does nothing.
   */
  attach(context: AudioContextLike): void {
    if (this.context !== null) return;
    this.context = context;
    const master = context.createGain();
    master.connect(context.destination);
    this.masterGain = master;
    for (const name of this.busNames) {
      const input = context.createGain();
      const duck = context.createGain();
      input.connect(duck);
      duck.connect(master);
      this.channels.set(name, { input, duck });
    }
    master.gain.value = this.targetOf(MASTER_BUS);
    for (const [name, channel] of this.channels) {
      channel.input.gain.value = this.targetOf(name);
      channel.duck.gain.value = this.duckTargetOf(name);
    }
  }

  /**
   * The input gain of a bus, which a voice connects to.
   * @throws Error when the mixer is not attached yet, or the bus is unknown. Both are bugs of the caller.
   */
  inputOf(name: string): GainNodeLike {
    const channel = this.channels.get(name);
    if (channel === undefined) throw new Error(`bus "${name}" has no channel: attach the mixer first`);
    return channel.input;
  }

  /** Sets the volume of a bus, or of the master. The caller has checked the range and the name. */
  setVolume(name: string, volume: number): void {
    this.volumes.set(name, volume);
    this.apply(name);
  }

  /** Mutes or unmutes a bus, or the master. */
  setMuted(name: string, muted: boolean): void {
    if (muted) this.muted.add(name);
    else this.muted.delete(name);
    this.apply(name);
  }

  /** Turns the game's pause gate on or off. While it is on, every bus except `ui` is silent. */
  setGate(gated: boolean): void {
    if (this.gated === gated) return;
    this.gated = gated;
    for (const name of this.busNames) {
      if (name !== UI_BUS) this.apply(name);
    }
  }

  /** Counts a voice that starts on a bus. The first voice of the trigger bus starts the ducking. */
  voiceStarted(bus: string): void {
    const ducking = this.ducking;
    if (ducking === null || bus !== ducking.trigger) return;
    this.activeTriggers += 1;
    if (this.activeTriggers === 1) this.rampDuck(ducking.attackMs / 1000);
  }

  /** Counts a voice that ends on a bus. The last voice of the trigger bus releases the ducking. */
  voiceEnded(bus: string): void {
    const ducking = this.ducking;
    if (ducking === null || bus !== ducking.trigger) return;
    this.activeTriggers = Math.max(0, this.activeTriggers - 1);
    if (this.activeTriggers === 0) this.rampDuck(ducking.releaseMs / 1000);
  }

  /** Moves the duck of the ducking target to its level now. */
  private rampDuck(seconds: number): void {
    const ducking = this.ducking;
    const context = this.context;
    if (ducking === null || context === null) return;
    const channel = this.channels.get(ducking.target);
    if (channel === undefined) return;
    rampTo(context, channel.duck.gain, this.duckTargetOf(ducking.target), seconds);
  }

  /** The gain that a duck should have now: the ducking level while a trigger plays, else 1. */
  private duckTargetOf(name: string): number {
    const ducking = this.ducking;
    if (ducking === null || name !== ducking.target || this.activeTriggers === 0) return 1;
    return decibelsToGain(ducking.decibels);
  }

  /** The gain that a bus input or the master should have now. */
  private targetOf(name: string): number {
    if (name === MASTER_BUS) {
      return this.muted.has(MASTER_BUS) ? 0 : volumeToGain(this.volumeOf(MASTER_BUS));
    }
    const silent = this.muted.has(name) || (this.gated && name !== UI_BUS);
    return silent ? 0 : volumeToGain(this.volumeOf(name));
  }

  /** Ramps the gain of one bus, or of the master, to its target now. */
  private apply(name: string): void {
    const context = this.context;
    if (context === null) return;
    const node = name === MASTER_BUS ? this.masterGain : (this.channels.get(name)?.input ?? null);
    if (node === null) return;
    rampTo(context, node.gain, this.targetOf(name), RAMP_SECONDS);
  }
}
