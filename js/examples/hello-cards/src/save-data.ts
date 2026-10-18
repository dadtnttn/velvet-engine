// The save document of hello-cards: the board, and the game clock. The save store wraps it in its
// own envelope (format, version, game, time), so this file only describes the game's part.

import type { Board, BoardSave } from "@velvet/cards";
import type { GameClockState } from "@velvet/core";

/** What a slot holds for this game. */
export interface GameSaveData {
  /** The board, as `Board.save` writes it. Queued events are not in it: take them first. */
  readonly board: BoardSave;
  /** The clock: game time, speed and pause. */
  readonly clock: GameClockState;
}

/**
 * Builds the save document of the game. Take the board's queued events before calling this, so
 * that no event is lost (see the game's `flushEvents`).
 *
 * @param board - the board to save.
 * @param clock - the clock's state, from `GameClock.snapshot`.
 */
export function buildSaveData(board: Board, clock: GameClockState): GameSaveData {
  return {
    board: board.save(),
    clock: { elapsed: clock.elapsed, speed: clock.speed, paused: clock.paused },
  };
}

/** True when `value` is a plain object (not null, not an array). */
function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

/** True for a whole number from 0 up. */
function isCount(value: unknown): value is number {
  return typeof value === "number" && Number.isSafeInteger(value) && value >= 0;
}

/** True when `value` has the shape of a board save written by velvet-stacks, version 1. */
function isBoardSave(value: unknown): value is BoardSave {
  return (
    isRecord(value) &&
    value.format === "velvet-stacks" &&
    value.version === 1 &&
    value.kind === "board" &&
    typeof value.rng === "string" &&
    isCount(value.next_card) &&
    isCount(value.next_stack) &&
    Array.isArray(value.cards) &&
    Array.isArray(value.stacks)
  );
}

/** True when `value` is a clock state: a finite, non-negative time, a positive speed and a pause flag. */
function isClockState(value: unknown): value is GameClockState {
  return (
    isRecord(value) &&
    typeof value.elapsed === "number" &&
    Number.isFinite(value.elapsed) &&
    value.elapsed >= 0 &&
    typeof value.speed === "number" &&
    Number.isFinite(value.speed) &&
    value.speed > 0 &&
    typeof value.paused === "boolean"
  );
}

/**
 * Reads the data of a slot. The shape is checked here; the board's own rules (ids, stacks, known
 * cards) are checked when the board is loaded, and that call can still refuse the save.
 *
 * @param value - the `data` that the save store returned.
 * @returns the save document, or null when the value is not one.
 */
export function parseSaveData(value: unknown): GameSaveData | null {
  if (!isRecord(value)) return null;
  const { board, clock } = value;
  if (!isBoardSave(board) || !isClockState(clock)) return null;
  return { board, clock };
}
