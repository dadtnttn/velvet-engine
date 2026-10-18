// The shapes of the cards API: the catalog a game writes, the events and snapshots of a board, and
// the save document. The field names are the JSON names that the Rust side writes (snake_case for
// the save document, as docs/formats/STACKS.md has them). Numbers out of a board are binary32
// values, written in their shortest decimal form: `Math.fround` gives back the exact binary32 value
// when arithmetic on them matters.

/** Picks cards: every field that is set must match. A selector with no field set matches every card. */
export interface Selector {
  /** The card's id is exactly this. */
  readonly id?: string;
  /** The card's kind is exactly this. */
  readonly kind?: string;
  /** The card carries this tag. */
  readonly tag?: string;
}

/** What may lie on a card, and how high the stack built on it may grow. */
export interface StackRule {
  /** The cards that may be dropped onto this card. An empty list accepts any card. */
  readonly accepts: readonly Selector[];
  /** The most cards the stack may hold after a drop onto this card. At least 1. */
  readonly max: number;
}

/** One kind of card, as the catalog defines it. */
export interface CardDef {
  /** The id: unique in the catalog. */
  readonly id: string;
  /** An i18n key. The engine never reads it. */
  readonly name: string;
  /** What the card is (`character`, `resource`, `building`, `zone`...). Free text. */
  readonly kind: string;
  /** Free tags. A selector can match one. */
  readonly tags: readonly string[];
  /** What may lie on the card, and how high the stack may grow. */
  readonly stack: StackRule;
  /** The value in the game's currency. The engine never reads it. */
  readonly value: number;
}

/** Some cards that a recipe needs. */
export interface Ingredient {
  /** Which cards qualify. */
  readonly select: Selector;
  /** How many of them the recipe needs. At least 1. */
  readonly count: number;
  /** Whether those cards are used up when the job finishes. Workers and zones usually are not. */
  readonly consume: boolean;
}

/** Something that a finished job makes. */
export interface RecipeOutput {
  /** The card definition to make. */
  readonly card: string;
  /** How many copies to make. At least 1. */
  readonly count: number;
  /** The chance, from 0 to 1, that each copy appears. A chance of 1 draws nothing from the generator. */
  readonly chance: number;
}

/** A recipe: a stack whose cards exactly fill its ingredients starts a job that takes `seconds`. */
export interface Recipe {
  /** The id: unique among the recipes. */
  readonly id: string;
  /** The cards a stack must hold, exactly. */
  readonly ingredients: readonly Ingredient[];
  /** How long the job takes, in seconds. Zero finishes on the next tick. */
  readonly seconds: number;
  /** What the job makes. May be empty: the ingredients are then used up. */
  readonly outputs: readonly RecipeOutput[];
  /** Higher runs first when several recipes match the same stack. */
  readonly priority: number;
}

/**
 * A catalog: the card definitions and recipes of a game, checked by the engine. `loadCatalog`
 * returns one in this form: every default written out, and the recipes in precedence order. A
 * catalog file may leave out the fields that have defaults (`tags` `[]`, `stack` `{accepts: [],
 * max: 30}`, `value` 0, `recipes` `[]`, `consume` true, `count` 1, `chance` 1, `priority` 0, and
 * `outputs` `[]`).
 */
export interface Catalog {
  /** Always `velvet-stacks`. */
  readonly format: "velvet-stacks";
  /** Always 1. */
  readonly version: 1;
  /** Always `catalog`. */
  readonly kind: "catalog";
  /** Every card definition, in the order the file lists them. */
  readonly cards: readonly CardDef[];
  /** Every recipe, in precedence order. */
  readonly recipes: readonly Recipe[];
}

/**
 * A board's seed, which makes its chance rolls: a number that is a safe integer (up to 2^53 - 1),
 * a bigint for any 64-bit value, or a string of decimal digits for any 64-bit value.
 */
export type Seed = number | bigint | string;

/**
 * What happened on the board. Tagged by `event`. A `stack_changed` with an empty `cards` list means
 * the stack is gone.
 */
export type BoardEvent =
  | { readonly event: "job_started"; readonly stack: number; readonly recipe: string }
  | { readonly event: "job_cancelled"; readonly stack: number; readonly recipe: string }
  | { readonly event: "job_finished"; readonly stack: number; readonly recipe: string }
  | {
      readonly event: "card_spawned";
      readonly card: number;
      readonly def: string;
      readonly x: number;
      readonly y: number;
    }
  | { readonly event: "card_consumed"; readonly card: number; readonly def: string }
  | { readonly event: "stack_changed"; readonly stack: number; readonly cards: readonly number[] };

/** A card on the table: its id, its definition and its position. */
export interface CardView {
  /** The instance id: unique for the life of the board, counted from 1. */
  readonly id: number;
  /** The card's definition in the catalog. */
  readonly def: string;
  /** Horizontal position, in board units. A card has the position of its stack's bottom card. */
  readonly x: number;
  /** Vertical position, in board units. */
  readonly y: number;
}

/** The job that a stack runs. */
export interface StackJob {
  /** The recipe the job runs. */
  readonly recipe: string;
  /** How far the job has got, from 0 to 1. */
  readonly progress: number;
}

/** A stack on the table: a pile of cards, bottom first. */
export interface StackView {
  /** The stack's id: unique for the life of the board, counted from 1. */
  readonly id: number;
  /** Its card ids, bottom to top. */
  readonly cards: readonly number[];
  /** The position of its bottom card, which every card of the stack shares. */
  readonly x: number;
  /** The position of its bottom card, which every card of the stack shares. */
  readonly y: number;
  /** The job the stack runs, or null when it runs none. */
  readonly job: StackJob | null;
}

/** A running job in a save: its recipe and the seconds it has run. */
export interface JobSave {
  /** The recipe the job runs. */
  readonly recipe: string;
  /** Seconds the job has run. */
  readonly elapsed: number;
}

/** A stack in a save. `job` is absent when the stack runs no job. */
export interface StackSave {
  /** The stack's id. */
  readonly id: number;
  /** Its card ids, bottom to top. */
  readonly cards: readonly number[];
  /** The job it runs, if any. */
  readonly job?: JobSave;
}

/**
 * A board's state, as `Board.save` writes it. The catalog is not in it, and queued events are not
 * either: take them before saving.
 */
export interface BoardSave {
  /** Always `velvet-stacks`. */
  readonly format: "velvet-stacks";
  /** Always 1. */
  readonly version: 1;
  /** Always `board`. */
  readonly kind: "board";
  /** The random generator's state, as decimal digits. */
  readonly rng: string;
  /** The id the next card gets. */
  readonly next_card: number;
  /** The id the next stack gets. */
  readonly next_stack: number;
  /** Every card on the table. */
  readonly cards: readonly CardView[];
  /** Every stack on the table. */
  readonly stacks: readonly StackSave[];
}
