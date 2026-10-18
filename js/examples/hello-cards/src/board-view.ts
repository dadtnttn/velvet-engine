// The board view: the cards and the progress bars of the table, drawn with DOM elements inside the
// stage. Each frame the view reads the board's snapshots and places every element. It keeps no game
// state of its own, so a loaded game is drawn from its snapshots alone.

import type { Board } from "@velvet/cards";
import { createMeter, type MeterProps, type VuComponent } from "@velvet/ui";
import { FADE_MS } from "./config";
import { createIcon } from "./icons";
import { JOB_BAR, cardTopOffset, stackBodyHeight, toPixels } from "./layout";

/** The texts that the view draws: card names, kinds, recipe names, and the label of the bars. */
export interface ViewLabels {
  /** The name of a card definition, as the player reads it. */
  readonly cardName: (def: string) => string;
  /** The kind of a card definition (`character`, `zone`...). It picks the colour of the card's header. */
  readonly cardKind: (def: string) => string;
  /** The name of a recipe, shown on its progress bar. */
  readonly recipeName: (recipe: string) => string;
  /** The label that a screen reader gives every progress bar. */
  readonly progressLabel: string;
}

/** A card element that a drop may land on, with the layer it is drawn on. */
export interface CardHit {
  readonly card: number;
  readonly el: HTMLElement;
  readonly z: number;
}

/** The layer of a stack that is not being dragged. Each card in a stack adds its index to it. */
const STACK_LAYER = 10;

/** The layer of the stack being dragged: above every other stack. */
const DRAG_LAYER = 1000;

/** A progress bar on the table, and the recipe it shows. */
interface BarEntry {
  readonly meter: VuComponent<MeterProps>;
  recipe: string;
}

/**
 * The table's view. Create it inside the stage's root; the game gives it the board and the labels
 * each frame through `render`, and the effects through `markNew`, `fadeOut` and `setTarget`.
 */
export class BoardView {
  /** The element that holds every card and bar. It takes the pointer presses of the drag controller. */
  readonly table: HTMLElement;

  private readonly cards = new Map<number, HTMLElement>();
  private readonly bars = new Map<number, BarEntry>();
  private readonly arriving = new Set<number>();
  private hits: CardHit[] = [];
  private target: number | null = null;

  /** @param parent - the element that the table is appended to (the stage's root). */
  constructor(parent: HTMLElement) {
    this.table = document.createElement("div");
    this.table.className = "hc-table";
    this.table.setAttribute("role", "group");
    parent.append(this.table);
  }

  /**
   * Draws the table from the board's snapshots: every card in its stack, and a progress bar under
   * each stack that runs a job. Elements of cards that left the table are removed.
   *
   * @param board - the board to draw.
   * @param labels - the texts to draw with.
   * @param dragged - a card of the stack being dragged, or null. That stack is drawn above the others.
   */
  render(board: Board, labels: ViewLabels, dragged: number | null): void {
    const defs = new Map<number, string>();
    for (const card of board.cards()) defs.set(card.id, card.def);

    const seenCards = new Set<number>();
    const seenBars = new Set<number>();
    const hits: CardHit[] = [];
    for (const stack of board.stacks()) {
      const count = stack.cards.length;
      const isDragged = dragged !== null && stack.cards.includes(dragged);
      const layer = isDragged ? DRAG_LAYER : STACK_LAYER;
      stack.cards.forEach((card, index) => {
        const el = this.elementFor(card, defs.get(card) ?? "", labels);
        const z = layer + index;
        el.style.left = `${toPixels(stack.x)}px`;
        el.style.top = `${toPixels(stack.y) + cardTopOffset(index, count)}px`;
        el.style.zIndex = String(z);
        el.classList.toggle("is-dragging", isDragged);
        el.classList.toggle("is-target", card === this.target);
        seenCards.add(card);
        hits.push({ card, el, z });
      });

      const bottom = stack.cards[0];
      if (stack.job !== null && bottom !== undefined) {
        const bar = this.barFor(stack.id, stack.job.recipe, labels);
        bar.meter.update({ value: board.progress(bottom) ?? 0 });
        bar.meter.el.style.left = `${toPixels(stack.x)}px`;
        bar.meter.el.style.top = `${toPixels(stack.y) + stackBodyHeight(count) + JOB_BAR.gap}px`;
        seenBars.add(stack.id);
      }
    }

    for (const [card, el] of this.cards) {
      if (seenCards.has(card)) continue;
      el.remove();
      this.cards.delete(card);
    }
    for (const [stack, bar] of this.bars) {
      if (seenBars.has(stack)) continue;
      bar.meter.destroy();
      this.bars.delete(stack);
    }
    this.hits = hits.sort((a, b) => a.z - b.z);
  }

