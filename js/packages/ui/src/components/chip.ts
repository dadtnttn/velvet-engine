// A chip: a small slanted label, such as an item name, with an optional count in a badge.

import { mergeDefined, type VuComponent } from "./component";
import { createElement } from "./dom";

/** Props of a chip. */
export interface ChipProps {
  /** The text of the chip. */
  readonly label: string;
  /** A number shown in a badge after the label. Null hides the badge. */
  readonly count?: number | null;
}

/**
 * Creates a chip.
 * @param props - the starting props.
 */
export function createChip(props: ChipProps): VuComponent<ChipProps> {
  const el = createElement("span", "vu-chip");
  const label = createElement("span", "vu-chip-label");
  const badge = createElement("span", "vu-chip-count");
  el.append(label, badge);
  let current: ChipProps = { ...props };

  const render = (): void => {
    label.textContent = current.label;
    const count = current.count;
    badge.hidden = count === null || count === undefined;
    badge.textContent = count === null || count === undefined ? "" : String(count);
  };
  render();

  return {
    el,
    update(next: Partial<ChipProps>): void {
      current = mergeDefined(current, next);
      render();
    },
    destroy(): void {
      el.remove();
    },
  };
}
