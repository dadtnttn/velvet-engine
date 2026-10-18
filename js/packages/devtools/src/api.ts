// What the overlay reads from the devtools and asks of them. The devtools class implements this interface,
// so the overlay never reaches into the devtools' fields, and it can be read on its own.

import type { ErrorReport, Logger } from "@velvet/core";
import type { Completion, ConsoleKind, ConsoleOutput } from "./console";
import type { EventLog } from "./events";
import type { TimeState } from "./time";
import type { DevtoolsStats } from "./types";

/** A view registered with `inspect`, as the State tab reads it. */
export interface InspectView {
  readonly name: string;
  readonly read: () => unknown;
  readonly refreshMs: number;
}

/** What the Performance and Time tabs need to know about the loop. */
export interface LoopState {
  /** True when the update and render times are measured: the loop was given to the tools, or its options went through `wrapLoop`. */
  readonly timed: boolean;
  /** True once the loop options have gone through `wrapLoop`, so that a pause holds the game's update. */
  readonly held: boolean;
  /** Whether the loop is running, or null when no loop was given. */
  readonly running: boolean | null;
}

/** The overlay's view of the devtools. */
export interface OverlayApi {
  /** The time in milliseconds, on the devtools' clock. */
  now(): number;
  /** Frame and memory numbers. */
  stats(): DevtoolsStats;
  /** The recent frame intervals, oldest first. */
  frameHistory(): readonly number[];
  /** Whether the loop is wrapped, and whether it runs. */
  loopState(): LoopState;
  /** A one-line description of the stage's pixels, or a note that no stage was given. */
  stageText(): string;
  /** The clock's state and the time controls. */
  timeState(): TimeState;
  /** Stops game time. */
  pause(): void;
  /** Lets game time run again. */
  resume(): void;
  /** Moves game time on by one step. */
  step(): void;
  /** Sets the clock's speed. */
  setSpeed(speed: number): void;
  /** Turns slow motion on or off. */
  setSlowMotion(on: boolean): void;
  /** Clears the frame counters. */
  resetCounters(): void;
  /** Hides the panel. */
  hide(): void;
  /** The event log. */
  events(): EventLog;
  /** True while an event bus is connected, so that the log has events to show. */
  eventsConnected(): boolean;
  /** The registered views, by name. */
  views(): ReadonlyMap<string, InspectView>;
  /** The pinned numbers, by name. */
  watches(): ReadonlyMap<string, () => number>;
  /** False when the game turned the pinned watches off with the `hud` option. */
  hudEnabled(): boolean;
  /** The logger whose entries the Log tab lists, or undefined when none was given. */
  logger(): Logger | undefined;
  /** The error reports, most recent first. Empty when no reporter was given. */
  errorReports(): readonly ErrorReport[];
  /** How many error entries the logger's buffer holds. */
  errorCount(): number;
  /** The console's output lines. */
  output(): ConsoleOutput;
  /** Adds a line to the console output. */
  print(kind: ConsoleKind, text: string): void;
  /** Runs one line of the console: its echo, its command and its result or error. */
  runLine(line: string): void;
  /** Completes the console line at its end. */
  complete(line: string): Completion;
  /** The line before the current one in the history, or null. The draft is kept for the Down key. */
  historyPrevious(draft: string): string | null;
  /** The line after the current one in the history, or the draft, or null when not browsing. */
  historyNext(): string | null;
  /** Stops browsing the history. */
  historyReset(): void;
}
