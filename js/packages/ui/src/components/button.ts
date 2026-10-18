// A button: a slanted push button with a label and an optional key hint. It is a native <button>,
// so Tab, Enter and Space work without extra code. A toggle button carries aria-pressed.

import { mergeDefined, type VuComponent } from "./component";
import { createElement } from "./dom";
import { createKeycap, type KeycapProps } from "./keycap";

/** Props of a button. */
export interface ButtonProps {
  /** The text on the button. */
  readonly label: string;
  /** A key hint drawn after the label, such as "E". Null or an empty string hides it. */
  readonly hotkey?: string | null;
  /**
   * Toggle state. True or false makes the button a toggle, with aria-pressed. Null makes it a
   * plain button. The game owns the state: the button never flips it by itself.
   */
  readonly pressed?: boolean | null;
  /** True disables the button: it cannot be clicked or reached with the keyboard. */
  readonly disabled?: boolean;
  /** Runs when the button is clicked, or activated from the keyboard. Not called when disabled. */
  readonly onPress?: () => void;
}

/**
 * Creates a button.
 * @param props - the starting props.
 */
export function createButton(props: ButtonProps): VuComponent<ButtonProps> {
  const el = createElement("button", "vu-button");
  el.type = "button";
  const label = createElement("span", "vu-button-label");
  el.append(label);
  let current: ButtonProps = { ...props };
  let hint: VuComponent<KeycapProps> | null = null;

  const render = (): void => {
    label.textContent = current.label;
    el.disabled = current.disabled === true;
    if (typeof current.pressed === "boolean") {
      el.setAttribute("aria-pressed", String(current.pressed));
    } else {
      el.removeAttribute("aria-pressed");
    }
    const key = current.hotkey ?? "";
    if (key !== "") {
      if (hint === null) {
        hint = createKeycap({ key });
        el.append(hint.el);
      } else {
        hint.update({ key });
      }
    } else if (hint !== null) {
      hint.destroy();
      hint = null;
    }
  };

  const onClick = (): void => {
    if (!el.disabled) current.onPress?.();
  };
  el.addEventListener("click", onClick);
  render();

  return {
    el,
    update(next: Partial<ButtonProps>): void {
      current = mergeDefined(current, next);
      render();
    },
    destroy(): void {
      el.removeEventListener("click", onClick);
      hint?.destroy();
      el.remove();
    },
  };
}
