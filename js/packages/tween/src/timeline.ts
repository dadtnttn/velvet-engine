// Timelines: tweens, timelines and calls placed on one time axis. A timeline plays forward or back,
// at any time scale, and it can be scrubbed to any time. Every child works out its state from the
// time it is given, not from the steps before, so seeking the timeline gives the same state as
// playing up to that time.
//
// Where a child goes is its position:
// - no position: after everything placed so far, which makes a sequence;
// - a number: that many seconds from the start;
// - a label name: at that label (labels are made with `label`);
// - "+=n" or "-=n": n seconds after, or before, the end of the timeline so far;
// - "<": at the start of the previous child; "<n", "<+=n" and "<-=n" shift it;
// - ">": at the end of the previous child; ">n", ">+=n" and ">-=n" shift it.
// Put children at 0 to run them in parallel.

import { clamp, Playback, type PlaybackOptions, type TimelineChild } from "./playback";

/** Where a timeline item goes. See the module comment for the forms. */
export type TimelinePosition = number | string;

/** Options for {@link timeline}. They are the same as the options of any playback. */
export type TimelineOptions = PlaybackOptions;

/** One item of a timeline, as an editor's timeline view draws it. */
export interface TimelineSpan {
  /** "child" for a tween or a timeline, "call" for a call. */
  readonly kind: "child" | "call";
  /** Where the item starts, in seconds from the start of the timeline. */
  readonly start: number;
  /** How long the item runs, in seconds. Zero for a call; Infinity for an endless child. */
  readonly duration: number;
  /** The child that was added (the same object), or undefined for a call. */
  readonly child: TimelineChild | undefined;
}

/** An item on the axis: a child that plays, or a call that runs when the playhead passes its time. */
type Entry =
  | { readonly kind: "child"; readonly child: TimelineChild; readonly start: number }
  | { readonly kind: "call"; readonly callback: () => void; readonly start: number };

/** A position from the end: "+=0.5" or "-=0.5". */
const FROM_END = /^([+-])=(\d*\.?\d+)$/;

/** A position from the previous item: "<", ">", "<0.2", "<+=0.2", ">-=0.5". */
const FROM_PREVIOUS = /^([<>])(?:([+-])=?)?(\d*\.?\d+)?$/;

/** A label name cannot look like a position, so that it is never mistaken for one. */
function isLabelName(name: string): boolean {
  return name.length > 0 && !/^[<>+-]/.test(name) && Number.isNaN(Number(name));
}

/** A signed number of seconds from a sign character and the digits after it. */
function signedAmount(sign: string | undefined, digits: string | undefined): number {
  const value = Number(digits ?? "0");
  return sign === "-" ? -value : value;
}

/**
 * A timeline: tweens, timelines and calls on one time axis. It is a playback, so it has the same
 * controls, and its duration is the end of its last item. Create one with {@link timeline}.
 */
export class Timeline extends Playback {
  private readonly entries: Entry[] = [];
  private readonly labels = new Map<string, number>();

  /** Starts an empty timeline on its ticker. */
  constructor(options: TimelineOptions = {}) {
    super(options);
    this.join();
  }

  /**
   * The end of the last item, in seconds. Infinity when a child is endless. It is worked out from the
   * items each time, so a nested timeline that gains children is measured again.
   */
  override get duration(): number {
    let end = 0;
    for (const entry of this.entries) {
      const reach = entry.kind === "child" ? entry.start + entry.child.duration : entry.start;
      end = Math.max(end, reach);
    }
    return end;
  }

  /**
   * Adds a child at a position (see the module comment). The child leaves its own ticker, and this
   * timeline moves it from now on. Returns this timeline, so calls can be chained.
   * @throws RangeError when the position is not valid, when the child already belongs to a timeline
   * or was cancelled, or when the child would contain this timeline.
   */
  add(child: TimelineChild, position?: TimelinePosition): this {
    if (child === this || (child instanceof Timeline && child.reaches(this))) {
      throw new RangeError("a timeline cannot hold itself");
    }
    const start = this.resolve(position);
    child.claim();
    this.entries.push({ kind: "child", child, start });
    return this;
  }

  /**
   * Adds a call that runs when the playhead passes its time, in either direction. A seek does not
   * run it. Returns this timeline.
   * @throws RangeError when the position is not valid.
   */
  call(callback: () => void, position?: TimelinePosition): this {
    const start = this.resolve(position);
    this.entries.push({ kind: "call", callback, start });
    return this;
  }

  /**
   * Names a time on the axis, so that a later position can use the name. Returns this timeline.
   * @throws RangeError when the name looks like a position, or the label already exists.
   */
  label(name: string, position?: TimelinePosition): this {
    if (!isLabelName(name)) {
      throw new RangeError(`label "${name}" must not start with <, >, + or -, and must not be a number`);
    }
    if (this.labels.has(name)) throw new RangeError(`label "${name}" already exists`);
    this.labels.set(name, this.resolve(position));
    return this;
  }