  /** Marks a card that has just appeared, so that it pops in. The mark waits if the card is not drawn yet. */
  markNew(card: number): void {
    const el = this.cards.get(card);
    if (el !== undefined) el.classList.add("is-new");
    else this.arriving.add(card);
  }

  /** Fades a card that the table has consumed, and removes its element once the fade is over. */
  fadeOut(card: number): void {
    const el = this.cards.get(card);
    if (el === undefined) return;
    this.cards.delete(card);
    el.classList.remove("is-target", "is-new");
    el.classList.add("is-leaving");
    window.setTimeout(() => {
      el.remove();
    }, FADE_MS);
  }

  /** Highlights the card under a dragged stack, or clears the highlight with null. */
  setTarget(card: number | null): void {
    if (this.target === card) return;
    if (this.target !== null) this.cards.get(this.target)?.classList.remove("is-target");
    this.target = card;
    if (card !== null) this.cards.get(card)?.classList.add("is-target");
  }

  /** The cards a drop may land on, bottom first, without the cards in `exclude`. */
  hitsExcept(exclude: ReadonlySet<number>): CardHit[] {
    return this.hits.filter((hit) => !exclude.has(hit.card));
  }

  /** Removes every card and bar, so that the next `render` draws them again with new texts. */
  invalidate(): void {
    for (const el of this.cards.values()) el.remove();
    this.cards.clear();
    for (const bar of this.bars.values()) bar.meter.destroy();
    this.bars.clear();
    this.arriving.clear();
    this.hits = [];
    this.target = null;
  }

  /** The element of a card. It is made the first time the card is drawn. */
  private elementFor(card: number, def: string, labels: ViewLabels): HTMLElement {
    const existing = this.cards.get(card);
    if (existing !== undefined) return existing;

    const el = document.createElement("div");
    el.className = "hc-card";
    el.dataset.card = String(card);
    el.dataset.def = def;
    el.dataset.kind = labels.cardKind(def);
    el.setAttribute("role", "group");
    el.setAttribute("aria-label", labels.cardName(def));

    const band = document.createElement("div");
    band.className = "hc-card-name";
    band.textContent = labels.cardName(def);
    const art = document.createElement("div");
    art.className = "hc-card-art";
    const icon = createIcon(def);
    if (icon !== null) art.append(icon);
    el.append(band, art);

    el.addEventListener("animationend", () => {
      el.classList.remove("is-new");
    });
    if (this.arriving.delete(card)) el.classList.add("is-new");
    this.table.append(el);
    this.cards.set(card, el);
    return el;
  }

  /** The progress bar of a stack. It is made the first time the stack runs a job. */
  private barFor(stack: number, recipe: string, labels: ViewLabels): BarEntry {
    let bar = this.bars.get(stack);
    if (bar === undefined) {
      const meter = createMeter({ label: labels.progressLabel, value: 0, max: 1, tone: "gold" });
      meter.el.classList.add("hc-job");
      this.table.append(meter.el);
      bar = { meter, recipe: "" };
      this.bars.set(stack, bar);
    }
    if (bar.recipe !== recipe) {
      bar.recipe = recipe;
      bar.meter.update({ label: `${labels.progressLabel}: ${labels.recipeName(recipe)}` });
    }
    return bar;
  }
}
