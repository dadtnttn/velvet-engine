// A plate: a slanted ink panel, the frame of the Persona look. A plate can show a title tab on
// its top edge. Its body holds whatever the game puts in it.

import { mergeDefined, type VuComponent } from "./component";
import { createElement, releaseFocus } from "./dom";

/** Props of a plate. */
export interface PlateProps {
  /** Text on the title tab. Null or an empty string hides the tab. */
  readonly title?: string | null;
}

/** A plate, with the element that holds its content. */
export interface Plate extends VuComponent<PlateProps> {
  /** Where the game puts the plate's content. */
  readonly body: HTMLElement;
}

/**
 * Creates a plate.
 * @param props - the starting props.
 */
export function createPlate(props: PlateProps = {}): Plate {
  const el = createElement("div", "vu-plate");
  const tab = createElement("div", "vu-plate-title");
  const body = createElement("div", "vu-plate-body");
  el.append(tab, body);
  let current: PlateProps = { ...props };

  const render = (): void => {
    const title = current.title ?? "";
    tab.textContent = title;
    tab.hidden = title === "";
    if (title === "") {
      el.removeAttribute("role");
      el.removeAttribute("aria-label");
    } else {
      el.setAttribute("role", "group");
      el.setAttribute("aria-label", title);
    }
  };
  render();

  return {
    el,
    body,
    update(next: Partial<PlateProps>): void {
      current = mergeDefined(current, next);
      render();
    },
    destroy(): void {
      releaseFocus(el);
      el.remove();
    },
  };
}
