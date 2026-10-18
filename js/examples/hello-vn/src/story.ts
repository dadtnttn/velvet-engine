// The story: one guion conversation at a time, in the language in use. It is the dialogue session that the
// dialogue box plays, and it keeps the record that a save needs.
//
// A conversation cannot be saved as it stands, so the story keeps two things: the scene it started in with
// the variables it had then, and the list of box actions taken since (`next`, or `choose` with an option).
// Restoring replays that list on a fresh conversation, which gives the same box and the same variables, because
// the guiones are deterministic. A language switch is the same replay on the other language's guion: both files
// have the same scenes and boxes (see content.test.ts).
//
// The stage directives of the boxes (`@fondo` for the background, `@musica` for the music) are read as the boxes
// are visited, replays included, so the stage state after a restore is the one of the boxes that were played.

import { Conversation, type GuionLibrary, type GuionVars, type Shown, type StageDirective } from "@velvet/guion";
import type { DialogueSession } from "@velvet/ui";
import { ENDING_VAR, type Language } from "./config";

/** One action of the player on the conversation: going on past a box, or taking an option. */
export type StoryStep = { readonly kind: "next" } | { readonly kind: "choose"; readonly index: number };

/** What a save keeps of the story: the scene and the variables it started with, and the actions since. */
export interface StorySnapshot {
  /** The label of the scene the conversation started in (`puerto`), which is the same in every language. */
  readonly scene: string;
  /** The variables when that scene started. */
  readonly vars: GuionVars;
  /** The box actions taken since then, in order. */
  readonly steps: readonly StoryStep[];
}

/** The stage that the boxes so far set up: the background and the music, as their ids (null for none). */
export interface StageState {
  readonly background: string | null;
  readonly music: string | null;
}

/** Callbacks for the player's own actions. A restore calls neither: it only replays what was played. */
export interface StoryHooks {
  /** The player took option `index` of a choice. Runs after the option is applied. */
  readonly onChoice?: (index: number) => void;
  /** The conversation moved into another scene (by a choice, a jump or going on). Runs with the label. */
  readonly onScene?: (scene: string) => void;
}

/** The variables of a story that has not started. */
export const NO_VARS: GuionVars = { values: {}, seen: [] };

/** What the boxes so far say about the scene, the stage and the end of the conversation. */
interface Track {
  background: string | null;
  music: string | null;
  scene: string | null;
  over: boolean;
}

/** A track of a conversation that has not played anything yet. */
function emptyTrack(): Track {
  return { background: null, music: null, scene: null, over: false };
}

/** A copy of variables that nothing else holds, so a snapshot cannot change with the conversation. */
function copyVars(vars: GuionVars): GuionVars {
  return { values: { ...vars.values }, seen: [...vars.seen] };
}

/** The label of a scene id: `faro.puerto` gives `puerto`, and so does `faro_en.puerto`. */
export function sceneLabel(id: string): string {
  const dot = id.indexOf(".");
  return dot < 0 ? id : id.slice(dot + 1);
}

/**
 * Reads one stage directive into the track. Only the two that this game uses change anything: `fondo` sets the
 * background art and `musica` the music (an empty argument stops the music).
 */
function applyDirective(track: Track, directive: StageDirective): void {
  const value = directive.arg === "" ? null : directive.arg;
  if (directive.cmd === "fondo") track.background = value;
  else if (directive.cmd === "musica") track.music = value;
}

/** The story of the game: a dialogue session over the guiones of the game's library. */
export class Story implements DialogueSession {
  private readonly library: GuionLibrary;
  private readonly fileIds: Readonly<Record<Language, string>>;
  private readonly hooks: StoryHooks;
  private language: Language;
  private conversation: Conversation | null = null;
  private startScene: string | null = null;
  private startVars: GuionVars = NO_VARS;
  private steps: StoryStep[] = [];
  private track: Track = emptyTrack();
  private revisions = 0;

  /**
   * @param library - the guiones of the game, both languages added.
   * @param fileIds - the id of each language's guion file (`faro` and `faro_en`).
   * @param language - the language to start in.
   * @param hooks - the callbacks for the player's own actions.
   */
  constructor(library: GuionLibrary, fileIds: Readonly<Record<Language, string>>, language: Language, hooks: StoryHooks = {}) {
    this.library = library;
    this.fileIds = fileIds;
    this.language = language;
    this.hooks = hooks;
  }

  /** The language whose guion plays. */
  get lang(): Language {
    return this.language;
  }

  /** True once a scene has started, by `begin` or `restore`. */
  get started(): boolean {
    return this.conversation !== null;
  }

  /** The label of the scene on screen, or null before the story starts. */
  get scene(): string | null {
    return this.track.scene;
  }

  /** The background and the music that the boxes so far set up. */
  get stage(): StageState {
    return { background: this.track.background, music: this.track.music };
  }

