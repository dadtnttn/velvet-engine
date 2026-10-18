// A stat bar: a thin bar that shows a value out of a maximum, such as a need, a loyalty or the progress
// of a job. The fill is drawn with a transform (scaleX), so a change of value moves no layout. The bar has
// role "meter", so a screen reader gets the name, the range and the value. Its optional text is drawn over
// the fill, such as "70 / 100".

import { mergeDefined, type VuComponent } from "./components/component";
import { createElement } from "./components/dom";

/** The colour of a bar's fill. */
export type BarTone = "accent" | "gold" | "danger";

/** Props of a stat bar. */
export interface BarProps {
  /** The name that a screen reader gives the bar, such as "Hunger". */
  readonly label: string;
  /** The current value. A value below 0 draws an empty bar and one above `max` a full bar. */
  readonly value: number;
  /** The value that means full. Default 100. Must be above zero. */
  readonly max?: number;
  /** The colour of the fill. Default "accent". */
  readonly tone?: BarTone;
  /** Text drawn over the bar, such as "70 / 100". Null or an empty string hides it. */
  readonly text?: string | null;
}

/** Default maximum when the props give none. */
const DEFAULT_MAX = 100;

/**
 * Creates a stat bar.
 * @param props - the starting props.
 * @throws RangeError when the value is not finite, or the maximum is not a finite number above zero.
 */
export function createBar(props: BarProps): VuComponent<BarProps> {
  const el = createElement("div", "vu-bar");
  el.setAttribute("role", "meter");
  el.setAttribute("aria-valuemin", "0");
  const fill = createElement("div", "vu-bar-fill");
  const text = createElement("span", "vu-bar-text");
  el.append(fill, text);
  let current: BarProps = { ...props };
  validate(current);
  /** The fill's last ratio, so that a value which does not move the fill writes nothing. */
  let lastRatio = Number.NaN;

  const render = (): void => {
    const max = current.max ?? DEFAULT_MAX;
    const value = Math.min(max, Math.max(0, current.value));
    const ratio = value / max;
    el.setAttribute("aria-label", current.label);
    el.setAttribute("aria-valuemax", String(max));
    el.setAttribute("aria-valuenow", String(value));
    el.setAttribute("data-tone", current.tone ?? "accent");
    if (ratio !== lastRatio) {
      lastRatio = ratio;
      fill.style.transform = `scaleX(${ratio})`;
    }
    const label = current.text ?? "";
    text.textContent = label;
    text.hidden = label === "";
  };
  render();

  return {
    el,
    update(next: Partial<BarProps>): void {
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

/**
 * Throws when the bar cannot show the props. The state is not changed when this throws.
 * @param props - the props to check.
 */
function validate(props: BarProps): void {
  if (!Number.isFinite(props.value)) {
    throw new RangeError(`bar value must be a finite number, got ${props.value}`);
  }
  const max = props.max ?? DEFAULT_MAX;
  if (!Number.isFinite(max) || max <= 0) {
    throw new RangeError(`bar max must be a finite number above zero, got ${max}`);
  }
}
