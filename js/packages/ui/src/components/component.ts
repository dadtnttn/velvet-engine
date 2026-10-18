// The shape that every HUD component has. A component is a small object: an element, a way to
// change what the element shows, and a way to remove itself. There is no framework. The element
// is plain DOM, so it can go in any page, or in the root of a stage.
//
// Conventions shared by all components:
// - `update(props)` merges the props it is given into the current ones. A prop that is left out
//   (or undefined) keeps its value. To clear a text or an optional value, pass null or "".
// - Text is set with textContent, never with innerHTML, so player-written names are safe.
// - Hover, focus and press change colour, brightness or opacity only (see velvet-ui.css).

/**
 * What every component factory returns.
 *
 * @typeParam Props - the props that `update` accepts.
 */
export interface VuComponent<Props> {
  /** The element to place in the page, or in the root of a stage. */
  readonly el: HTMLElement;
  /** Changes what the component shows. Props that are left out keep their values. */
  update(props: Partial<Props>): void;
  /** Removes the element and every listener that the component added. */
  destroy(): void;
}

/**
 * Returns `current` with every defined value of `next` written over it. Undefined values in
 * `next` are skipped, so an omitted prop keeps its value.
 */
export function mergeDefined<Props extends object>(current: Props, next: Partial<Props>): Props {
  const merged: Props = { ...current };
  for (const key of Object.keys(next) as Array<keyof Props>) {
    const value = next[key];
    if (value !== undefined) merged[key] = value as Props[keyof Props];
  }
  return merged;
}