  /** True when the conversation has ended. */
  get finished(): boolean {
    return this.track.over;
  }

  /** The story's variables as they are now. */
  get vars(): GuionVars {
    return this.conversation === null ? NO_VARS : this.conversation.vars;
  }

  /** The number of the ending that the player reached (1 or 2), or 0 while the story has not ended. */
  get endingNumber(): number {
    return this.track.over ? Number(this.vars.values[ENDING_VAR] ?? 0) : 0;
  }

  /** Counts every change of where the story stands: a start, a restore, a box passed or an option taken. */
  get revision(): number {
    return this.revisions;
  }

  /** How many actions the player has taken since the story began. */
  get stepCount(): number {
    return this.steps.length;
  }

  /**
   * Starts a scene in the language in use, and visits its first box. A new story: the record starts again.
   * @param scene - the label of the scene.
   * @param vars - the variables when it starts.
   * @throws GuionError with code "unknown-scene" when the guion has no such scene.
   */
  begin(scene: string, vars: GuionVars = NO_VARS): void {
    const conversation = this.open(scene, vars);
    this.conversation = conversation;
    this.startScene = scene;
    this.startVars = copyVars(vars);
    this.steps = [];
    this.track = emptyTrack();
    this.revisions += 1;
    const changed = this.visit(conversation.current(), this.track);
    if (changed) this.hooks.onScene?.(this.track.scene ?? scene);
  }

  /**
   * The record of the story: the scene it started in, its variables then, and the actions since.
   * @throws Error when the story has not started.
   */
  snapshot(): StorySnapshot {
    if (this.startScene === null) throw new Error("the story has not started");
    return { scene: this.startScene, vars: copyVars(this.startVars), steps: [...this.steps] };
  }

  /**
   * Replaces the story with a record, by replaying its actions. The story is left as it was when the record
   * cannot be replayed (the error is thrown, and nothing changes).
   * @param snapshot - the record, as `snapshot` gives it.
   * @throws GuionError when the scene or an action of the record does not fit the guion.
   */
  restore(snapshot: StorySnapshot): void {
    const conversation = this.open(snapshot.scene, snapshot.vars);
    const track = emptyTrack();
    this.visit(conversation.current(), track);
    for (const step of snapshot.steps) {
      const box = step.kind === "next" ? conversation.next() : conversation.choose(step.index);
      this.visit(box, track);
    }
    this.conversation = conversation;
    this.startScene = snapshot.scene;
    this.startVars = copyVars(snapshot.vars);
    this.steps = [...snapshot.steps];
    this.track = track;
    this.revisions += 1;
  }

  /**
   * Plays the same story in another language: the record is replayed on that language's guion. Nothing changes
   * when the story has not started (the language is kept for the next start), or when the replay fails.
   * @param language - the language to switch to.
   */
  switchLanguage(language: Language): void {
    if (language === this.language) return;
    if (this.startScene === null) {
      this.language = language;
      return;
    }
    const record = this.snapshot();
    const previous = this.language;
    this.language = language;
    try {
      this.restore(record);
    } catch (error) {
      this.language = previous;
      throw error;
    }
  }

  /** The box on screen. Changes nothing. */
  current(): Shown {
    return this.active().current();
  }

  /** Goes on past the box on screen, and returns the box that follows. */
  next(): Shown {
    const box = this.active().next();
    this.steps.push({ kind: "next" });
    this.revisions += 1;
    const changed = this.visit(box, this.track);
    if (changed) this.hooks.onScene?.(this.track.scene ?? "");
    return box;
  }

  /**
   * Takes an option of the choice on screen, and returns the box after it. A closed option is refused by the
   * guion, which throws, and then nothing changes.
   * @param index - the option, counted from 0.
   */
  choose(index: number): Shown {
    const box = this.active().choose(index);
    this.steps.push({ kind: "choose", index });
    this.revisions += 1;
    const changed = this.visit(box, this.track);
    this.hooks.onChoice?.(index);
    if (changed) this.hooks.onScene?.(this.track.scene ?? "");
    return box;
  }

  /** The conversation of the story, or an error when no scene has started. */
  private active(): Conversation {
    if (this.conversation === null) throw new Error("the story has not started");
    return this.conversation;
  }

  /** Starts a scene of the language in use, and returns its conversation. */
  private open(scene: string, vars: GuionVars): Conversation {
    return this.library.start(`${this.fileIds[this.language]}.${scene}`, vars);
  }

  /**
   * Reads what a box says into the track: its stage directives, its scene and whether it ends the conversation.
   * @returns true when the box is in another scene than the track's.
   */
  private visit(box: Shown, track: Track): boolean {
    for (const directive of box.stage) applyDirective(track, directive);
    const label = sceneLabel(box.scene);
    const changed = label !== track.scene;
    track.scene = label;
    track.over = box.type === "end" || box.type === "vn";
    return changed;
  }
}
