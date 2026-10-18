// Effects: what a board event does on screen. The engine reports what happened (a card appeared, a
// job finished); this module says which effects follow. It is pure, so the mapping is tested.

import type { BoardEvent } from "@velvet/cards";
import { GATHER_RECIPES } from "./config";

/** The colour of a notice: `info` by default, `warn` for a stop, `danger` for a failure. */
export type ToastTone = "info" | "warn" | "danger";

/** A notice to show. `key` is an i18n id, so the text is looked up when the notice shows. */
export interface ToastRequest {
  readonly key: string;
  readonly tone: ToastTone;
}

/** One thing to do on screen. */
export type Effect =
  | { readonly kind: "pop"; readonly card: number }
  | { readonly kind: "fade"; readonly card: number }
  | { readonly kind: "settle"; readonly card: number; readonly x: number; readonly y: number }
  | { readonly kind: "toast"; readonly toast: ToastRequest };

/**
 * The effects of one board event, in the order to apply them.
 * - A new card pops in, and is moved to a free place near where the engine put it (`settle`).
 * - A consumed card fades out.
 * - A finished crafting or trading job says so in a toast; gathering stays quiet.
 * - A job that was stopped says so in a warning.
 */
export function effectsOf(event: BoardEvent): Effect[] {
  switch (event.event) {
    case "card_spawned":
      return [
        { kind: "pop", card: event.card },
        { kind: "settle", card: event.card, x: event.x, y: event.y },
      ];
    case "card_consumed":
      return [{ kind: "fade", card: event.card }];
    case "job_finished":
      if (GATHER_RECIPES.includes(event.recipe)) return [];
      return [{ kind: "toast", toast: { key: `done.${event.recipe}`, tone: "info" } }];
    case "job_cancelled":
      return [{ kind: "toast", toast: { key: "toast.cancelled", tone: "warn" } }];
    case "job_started":
    case "stack_changed":
      return [];
  }
}
