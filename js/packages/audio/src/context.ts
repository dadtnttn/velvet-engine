// The part of the Web Audio API that this package uses, written as TypeScript interfaces. The
// browser's own AudioContext satisfies them, and so does the fake context in the tests. Keeping the
// surface small makes a custom host or a test double easy to write.

/** A parameter of a node, such as a gain. Changes are scheduled on the context's clock. */
export interface AudioParamLike {
  /** The value now. Setting it changes the value at once. */
  value: number;
  /** Sets the value from `time` on. Returns the parameter, as the browser does. */
  setValueAtTime(value: number, time: number): unknown;
  /** Ramps linearly from the previous event to `value` at `time`. */
  linearRampToValueAtTime(value: number, time: number): unknown;
  /** Removes the scheduled changes at or after `time`. */
  cancelScheduledValues(time: number): unknown;
}

/** A node of the audio graph that can be connected to another one. */
export interface AudioNodeLike {
  /** Sends this node's output to `destination`. */
  connect(destination: AudioNodeLike): unknown;
  /** Removes every connection from this node. */
  disconnect(): void;
}

/** A node that multiplies its input by a gain. */
export interface GainNodeLike extends AudioNodeLike {
  /** The gain. */
  readonly gain: AudioParamLike;
}

/** A node that places its input between the left and right speakers. */
export interface StereoPannerLike extends AudioNodeLike {
  /** The position: -1 is full left, 0 is the middle, 1 is full right. */
  readonly pan: AudioParamLike;
}

/** Decoded audio. Only its length is needed here. */
export interface AudioBufferLike {
  /** The length in seconds. */
  readonly duration: number;
}

/** A node that plays a decoded buffer. It can be started and stopped at a time on the context's clock. */
export interface AudioBufferSourceLike extends AudioNodeLike {
  /** The buffer to play, or null before one is set. */
  buffer: AudioBufferLike | null;
  /** True when the buffer repeats. */
  loop: boolean;
  /** Where the repeated part starts, in seconds. */
  loopStart: number;
  /** Where the repeated part ends, in seconds. */
  loopEnd: number;
  /** The speed of playback: 1 is normal. */
  readonly playbackRate: AudioParamLike;
  /** The pitch shift, in cents. */
  readonly detune: AudioParamLike;
  /** Starts at `when` on the context's clock, from `offset` seconds into the buffer. */
  start(when?: number, offset?: number): void;
  /** Stops at `when` on the context's clock. */
  stop(when?: number): void;
  /** Called when the buffer has finished, or after a stop. */
  onended: ((event: Event) => void) | null;
}

/** An audio context: the clock, the output and the factory for nodes. */
export interface AudioContextLike {
  /** The output of the graph: the speakers. */
  readonly destination: AudioNodeLike;
  /** The time in seconds on the context's clock. It moves only while the context runs. */
  readonly currentTime: number;
  /** The state: "running" plays, "suspended" is paused (or waits for a user gesture), "closed" is done. */
  readonly state: "suspended" | "running" | "closed" | "interrupted";
  /** Creates a gain node. */
  createGain(): GainNodeLike;
  /** Creates a source node that plays a buffer. */
  createBufferSource(): AudioBufferSourceLike;
  /** Creates a stereo panner. Optional: a host without it plays without panning. */
  createStereoPanner?(): StereoPannerLike;
  /** Decodes a whole file. Rejects when the bytes are not audio the context can read. */
  decodeAudioData(data: ArrayBuffer): Promise<AudioBufferLike>;
  /** Lets the clock run. */
  resume(): Promise<void>;
  /** Stops the clock. */
  suspend(): Promise<void>;
  /** Ends the context for good. */
  close(): Promise<void>;
}
