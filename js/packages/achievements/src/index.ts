// @velvet/achievements: the achievements of a game, from a checked definitions file (docs/formats/ACHIEVEMENTS.md).
//
// Import from the package root, for example:
//   import { createAchievements } from "@velvet/achievements";

export { createAchievements } from "./achievements";
export { parseAchievements, readAchievements } from "./definitions";
export { AchievementError } from "./error";
export type { AchievementErrorCode, AchievementIssue } from "./error";
export { ACHIEVEMENTS_FORMAT, ACHIEVEMENTS_KEY, ACHIEVEMENTS_STORAGE_PREFIX, ACHIEVEMENTS_VERSION } from "./format";
export type {
  Achievements,
  AchievementCatalog,
  AchievementDefinition,
  AchievementEntry,
  AchievementEvents,
  AchievementGoal,
  AchievementLoad,
  AchievementProgress,
  AchievementsOptions,
  AchievementUnlock,
  CounterDefinition,
  ListOptions,
  LoadStatus,
  ProgressDocument,
  ProgressNotice,
  SetCounterOptions,
  UnlockOptions,
} from "./types";
