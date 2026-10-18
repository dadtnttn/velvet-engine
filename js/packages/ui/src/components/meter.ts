// A meter: a horizontal bar that shows a value, such as health or a charge. It can be a
// continuous fill, or a row of cells that light up to the value. It has role "meter", so a
// screen reader gets the name, the minimum, the maximum and the current value.

import { mergeDefined, type VuComponent } from "./component";
import { createElement } from "./dom";

/** The colour of a meter's fill. */
export type MeterTone = "accent" | "gold" | "danger";

/** Props of a meter. */
export interface MeterProps {
  /** The name that a screen reader gives the meter, such as "Health". */
  readonly label: string;
  /** The current value. Values below 0 or above `max` are shown at the end of the bar. */
  readonly value: number;
  /** The value that means full. Default 100. Must be above zero. */
  readonly max?: number;
  /** Draws the bar as this many cells, lit up to the value. Null draws a continuous fill. */
  readonly segments?: number | null;
  /** The colour of the fill. Default "accent". */
  readonly tone?: MeterTone;
}

/** Default maximum when the props give none. */
const DEFAULT_MAX = 100;

/**
 * Creates a meter.
 * @param props - the starting props.
 * @throws RangeError when the value is not finite, the maximum is not above zero, or the
 * segment count is not a whole number of at least 1.
 */
export function createMeter(props: MeterProps): VuComponent<MeterProps> {
  const el = createElement("div", "vu-meter");
  el.setAttribute("role", "meter");
  el.setAttribute("aria-valuemin", "0");
  let current: MeterProps = { ...props };
  validate(current);

  const render = (): void => {
    const max = current.max ?? DEFAULT_MAX;
    const value = Math.min(max, Math.max(0, current.value));
    const fraction = value / max;
    el.setAttribute("aria-label", current.label);
    el.setAttribute("aria-valuemax", String(max));
    el.setAttribute("aria-valuenow", String(value));
    el.setAttribute("data-tone", current.tone ?? "accent");
    const segments = current.segments;
    if (segments !== undefined && segments !== null) {
      const lit = Math.round(fraction * segments);
      const row = createElement("div", "vu-meter-segments");
      for (let index = 0; index < segments; index += 1) {
        const cell = createElement("span", "vu-meter-cell");
        if (index < lit) cell.classList.add("is-lit");
        row.append(cell);
      }
      el.replaceChildren(row);
    } else {
      const fill = createElement("span", "vu-meter-fill");
      fill.style.width = `${fraction * 100}%`;
      el.replaceChildren(fill);
    }
  };
  render();

  return {
    el,
    update(next: Partial<MeterProps>): void {
      const merged = mergeDefined(current, next);
      validate(merged);
      current = merged;
      render();
    },
    destroy(): void {
      el.remove();
    },
  };
}

/** Throws when the props cannot be shown. The state is not changed when this throws. */
function validate(props: MeterProps): void {
  if (!Number.isFinite(props.value)) {
    throw new RangeError(`meter value must be a finite number, got ${props.value}`);
  }
  const max = props.max ?? DEFAULT_MAX;
  if (!Number.isFinite(max) || max <= 0) {
    throw new RangeError(`meter max must be a number above zero, got ${max}`);
  }
  const segments = props.segments;
  if (segments !== undefined && segments !== null && (!Number.isInteger(segments) || segments < 1)) {
    throw new RangeError(`meter segments must be a whole number from 1, got ${segments}`);
  }
}
