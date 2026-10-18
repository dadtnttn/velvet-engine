// The rules of the game, kept apart from the page so that they can be tested without a browser.
// Replace them with your own; keep the save functions in step with SAVE_VERSION.

/** The version of the save data. Bump it, and add a migration, when the shape of a save changes. */
export const SAVE_VERSION = 1;

/** The state of one game. */
export interface GameState {
  /** How many turns the player has taken. */
  readonly turns: number;
}

/** A new game: no turns taken yet. */
export function newGame(): GameState {
  return { turns: 0 };
}

/**
 * The state after one more turn.
 * @param state - the state before the turn.
 */
export function takeTurn(state: GameState): GameState {
  return { turns: state.turns + 1 };
}

/**
 * The state as a save holds it: plain JSON.
 * @param state - the state to save.
 */
export function toSave(state: GameState): { readonly turns: number } {
  return { turns: state.turns };
}

/**
 * Reads a save. A value that is not a state of this game gives null, so that a damaged save is refused.
 * @param data - the save's data, as the save store gives it.
 */
export function fromSave(data: unknown): GameState | null {
  if (typeof data !== "object" || data === null) return null;
  const turns = Reflect.get(data, "turns");
  return typeof turns === "number" && Number.isInteger(turns) && turns >= 0 ? { turns } : null;
}
