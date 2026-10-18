// The drag controller: picks a card up with the cards above it, follows the pointer, and lets go.
//
// Pressing a card splits its stack (`split`), so the picked card and the cards above it form their
// own stack, which then follows the pointer (`moveStack`). Letting go either drops that stack on the
// card under the pointer (`dropOn`), or places it on the table. A drop that the rules refuse springs
// back to where it came from, with a notice. Pointer positions go through Stage.toLogical, so they
// map exactly onto the 16:9 surface, whatever the window size. Once a drag has started, the pointer
// may leave the surface, so its moves and its release use Stage.toLogicalClamped: the stack follows
// the pointer to the nearest edge instead of freezing where the pointer left.

import { CardsError, type Board } from "@velvet/cards";
import type { Stage } from "@velvet/ui";
import type { BoardView } from "./board-view";
import type { ToastTone } from "./effects";
import { decideDrop, isClick, type DropDecision, type DropTarget } from "./drop";
import { cardTopOffset, placeStack, toPixels, toTable, toUnits, type Point } from "./layout";

/** What the controller needs from the game. */
export interface DragHost {
  /** The stage, for pointer conversion and the rectangles of the cards. */
  readonly stage: Stage;
  /** The view that draws the cards and receives the highlight. */
  readonly view: BoardView;
  /** The board. A load may replace it between two calls, so it is read each time. */
  board(): Board;
  /** True while the player may move cards: no dialogue or menu is open. */
  playable(): boolean;
  /** Shows a notice with an i18n id. */
  say(key: string, tone: ToastTone): void;
  /** Reports a failure that the player cannot act on. It is written to the log. */
  fail(error: unknown): void;
}

/** One drag in progress. */
interface Drag {
  /** The pointer that picked the card up. Other pointers are ignored. */
  readonly pointerId: number;
  /** The card that was pressed. It is the bottom card of the stack being dragged. */
  readonly card: number;
  /** The card that lay under it before the split, or null when it was the bottom card. */
  readonly below: number | null;
  /** The bottom card's place before the split, in board units. */
  readonly origin: Point;
  /** From the picked card's top-left corner to the pointer, in table pixels. */
  readonly grab: Point;
  /** Where the press happened, in logical pixels. */
  readonly start: Point;
  /** True once the pointer has moved far enough to count as a drag rather than a click. */
  moved: boolean;
}

/** Picks cards up and puts them down. Create it with the host, then call `attach`. */
export class DragController {
  private readonly host: DragHost;
  private drag: Drag | null = null;

  /** @param host - the game's side of the drag: the stage, the view, the board and the notices. */
  constructor(host: DragHost) {
    this.host = host;
  }

  /** The card whose stack is being dragged, or null when nothing is. */
  get dragged(): number | null {
    return this.drag?.card ?? null;
  }

  /** Listens for presses on the table. */
  attach(): void {
    this.host.view.table.addEventListener("pointerdown", this.onPress);
  }

  /** Stops listening. A drag in progress springs back, so that no stack is left in mid-air. */
  detach(): void {
    this.host.view.table.removeEventListener("pointerdown", this.onPress);
    const drag = this.drag;
    if (drag !== null) {
      this.end();
      this.springBack(drag);
    }
  }

  /** A press on a card: split its stack and start following the pointer. */
  private readonly onPress = (event: PointerEvent): void => {
    if (this.drag !== null || event.button !== 0 || !this.host.playable()) return;
    const element = event.target instanceof Element ? event.target.closest(".hc-card") : null;
    if (!(element instanceof HTMLElement)) return;
    const card = Number(element.dataset.card);
    const point = this.host.stage.toLogical(event.clientX, event.clientY);
    if (point === null || !Number.isSafeInteger(card)) return;

    const board = this.host.board();
    const stack = board.stacks().find((candidate) => candidate.cards.includes(card));
    if (stack === undefined) return;
    const index = stack.cards.indexOf(card);
    const cardTop = toPixels(stack.y) + cardTopOffset(index, stack.cards.length);
    const cardLeft = toPixels(stack.x);
    const local = toTable(point);
    const below = index > 0 ? (stack.cards[index - 1] ?? null) : null;
    try {
      board.split(card);
      // The picked card keeps its place until the pointer moves, so nothing jumps.
      board.moveStack(card, stack.x, toUnits(cardTop));
    } catch (error) {
      this.host.fail(error);
      return;
    }

    this.drag = {
      pointerId: event.pointerId,
      card,
      below,
      origin: { x: stack.x, y: stack.y },
      grab: { x: local.x - cardLeft, y: local.y - cardTop },
      start: point,
      moved: false,
    };
    window.addEventListener("pointermove", this.onMove);
    window.addEventListener("pointerup", this.onRelease);
    window.addEventListener("pointercancel", this.onCancel);
    event.preventDefault();
  };

