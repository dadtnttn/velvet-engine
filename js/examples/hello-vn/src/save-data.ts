// The save document of hello-vn: the record of the story (where the conversation is, its variables and the
// actions since). The save store wraps it in its own envelope, and the save list wraps it in { meta, state },
// so this file only describes the game's part and checks it when it is read back.

import type { GuionVars } from "@velvet/guion";
import type { StorySnapshot, StoryStep } from "./story";

/** What a slot holds for this game. */
export interface GameSave {
  /** The story: the scene it started in, its variables then, and the actions since. */
  readonly story: StorySnapshot;
}

/** True when `value` is a plain object (not null, not an array). */
function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

/** True for a whole number from 0 up. */
function isCount(value: unknown): value is number {
  return typeof value === "number" && Number.isSafeInteger(value) && value >= 0;
}

/** A scene label or a variable name: lower-case letters, digits and underscores, as the guion slugs them. */
const LABEL = /^[a-z0-9_]{1,64}$/;

/**
 * Reads the variables of a save: whole numbers by name, and the scenes already seen.
 * @param value - the value stored under `vars`.
 */
function readVars(value: unknown): GuionVars | null {
  if (!isRecord(value)) return null;
  const stored = value["values"];
  const seenValue = value["seen"];
  if (!isRecord(stored) || !Array.isArray(seenValue)) return null;
  const values: Record<string, number> = {};
  for (const [name, amount] of Object.entries(stored)) {
    if (!LABEL.test(name) || typeof amount !== "number" || !Number.isSafeInteger(amount)) return null;
    values[name] = amount;
  }
  const seen: string[] = [];
  for (const item of seenValue) {
    if (typeof item !== "string") return null;
    seen.push(item);
  }
  return { values, seen };
}

/**
 * Reads one action of a record: going on, or taking option `index`.
 * @param value - the stored action.
 */
function readStep(value: unknown): StoryStep | null {
  if (!isRecord(value)) return null;
  if (value["kind"] === "next") return { kind: "next" };
  const index = value["index"];
  if (value["kind"] === "choose" && isCount(index)) return { kind: "choose", index };
  return null;
}

/**
 * Builds the save document of the game from the story's record.
 * @param story - the record of the story, as `Story.snapshot` gives it.
 */
export function buildSave(story: StorySnapshot): GameSave {
  return { story };
}

/**
 * Reads a save document. The shape is checked here; the guion checks the scene and the actions when the record
 * is replayed, and that can still refuse the save.
 * @param value - the `state` that the save list read back.
 * @returns the save, or null when the value is not one that this game wrote.
 */
export function parseSave(value: unknown): GameSave | null {
  if (!isRecord(value)) return null;
  const story = value["story"];
  if (!isRecord(story)) return null;
  const scene = story["scene"];
  const vars = readVars(story["vars"]);
  const stored = story["steps"];
  if (typeof scene !== "string" || !LABEL.test(scene) || vars === null || !Array.isArray(stored)) return null;
  const steps: StoryStep[] = [];
  for (const item of stored) {
    const step = readStep(item);
    if (step === null) return null;
    steps.push(step);
  }
  return { story: { scene, vars, steps } };
}
