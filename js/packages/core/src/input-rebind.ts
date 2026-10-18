// The rules of rebinding, as pure functions over the list of stored bindings: how a request becomes
// a stored binding, when two bindings conflict, what the three conflict policies do, and which
// conflicts a map holds. The action map keeps the list and applies the result; nothing here touches
// the page or the browser.
//
// A conflict is two bindings of different actions, in the same context, on the same control. An
// axis control is the same control only in the same direction (see conflictKeyOf).

import { conflictKeyOf, controlKey, readControl } from "./input-controls";
import type { Binding, Control, ConflictGroup, ConflictPolicy, RebindConflict, RebindOutcome } from "./input";

/** The context of a binding that names none. */
export const DEFAULT_CONTEXT = "default";

/** The longest context name. */
const MAX_CONTEXT_LENGTH = 64;

/** A binding in the form the action map keeps it. */
export interface StoredBinding<Action extends string> {
  readonly action: Action;
  /** The slot of the action that holds the binding. */
  readonly slot: number;
  /** The control, validated and copied. */
  readonly control: Control;
  /** The scale of the binding. */
  readonly scale: number;
  /** The axis component the binding feeds, or null for a digital binding. */
  readonly axis: "x" | "y" | null;
  /** The context the binding belongs to. */
  readonly context: string;
  /** The key that the map uses to find a key, mouse or wheel control that is down. */
  readonly id: string;
  /** The key that conflicts are checked by. */
  readonly conflictKey: string;
}

/** The fields of a stored binding that a caller gives. */
interface StoredFields<Action extends string> {
  readonly action: Action;
  readonly slot: number;
  readonly control: Control;
  readonly scale: number;
  readonly axis: "x" | "y" | null;
  readonly context: string;
}

/**
 * Turns a binding request into a stored binding for `slot`. The control is checked and copied; the
 * scale defaults to 1, the context to "default", and the binding is digital unless it names an axis.
 * @throws RangeError when the control, the scale, the axis component or the context is not valid.
 */
export function makeStored<Action extends string>(binding: Binding<Action>, slot: number): StoredBinding<Action> {
  const scale = binding.scale ?? 1;
  if (!Number.isFinite(scale)) {
    throw new RangeError(`binding scale must be a finite number, got ${scale}`);
  }
  const axis = axisOf(binding.axisComponent);
  const context = binding.context ?? DEFAULT_CONTEXT;
  if (context === "" || context.length > MAX_CONTEXT_LENGTH) {
    throw new RangeError(`context must be 1 to ${MAX_CONTEXT_LENGTH} characters, got ${JSON.stringify(context)}`);
  }
  return storedOf({
    action: binding.action,
    slot,
    control: readControl(binding.control),
    scale,
    axis,
    context,
  });
}

/** The axis component of a binding: null for a digital binding, else "x" or "y". */
function axisOf(value: unknown): "x" | "y" | null {
  if (value === undefined) return null;
  if (value === "x" || value === "y") return value;
  throw new RangeError(`axisComponent must be "x" or "y", got ${String(value)}`);
}

/** The same binding with another control, keeping its action, slot, scale, axis and context. */
export function withControl<Action extends string>(stored: StoredBinding<Action>, control: Control): StoredBinding<Action> {
  return storedOf({
    action: stored.action,
    slot: stored.slot,
    control,
    scale: stored.scale,
    axis: stored.axis,
    context: stored.context,
  });
}

/**
 * Plans a rebinding. `current` is the list of stored bindings and `target` the binding that the
 * request puts into its slot. The plan gives the new list and the outcome:
 * - no conflict: the binding is placed, and the status is "bound";
 * - "refuse": nothing changes, and the status is "refused" with the conflicting slots;
 * - "allow": the binding is placed next to the conflicting ones, and the status is "allowed";
 * - "swap": the conflicting slots take the control that the target slot held before. When that slot
 *   was empty, they are cleared. The status is "swapped".
 */
export function planRebind<Action extends string>(
  current: readonly StoredBinding<Action>[],
  target: StoredBinding<Action>,
  policy: ConflictPolicy,
): { readonly entries: readonly StoredBinding<Action>[]; readonly outcome: RebindOutcome<Action> } {
  const conflicts = current.filter((entry) => isConflict(entry, target));
  const refs: RebindConflict<Action>[] = conflicts.map((entry) => ({ action: entry.action, slot: entry.slot }));
  if (conflicts.length === 0) {
    return { entries: placed(current, target), outcome: { status: "bound", conflicts: refs } };
  }
  if (policy === "refuse") {
    return { entries: current, outcome: { status: "refused", conflicts: refs } };
  }
  if (policy === "allow") {
    return { entries: placed(current, target), outcome: { status: "allowed", conflicts: refs } };
  }
  const previous = current.find((entry) => entry.action === target.action && entry.slot === target.slot);
  const moved = current.flatMap((entry) => {
    if (!conflicts.includes(entry)) return [entry];
    return previous === undefined ? [] : [withControl(entry, previous.control)];
  });
  return { entries: placed(moved, target), outcome: { status: "swapped", conflicts: refs } };
}

/**
 * The groups of conflicting bindings in a list: each group holds bindings of two or more actions
 * that share a context and a control. Groups are returned in the order they first appear.
 */
export function findConflictGroups<Action extends string>(
  entries: readonly StoredBinding<Action>[],
): ConflictGroup<Action>[] {
  const groups = new Map<string, { context: string; control: string; entries: RebindConflict<Action>[]; actions: Set<Action> }>();
  for (const entry of entries) {
    const key = `${entry.context}\u0000${entry.conflictKey}`;
    let group = groups.get(key);
    if (group === undefined) {
      group = { context: entry.context, control: entry.conflictKey, entries: [], actions: new Set() };
      groups.set(key, group);
    }
    group.entries.push({ action: entry.action, slot: entry.slot });
    group.actions.add(entry.action);
  }
  return [...groups.values()]
    .filter((group) => group.actions.size >= 2)
    .map((group) => ({ context: group.context, control: group.control, entries: group.entries }));
}

/** True when two stored bindings are different actions that share a context and a control. */
function isConflict<Action extends string>(entry: StoredBinding<Action>, target: StoredBinding<Action>): boolean {
  return entry.action !== target.action && entry.context === target.context && entry.conflictKey === target.conflictKey;
}

/** The list with `entry` in place of the binding of its slot, or added at the end when the slot is new. */
function placed<Action extends string>(
  list: readonly StoredBinding<Action>[],
  entry: StoredBinding<Action>,
): StoredBinding<Action>[] {
  const index = list.findIndex((candidate) => candidate.action === entry.action && candidate.slot === entry.slot);
  if (index < 0) return [...list, entry];
  return list.map((candidate, position) => (position === index ? entry : candidate));
}

/** A stored binding with its two keys worked out from its fields. */
function storedOf<Action extends string>(fields: StoredFields<Action>): StoredBinding<Action> {
  return {
    ...fields,
    id: controlKey(fields.control),
    conflictKey: conflictKeyOf(fields.control, fields.scale),
  };
}
