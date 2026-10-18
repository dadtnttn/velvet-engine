// GuionLibrary: the guiones of one game, read with one cast. It finds the scene to play and starts
// conversations.

import { loadVelvetWasm, type GuionLibrary as NativeLibrary } from "@velvet/wasm";
import { Conversation } from "./conversation";
import type { CastSpec, GuionFile, GuionProblem, GuionVars, SceneInfo } from "./types";
import { attempt, decode } from "./wire";

/** Who the player talks to, or which place they use: the arguments of `entry`. */
export interface EntryQuery {
  /** The NPC, named as the guiones name them (`quien=` in a scene's heading). */
  readonly who?: string;
  /** The place, named as the guiones name it (`en=` in a scene's heading). */
  readonly place?: string;
}

/** The variables of a story that has not started yet. */
const EMPTY_VARS: GuionVars = { values: {}, seen: [] };

/**
 * The guiones of one game. Make one with `GuionLibrary.create`, add each guion file with `add`,
 * then ask `entry` which scene plays when the player talks to someone or uses a place, and start
 * it with `start`. A conversation keeps the guiones as they were when it started.
 *
 * @example
 * const library = await GuionLibrary.create({ thinker: { id: "viajera", name: "La viajera" } });
 * library.add("posadera.md", await readText("posadera.md"));
 * const scene = library.entry({ who: "greta" }, vars);
 * if (scene !== null) conversation = library.start(scene, vars);
 */
export class GuionLibrary {
  readonly #native: NativeLibrary;

  /** @param native - the module's library object. Use `GuionLibrary.create` to make one. */
  private constructor(native: NativeLibrary) {
    this.#native = native;
  }

  /**
   * Makes an empty library whose guiones are read with `cast`.
   *
   * @param cast - the game's cast. Without one, `narrador` and `narrator` are the narrator.
   * @returns the library.
   * @throws {GuionError} with code `bad-input` when the cast is not valid.
   */
  static async create(cast: CastSpec = {}): Promise<GuionLibrary> {
    const wasm = await loadVelvetWasm();
    const native = attempt(() => new wasm.GuionLibrary(JSON.stringify(cast)));
    return new GuionLibrary(native);
  }

  /**
   * Reads one more guion and returns its summary. The file's problems are kept in the library too.
   *
   * @param file - the file name, which gives the file id (`posadera.md` is `posadera`).
   * @param text - the Markdown of the guion.
   */
  add(file: string, text: string): GuionFile {
    return decode<GuionFile>(this.#native.add(file, text));
  }

  /** Every problem of every guion added, in the order they were found. */
  problems(): GuionProblem[] {
    return decode<GuionProblem[]>(this.#native.problems());
  }

  /** The scenes of every guion added, in file order, with their attributes. */
  scenes(): SceneInfo[] {
    return decode<SceneInfo[]>(this.#native.scenes());
  }

  /**
   * The scene to play when the player talks to `who` or uses `place`: the first in file order
   * that matches, whose conditions hold for `vars` and that is not a spent `una_vez` scene.
   *
   * @param query - who the player talks to, or which place they use.
   * @param vars - the story's variables.
   * @returns the scene's id, or null when no scene plays.
   */
  entry(query: EntryQuery, vars: GuionVars = EMPTY_VARS): string | null {
    const reply = decode<{ scene: string | null }>(
      this.#native.entry(query.who, query.place, JSON.stringify(vars)),
    );
    return reply.scene;
  }

  /**
   * Starts a scene: runs it up to its first box, and returns the conversation. The scene is
   * marked as seen in the story's variables of the conversation.
   *
   * @param scene - the scene's id, `<file id>.<label>` (such as `posadera.llegada`).
   * @param vars - the story's variables when the conversation starts.
   * @throws {GuionError} with code `unknown-scene` when no scene has that id, and `bad-input` when
   *   `vars` is not valid.
   */
  start(scene: string, vars: GuionVars = EMPTY_VARS): Conversation {
    const native = attempt(() => this.#native.start(scene, JSON.stringify(vars)));
    return new Conversation(native);
  }
}
