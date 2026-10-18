// The shapes of the characters API: the schema and character documents as they are read (every
// default written out, so code never checks for a missing field), the state that a save keeps, the
// events of the runtime and the answers of the roster queries. The field names are the JSON names
// (snake_case where the document has it), as @velvet/cards does. Texts are i18n keys: the engine
// never reads them.

/** The format tag that every document and every state carries. */
export const CHARACTER_FORMAT = "velvet-character";

/** The version of the format this package reads. Any other version is refused. */
export const CHARACTER_VERSION = 1;

/** Which way a threshold is crossed to raise its event. */
export type ThresholdWhen = "below" | "above";

/**
 * A line on a need's scale. `below` raises the event when the value drops under `at`; `above` raises
 * it when the value rises to `at` or more.
 */
export interface ThresholdDef {
  /** The id, unique within its need. It is the name of the event. */
  readonly id: string;
  /** The value the threshold sits at. Within the need's range. */
  readonly at: number;
  /** Which way the threshold raises its event. */
  readonly when: ThresholdWhen;
}

/** A need: a value that changes over time, such as hunger or energy. */
export interface NeedDef {
  /** The id. */
  readonly id: string;
  /** An i18n key. */
  readonly label: string;
  /** The lowest value. */
  readonly min: number;
  /** The highest value. */
  readonly max: number;
  /** The value a new character starts with. Within the range. */
  readonly default: number;
  /** The change per game second, before the modifiers of traits. Zero for a need that changes by events only. */
  readonly rate: number;
  /** The thresholds that raise events, in the order the schema lists them. */
  readonly thresholds: readonly ThresholdDef[];
}

/** A base stat: a value the character holds, which changes by events and traits. */
export interface BaseStatDef {
  /** The id. */
  readonly id: string;
  /** An i18n key. */
  readonly label: string;
  /** The lowest value, which the effective value never goes under. */
  readonly min: number;
  /** The highest value, which the effective value never goes over. */
  readonly max: number;
  /** The base value a new character starts with. Within the range. */
  readonly default: number;
  /** Never present on a base stat. */
  readonly formula?: undefined;
}

/** A derived stat: its value is a formula over other stats. */
export interface DerivedStatDef {
  /** The id. */
  readonly id: string;
  /** An i18n key. */
  readonly label: string;
  /** The lowest value. */
  readonly min: number;
  /** The highest value. */
  readonly max: number;
  /** The formula, in the language of docs/formats/CHARACTER.md. */
  readonly formula: string;
  /** Never present on a derived stat. */
  readonly default?: undefined;
}

/** A stat of the schema: base or derived. */
export type StatDef = BaseStatDef | DerivedStatDef;

/** A profession with its experience curve. */
export interface ProfessionDef {
  /** The id. */
  readonly id: string;
  /** An i18n key. */
  readonly label: string;
  /**
   * The total experience at which each level starts. The first is 0 (level 1), and the list rises
   * strictly. Experience stops at the last threshold.
   */
  readonly xp_thresholds: readonly number[];
}

/** One named level of affinity: it covers the values from `at` up to the next level's `at`. */
export interface LevelDef {
  /** The id, such as `friendly`. */
  readonly id: string;
  /** An i18n key. */
  readonly label: string;
  /** The lowest affinity of the level. */
  readonly at: number;
}

/** The rules of affinity between characters. */
export interface RelationshipRules {
  /** The lowest affinity. */
  readonly min: number;
  /** The highest affinity. */
  readonly max: number;
  /** The affinity of a pair that has no value yet. */
  readonly default: number;
  /** How many changes a character keeps in its history. The oldest go first. */
  readonly history_limit: number;
  /** The named levels, from the lowest. The first starts at `min`. Empty when the game has none. */
  readonly levels: readonly LevelDef[];
}

/** One change that a trait makes to a stat or to the rate of a need. */
export interface TraitModifier {
  /** Whether the modifier changes a stat (`stat`) or the rate of a need (`rate`). */
  readonly target: "stat" | "rate";
  /** The id of the stat or of the need. */
  readonly id: string;
  /** `add` adds `value`; `mul` multiplies by it. */
  readonly op: "add" | "mul";
  /** The amount to add, or the factor to multiply by. */
  readonly value: number;
}

/** A trait: a named set of modifiers that characters may have. */
export interface TraitDef {
  /** The id. */
  readonly id: string;
  /** An i18n key. */
  readonly label: string;
  /** The modifiers, in the order the schema lists them. */
  readonly modifiers: readonly TraitModifier[];
}

/** The definitions that every character of a game shares: the normal form of a schema document. */
export interface StatSchema {
  /** Always `velvet-character`. */
  readonly format: typeof CHARACTER_FORMAT;
  /** Always 1. */
  readonly version: typeof CHARACTER_VERSION;
  /** Always `schema`. */
  readonly kind: "schema";
  /** The stats, base and derived, in the order the schema lists them. */
  readonly stats: readonly StatDef[];
  /** The needs, in order. */
  readonly needs: readonly NeedDef[];
  /** The professions, in order. */
  readonly professions: readonly ProfessionDef[];
  /** The rules of affinity. */
  readonly relationships: RelationshipRules;
  /** The traits, in order. */
  readonly traits: readonly TraitDef[];
}