  /** The pointer moved: follow it, and highlight the card under it. */
  private readonly onMove = (event: PointerEvent): void => {
    const drag = this.drag;
    if (drag === null || event.pointerId !== drag.pointerId) return;
    // Off the surface (over the bars, or outside the window) the point is clamped to the nearest edge.
    const point = this.host.stage.toLogicalClamped(event.clientX, event.clientY);
    if (!drag.moved) {
      if (isClick(drag.start, point)) return;
      drag.moved = true;
    }
    this.placeAt(drag, point);
    const decision = this.decide(drag, point);
    this.host.view.setTarget(decision.kind === "drop" ? decision.onto : null);
  };

  /** The pointer was released: drop the stack on a card, or place it on the table. */
  private readonly onRelease = (event: PointerEvent): void => {
    const drag = this.drag;
    if (drag === null || event.pointerId !== drag.pointerId) return;
    const point = this.host.stage.toLogicalClamped(event.clientX, event.clientY);
    this.end();
    if (!drag.moved) {
      // A click on a stacked card must not leave it split: it goes back on the card below.
      this.springBack(drag);
      return;
    }
    const decision = this.decide(drag, point);
    if (decision.kind === "place") {
      this.placeAt(drag, point);
      return;
    }
    try {
      this.host.board().dropOn(drag.card, decision.onto);
    } catch (error) {
      this.refuse(error);
      this.springBack(drag);
    }
  };

  /** The browser cancelled the pointer (for example, the window lost it): put the stack back. */
  private readonly onCancel = (event: PointerEvent): void => {
    const drag = this.drag;
    if (drag === null || event.pointerId !== drag.pointerId) return;
    this.end();
    this.springBack(drag);
  };

  /** Stops the listeners and forgets the drag. */
  private end(): void {
    window.removeEventListener("pointermove", this.onMove);
    window.removeEventListener("pointerup", this.onRelease);
    window.removeEventListener("pointercancel", this.onCancel);
    this.drag = null;
    this.host.view.setTarget(null);
  }

  /** Decides what a release at `point` does. The dragged stack's own cards are never targets. */
  private decide(drag: Drag, point: Point): DropDecision {
    const board = this.host.board();
    const own = board.stacks().find((candidate) => candidate.cards.includes(drag.card))?.cards ?? [drag.card];
    const targets: DropTarget[] = this.host.view
      .hitsExcept(new Set(own))
      .map((hit) => ({ card: hit.card, rect: this.host.stage.rectOf(hit.el) }));
    return decideDrop(point, targets);
  }

  /** Moves the dragged stack so that the picked card stays under the pointer, kept on the table. */
  private placeAt(drag: Drag, point: Point): void {
    const board = this.host.board();
    const stack = board.stacks().find((candidate) => candidate.cards.includes(drag.card));
    if (stack === undefined) return;
    const local = toTable(point);
    const spot = placeStack(local.x - drag.grab.x, local.y - drag.grab.y, stack.cards.length);
    board.moveStack(drag.card, spot.x, spot.y);
  }

  /**
   * Puts the dragged stack back where it came from: onto the card that lay under it, which restores
   * the old stack exactly. When that card is gone, the stack goes back to the old place instead.
   */
  private springBack(drag: Drag): void {
    const board = this.host.board();
    if (drag.below !== null) {
      try {
        board.dropOn(drag.card, drag.below);
        return;
      } catch {
        // The card below is gone, or the rules refuse the old height: fall back to the old place.
      }
    }
    try {
      board.moveStack(drag.card, drag.origin.x, drag.origin.y);
    } catch {
      // The picked card itself is gone (a job consumed it during the drag): nothing to put back.
    }
  }

  /** Says why a drop was refused. Errors the player cannot act on are logged. */
  private refuse(error: unknown): void {
    if (error instanceof CardsError) {
      switch (error.code) {
        case "drop-refused":
          this.host.say("toast.refused", "warn");
          return;
        case "stack-full":
          this.host.say("toast.full", "warn");
          return;
        case "same-stack":
          return;
        case "no-such-card":
          this.host.say("toast.back", "warn");
          return;
        default:
          break;
      }
    }
    this.host.fail(error);
  }
}
