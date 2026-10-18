// The achievements of hello-2d: the runtime of @velvet/achievements, made from the game's definitions file, with the
// progress saved under the game's own key. The game advances the counters and unlocks the achievements that
// need a manual call (a clean finish, a fast one); the runtime does the rest, including the achievement that
// needs the others.

import { createAchievements, type Achievements, type AchievementUnlock } from "@velvet/achievements";
import type { SettingsBackend, StorageLike } from "@velvet/core";
import { GAME_ID } from "./config";

/** What the achievements need to start. */
export interface AchievementsSetup {
  /** The text of the definitions file. */
  readonly definitions: string;
  /** Runs for each unlock made in this run: the place where a platform bridge would report it. */
  readonly onUnlock: (unlock: AchievementUnlock) => void;
  /** Where the progress is kept, for tests and custom hosts. Default: the bridge, else localStorage, else memory. */
  readonly backend?: SettingsBackend;
  /** The web storage. `null` skips it. */
  readonly storage?: StorageLike | null;
  /** The bridge's settings. `null` skips the bridge. */
  readonly bridge?: { readonly settings: SettingsBackend } | null;
}

/**
 * Makes the achievements of the game. The progress is loaded by the caller, once, before the first screen.
 * @param setup - the definitions and where the progress is kept.
 * @throws AchievementError when the definitions file is not valid.
 */
export function createGameAchievements(setup: AchievementsSetup): Achievements {
  return createAchievements({
    definitions: setup.definitions,
    key: GAME_ID,
    clock: () => Date.now(),
    onUnlock: setup.onUnlock,
    backend: setup.backend,
    storage: setup.storage,
    bridge: setup.bridge,
  });
}