/** The portrait and full-body art of a character: ids from the art manifest. */
export interface CharacterArt {
  /** The art id of the portrait, if the character has one. */
  readonly portrait?: string;
  /** The art id of the full-body picture, if the character has one. */
  readonly fullbody?: string;
}

/**
 * A character's static definition: the normal form of a character document. Its values are the
 * starting values; the runtime changes them, and a save keeps the changes (see CharacterState).
 */
export interface CharacterDefinition {
  /** Always `velvet-character`. */
  readonly format: typeof CHARACTER_FORMAT;
  /** Always 1. */
  readonly version: typeof CHARACTER_VERSION;
  /** Always `character`. */
  readonly kind: "character";
  /** The id: unique in a roster. */
  readonly id: string;
  /** An i18n key for the name. */
  readonly name: string;
  /** An i18n key for the title, if the character has one. */
  readonly title?: string;
  /** The art ids. */
  readonly art: CharacterArt;
  /** Free tags, such as `villager`. A roster can find characters by one. */
  readonly tags: readonly string[];
  /** The starting base value of every base stat. */
  readonly stats: Readonly<Record<string, number>>;
  /** The starting value of every need. */
  readonly needs: Readonly<Record<string, number>>;
  /** The starting experience of every profession. */
  readonly professions: Readonly<Record<string, number>>;
  /** The traits the character starts with. */
  readonly traits: readonly string[];
  /** The starting affinity toward other characters, by their ids. Pairs not listed start at the default. */
  readonly relationships: Readonly<Record<string, number>>;
}

/** One entry of a character's affinity history. */
export interface AffinityChange {
  /** The id of the other character. */
  readonly other: string;
  /** The change that was applied, after the clamp to the range. Never zero. */
  readonly delta: number;
  /** The affinity after the change. */
  readonly value: number;
  /** An i18n key or an id that says why the change happened. */
  readonly reason: string;
}

/**
 * What a save keeps of one character: the runtime values only. The static definition is not in it;
 * a state is restored into a character made from its definition.
 */
export interface CharacterState {
  /** Always `velvet-character`. */
  readonly format: typeof CHARACTER_FORMAT;
  /** The version of the state. Always 1 for states that this package writes. */
  readonly version: typeof CHARACTER_VERSION;
  /** Always `state`. */
  readonly kind: "state";
  /** The id of the character. */
  readonly id: string;
  /** The current base value of every base stat (not the effective value). */
  readonly stats: Readonly<Record<string, number>>;
  /** The current value of every need. */
  readonly needs: Readonly<Record<string, number>>;
  /** The current experience of every profession. */
  readonly xp: Readonly<Record<string, number>>;
  /** The traits the character has now, sorted by id. */
  readonly traits: readonly string[];
  /** The affinity of the pairs that hold a value, by the other character's id, sorted by id. */
  readonly affinity: Readonly<Record<string, number>>;
  /** The affinity changes, oldest first, at most `history_limit` of them. */
  readonly history: readonly AffinityChange[];
}

/** A need crossed one of its thresholds. */
export interface ThresholdCrossed {
  readonly event: "threshold_crossed";
  /** The character whose need moved. */
  readonly character: string;
  /** The need. */
  readonly need: string;
  /** The threshold's id. */
  readonly threshold: string;
  /** Which way the threshold was crossed. */
  readonly when: ThresholdWhen;
  /** The value of the need after the change. */
  readonly value: number;
}

/** A profession reached a level. One event per level, in order, when a gain crosses several. */
export interface LevelUp {
  readonly event: "level_up";
  /** The character. */
  readonly character: string;
  /** The profession. */
  readonly profession: string;
  /** The level reached. Level 1 is the first level. */
  readonly level: number;
}

/** A character's affinity toward another moved to a different level. */
export interface AffinityLevel {
  readonly event: "affinity_level";
  /** The character who feels the affinity. */
  readonly character: string;
  /** The character it is toward. */
  readonly other: string;
  /** The level it was on. */
  readonly from: string;
  /** The level it is on now. */
  readonly to: string;
  /** `up` when the new level is higher. */
  readonly direction: "up" | "down";
}

/** What the runtime reports, tagged by `event`. */
export type CharacterEvent = ThresholdCrossed | LevelUp | AffinityLevel;

/** One pair in a character's affinities: the other character, the value and its level. */
export interface Relation {
  /** The other character's id. */
  readonly other: string;
  /** The affinity toward it. */
  readonly value: number;
  /** The level of the value, or null when the schema has no levels. */
  readonly level: string | null;
}

/** One opinion of a roster member about a character: who feels it, the value and its level. */
export interface Opinion {
  /** The id of the character who holds the affinity. */
  readonly from: string;
  /** The affinity that character has toward the one asked about. */
  readonly value: number;
  /** The level of the value, or null when the schema has no levels. */
  readonly level: string | null;
}
