// A tooltip: a small label that points at an element. It sits above the element, or below it when
// there is no room above, and it is always kept inside the stage. Its position is in logical
// pixels, so append its element to the stage's root.

import type { Stage } from "../stage";
import { mergeDefined, type VuComponent } from "./component";
import { createElement, uniqueId } from "./dom";

/** Options for {@link createTooltip}. */
export interface TooltipOptions {
  /** The stage that the tooltip lives in. It keeps the tooltip inside and follows its resizes. */
  readonly stage: Stage;
}

/** Props of a tooltip. */
export interface TooltipProps {
  /** The element that the tooltip points at. Null hides the tooltip. */
  readonly anchor?: Element | null;
  /** The text of the tooltip. */
  readonly text?: string;
  /** Where to show it. "above" by default. It moves below the anchor when there is no room above. */
  readonly placement?: "above" | "below";
}

/** Space between the anchor and the tooltip, in logical pixels. */
const GAP = 12;

/**
 * Creates a tooltip. It starts hidden. Set `anchor` to show it.
 * @param options - see {@link TooltipOptions}.
 */
export function createTooltip(options: TooltipOptions): VuComponent<TooltipProps> {
  const { stage } = options;
  const el = createElement("div", "vu-tooltip");
  el.id = uniqueId("tooltip");
  el.setAttribute("role", "tooltip");
  el.style.position = "absolute";
  el.hidden = true;
  let current: TooltipProps = { anchor: null, text: "", placement: "above" };
  /** The anchor that now carries aria-describedby for this tooltip, if any. */
  let described: Element | null = null;

  const place = (anchor: Element): void => {
    const box = stage.rectOf(anchor);
    const width = el.offsetWidth;
    const height = el.offsetHeight;
    const aboveTop = box.top - height - GAP;
    const belowTop = box.top + box.height + GAP;
    let top = current.placement === "below" ? belowTop : aboveTop;
    if (current.placement !== "below" && top < 0) top = belowTop;
    if (current.placement === "below" && top + height > stage.height) top = aboveTop;
    const left = clamp(box.left + box.width / 2 - width / 2, 0, stage.width - width);
    top = clamp(top, 0, stage.height - height);
    el.style.left = `${left}px`;
    el.style.top = `${top}px`;
  };

  const detach = (): void => {
    if (described === null) return;
    if (described.getAttribute("aria-describedby") === el.id) described.removeAttribute("aria-describedby");
    described = null;
  };

  const render = (): void => {
    el.textContent = current.text ?? "";
    const anchor = current.anchor ?? null;
    if (anchor === null) {
      el.hidden = true;
      detach();
      return;
    }
    // The text is set before the tooltip is measured, so its size is the size of the new text.
    el.hidden = false;
    place(anchor);
    if (described !== anchor) {
      detach();
      described = anchor;
    }
    anchor.setAttribute("aria-describedby", el.id);
  };

  const onResize = (): void => {
    const anchor = current.anchor ?? null;
    if (anchor !== null && !el.hidden) place(anchor);
  };
  const unsubscribe = stage.onResize(onResize);
  render();

  return {
    el,
    update(next: Partial<TooltipProps>): void {
      current = mergeDefined(current, next);
      render();
    },
    destroy(): void {
      unsubscribe();
      detach();
      el.remove();
    },
  };
}

/** The value, held between the two bounds. */
function clamp(value: number, low: number, high: number): number {
  return Math.min(Math.max(value, low), Math.max(low, high));
}
