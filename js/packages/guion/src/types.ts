// The shapes of the guion API: what velvet-wasm replies, as TypeScript types. The field names
// are the JSON names that the Rust side writes (camelCase); the words of the guion format keep
// their own spelling. See docs/formats/GUION.md for what each one means.

/** A condition, as written in a heading (`si=`), an option (`[...]`) or `@si`. */
export interface GuionCondition {
  readonly var: string;
  readonly op: "eq" | "ne" | "gt" | "gte" | "lt" | "lte";
  readonly value: number;
}

/** The attributes of a scene, from the braces after its heading. */
export interface SceneAttrs {
  /** The NPC who starts the scene when the player talks to them (`quien=`), or null. */
  readonly quien: string | null;
  /** The place whose use starts the scene (`en=`), or null. */
  readonly en: string | null;
  /** The conditions that must hold for the scene to start (`si=`). */
  readonly si: readonly GuionCondition[];
  /** True when the scene starts only the first time (`una_vez`). */
  readonly unaVez: boolean;
  /** True for a visual-novel scene (`vn`). */
  readonly vn: boolean;
}

/** A scene of a guion file, without its lines. */
export interface SceneInfo {
  /** `<file id>.<label>`: how the game names the scene (for `start` and the story's `vn`). */
  readonly id: string;
  /** The heading as an identifier (`## La Llegada` is `la_llegada`). */
  readonly label: string;
  /** The heading as written, without its attributes. */
  readonly title: string;
  /** The line of the heading, counted from 1. */
  readonly line: number;
  /** True when a jump or an option of the same file leads to this scene (a branch). */
  readonly targeted: boolean;
  readonly attrs: SceneAttrs;
}

/** A mistake found in a guion file, with the line it is on (counted from 1). */
export interface GuionProblem {
  readonly file: string;
  readonly line: number;
  readonly message: string;
}

/** The summary of one guion file: its id, title, room, scenes, problems and metadata. */
export interface GuionFile {
  readonly file: string;
  readonly id: string;
  readonly title: string;
  /** The default room of the file (`sala:` in its front matter), or null. */
  readonly sala: string | null;
  readonly scenes: readonly SceneInfo[];
  readonly problems: readonly GuionProblem[];
  /**
   * The front matter's other keys (`categoria: escena`), with each value as written: a list such as
   * `participantes: Lira, Nyxara` stays one string, and the game splits it. The keys are in the
   * order of the file. Read it with `meta[key]`, or `Object.hasOwn(meta, key)` to tell an empty value
   * from a missing key. See docs/formats/GUION.md, "Metadata".
   */
  readonly meta: Readonly<Record<string, string>>;
}

/** The story's variables and the scenes already seen. The shape is saved with the game. */
export interface GuionVars {
  readonly values: Readonly<Record<string, number>>;
  readonly seen: readonly string[];
}

/** Who is who in a game's guiones (see docs/formats/GUION.md, "The cast"). */
export interface CastSpec {
  /** Each character id with the names that the guiones use for it. */
  readonly characters?: Readonly<Record<string, readonly string[]>>;
  /** The character whose thoughts the bracketed lines are, and the name they are shown under. */
  readonly thinker?: { readonly id: string; readonly name: string };
}

/** A stage directive run since the previous box: `@fondo posada/entrada` is `fondo` and its argument. */
export interface StageDirective {
  /** The directive without its `@` (`fondo`, `izq`, `titulo`...). */
  readonly cmd: string;
  /** What follows the directive on its line; empty when nothing does. */
  readonly arg: string;
}

/** The kinds of line that the box shows. */
export type LineKind = "dialogue" | "narration" | "sound" | "thought";

/** The parts that every box has. */
export interface ShownBase {
  /** The scene the conversation is in (`<file id>.<label>`). */
  readonly scene: string;
  /** The stage directives run since the box before, in order. The box itself does not show them. */
  readonly stage: readonly StageDirective[];
}

/** A line: spoken dialogue, narration, a sound or a thought. */
export interface ShownLine extends ShownBase {
  readonly type: "line";
  /** The character id of the speaker (`narrator` for narration and sounds). */
  readonly who: string;
  /** The name shown above the box: empty for narration and sounds. */
  readonly name: string;
  readonly text: string;
  readonly kind: LineKind;
  /** How the speaker looks (`**Greta (sonriendo):**` is `sonriendo`), or null. */
  readonly mood: string | null;
}

/** One option of a choice: its label, and whether its condition holds (it can be taken). */
export interface ChoiceOption {
  readonly label: string;
  readonly available: boolean;
}

/** A choice, waiting for the player to take one of its options with `choose`. */
export interface ShownChoice extends ShownBase {
  readonly type: "choice";
  readonly prompt: string | null;
  readonly options: readonly ChoiceOption[];
}

/** The conversation hands over to a visual-novel scene (`@vn`). */
export interface ShownVn extends ShownBase {
  readonly type: "vn";
  /** The scene that takes over (`<file id>.<label>`). */
  readonly to: string;
}

/** The conversation is over. */
export interface ShownEnd extends ShownBase {
  readonly type: "end";
}

/** What the dialogue box shows now. Switch on `type` to tell the cases apart. */
export type Shown = ShownLine | ShownChoice | ShownVn | ShownEnd;