  /**
   * The time of a label, in seconds from the start.
   * @throws RangeError when there is no such label.
   */
  timeOf(name: string): number {
    const at = this.labels.get(name);
    if (at === undefined) throw new RangeError(`no label named "${name}"`);
    return at;
  }

  /** The items in the order they were added, with their starts and durations, for a timeline view. */
  spans(): TimelineSpan[] {
    return this.entries.map((entry): TimelineSpan =>
      entry.kind === "child"
        ? { kind: "child", start: entry.start, duration: entry.child.duration, child: entry.child }
        : { kind: "call", start: entry.start, duration: 0, child: undefined },
    );
  }

  /** The labels and their times, in seconds from the start. The map is a copy. */
  labelTimes(): ReadonlyMap<string, number> {
    return new Map(this.labels);
  }

  /** True when this timeline holds `target`, at any depth. Used to stop a timeline from holding itself. */
  reaches(target: Timeline): boolean {
    return this.entries.some(
      (entry) =>
        entry.kind === "child" &&
        (entry.child === target || (entry.child instanceof Timeline && entry.child.reaches(target))),
    );
  }

  /** Draws every child at `time` seconds from the start of the timeline. */
  protected override render(time: number): void {
    for (const entry of this.entries) {
      if (entry.kind === "child") entry.child.renderAt(time - entry.start);
    }
  }

  /**
   * Runs the calls that the playhead passed, in time order, and tells the children that it played
   * past their ends and past their own inner times.
   */
  protected override onCrossing(from: number, to: number): void {
    if (from === to) return;
    const forward = to > from;
    const due: Array<{ at: number; run: () => void }> = [];
    for (const entry of this.entries) {
      if (entry.kind === "call") {
        const inside = forward ? entry.start > from && entry.start <= to : entry.start >= to && entry.start < from;
        if (inside) due.push({ at: entry.start, run: entry.callback });
        continue;
      }
      const { child, start } = entry;
      const end = start + child.duration;
      if (forward && from < end && end <= to) {
        due.push({ at: end, run: () => child.endedByParent() });
      }
      const childFrom = clamp(from - start, 0, child.duration);
      const childTo = clamp(to - start, 0, child.duration);
      if (childFrom !== childTo) child.crossed(childFrom, childTo);
    }
    due.sort((a, b) => (forward ? a.at - b.at : b.at - a.at));
    for (const item of due) item.run();
  }

  /** Turns a position into seconds from the start of the timeline. */
  private resolve(position: TimelinePosition | undefined): number {
    if (position === undefined) return this.finiteEnd();
    if (typeof position === "number") {
      if (!Number.isFinite(position) || position < 0) {
        throw new RangeError(`a position must be a number of seconds from 0, got ${position}`);
      }
      return position;
    }
    const fromEnd = FROM_END.exec(position);
    if (fromEnd !== null) {
      return this.checked(this.finiteEnd() + signedAmount(fromEnd[1], fromEnd[2]), position);
    }
    const previous = FROM_PREVIOUS.exec(position);
    if (previous !== null) {
      const last = this.entries[this.entries.length - 1];
      if (previous[2] !== undefined && previous[3] === undefined) {
        throw new RangeError(`position "${position}" needs an amount after its sign`);
      }
      const offset = signedAmount(previous[2], previous[3]);
      if (last === undefined) return this.checked(offset, position);
      const base =
        previous[1] === "<" ? last.start : last.start + (last.kind === "child" ? last.child.duration : 0);
      return this.checked(base + offset, position);
    }
    const known = this.labels.get(position);
    if (known === undefined) {
      throw new RangeError(`unknown position "${position}": use a number, a label, +=n, -=n, < or >`);
    }
    return known;
  }

  /** The end of the timeline, which must be finite to place anything after it. */
  private finiteEnd(): number {
    const end = this.duration;
    if (!Number.isFinite(end)) {
      throw new RangeError("cannot place an item relative to an endless child");
    }
    return end;
  }

  /** Returns `seconds` when it is a valid start time, and throws otherwise. */
  private checked(seconds: number, position: string): number {
    if (!Number.isFinite(seconds) || seconds < 0) {
      throw new RangeError(`position "${position}" would start before the timeline or past its limit`);
    }
    return seconds;
  }
}

/**
 * Makes a timeline. It joins `options.ticker`, or the shared real-time ticker when there is none.
 * Add children with `add`, then play, reverse or seek it.
 */
export function timeline(options: TimelineOptions = {}): Timeline {
  return new Timeline(options);
}
