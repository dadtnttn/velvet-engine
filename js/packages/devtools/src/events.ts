// The event log: the events that the game emits, kept in arrival order with a cap, so a long session
// costs no more memory than the cap. The panel shows them newest first and can filter them by name.

import { briefText } from "./text";

/** One logged event. */
export interface EventEntry {
  /** Counts the events in the order they arrived. */
  readonly seq: number;
  /** When the event arrived, in milliseconds on the devtools' clock. */
  readonly atMs: number;
  /** The event name. */
  readonly name: string;
  /** The payload on one line. Empty for an event without a payload. */
  readonly summary: string;
}

/** The log of game events, with a cap, a pause and a name filter. */
export class EventLog {
  private readonly entries: EventEntry[] = [];
  private readonly capacity: number;
  private readonly readClock: () => number;
  private sequence = 0;
  private droppedCount = 0;
  private isPaused = false;
  private changes = 0;

  /**
   * @param capacity - how many entries the log keeps. The oldest go first.
   * @param readClock - the time, in milliseconds, given to each entry.
   */
  constructor(capacity: number, readClock: () => number) {
    this.capacity = capacity;
    this.readClock = readClock;
  }

  /** True while the log refuses new events. */
  get paused(): boolean {
    return this.isPaused;
  }

  /** Events that arrived while the log was paused, since the last clear. */
  get dropped(): number {
    return this.droppedCount;
  }

  /** The number of entries that the log holds. */
  get size(): number {
    return this.entries.length;
  }

  /** Goes up on every change, so that a view can tell when it must redraw. */
  get revision(): number {
    return this.changes;
  }

  /** Pauses or resumes the log. While paused, events are counted as dropped and not kept. */
  setPaused(on: boolean): void {
    this.isPaused = on;
    this.changes += 1;
  }

  /** Logs one event. */
  record(name: string, payload: unknown): void {
    this.changes += 1;
    if (this.isPaused) {
      this.droppedCount += 1;
      return;
    }
    this.sequence += 1;
    this.entries.push({
      seq: this.sequence,
      atMs: this.readClock(),
      name,
      summary: payload === undefined ? "" : briefText(payload),
    });
    const excess = this.entries.length - this.capacity;
    if (excess > 0) this.entries.splice(0, excess);
  }

  /** Empties the log and the dropped count. */
  clear(): void {
    this.entries.length = 0;
    this.droppedCount = 0;
    this.changes += 1;
  }

  /**
   * The entries whose names contain the filter, ignoring case, newest first. An empty filter keeps them all.
   */
  list(filter: string): EventEntry[] {
    const needle = filter.trim().toLowerCase();
    const matching =
      needle === "" ? this.entries : this.entries.filter((entry) => entry.name.toLowerCase().includes(needle));
    return [...matching].reverse();
  }
}
