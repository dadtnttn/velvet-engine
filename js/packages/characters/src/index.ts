// @velvet/characters: the character sheet format (docs/formats/CHARACTER.md). It holds stat schemas,
// character sheets, a safe formula language for derived stats, needs with thresholds, professions with
// experience curves, affinity between characters with levels and a history, traits, and the saves of
// characters. Import from the package root, for example:
//   import { Character, Roster, parseCharacter, parseStatSchema } from "@velvet/characters";

export { parseStatSchema } from "./schema";
export { parseCharacter } from "./definition";
export { Character } from "./character";
export type { CharacterOptions } from "./character";
export { Roster } from "./roster";
export type { RelationQuery } from "./roster";
export { CharacterError } from "./error";
export type { CharacterErrorCode } from "./error";
export { evaluateExpression, expressionVariables, parseExpression } from "./expression";
export type { Expression, FunctionName } from "./expression";
export { CHARACTER_FORMAT, CHARACTER_VERSION } from "./types";
export type {
  AffinityChange,
  AffinityLevel,
  BaseStatDef,
  CharacterArt,
  CharacterDefinition,
  CharacterEvent,
  CharacterState,
  DerivedStatDef,
  LevelDef,
  LevelUp,
  NeedDef,
  Opinion,
  ProfessionDef,
  ThresholdCrossed,
  ThresholdDef,
  ThresholdWhen,
  Relation,
  RelationshipRules,
  StatDef,
  StatSchema,
  TraitDef,
  TraitModifier,
} from "./types";
