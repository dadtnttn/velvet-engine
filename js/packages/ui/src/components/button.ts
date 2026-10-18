// A button: a slanted push button with a label and an optional key hint. It is a native <button>,
// so Tab, Enter and Space work without extra code. A toggle button carries aria-pressed.

import { mergeDefined, type VuComponent } from "./component";
import { createElement, releaseFocus } from "./dom";
import { createKeycap, type KeycapProps, type KeycapTextCase } from "./keycap";

/** Props of a button. */
export interface ButtonProps {
  /** The text on the button. */
  readonly label: string;
  /**
   * A key hint drawn after the label, such as "E". Null or an empty string hides it, unless `hotkeyLabel` gives the
   * hint a text of its own.
   */
  readonly hotkey?: string | null;
  /**
   * The text of the key hint, as written, in place of the key's own label: a translated name such as "Espacio". Null
   * or an empty string keeps the label. A mouse key draws the mouse and ignores the text.
   */
  readonly hotkeyLabel?: string | null;
  /**
   * How the text of the key hint is cased. "upper" draws capitals, as a keyboard's keys are drawn, and is the default.
   * "as-is" keeps the case the text is written in, such as "Esc" or "Espacio".
   */
  readonly hotkeyCase?: KeycapTextCase;
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
    const text = current.hotkeyLabel ?? "";
    if (key !== "" || text !== "") {
      // A hint that has only a text takes the text as its name too, since the keycap needs a key.
      const props: KeycapProps = { key: key !== "" ? key : text, text, textCase: current.hotkeyCase ?? "upper" };
      if (hint === null) {
        hint = createKeycap(props);
        el.append(hint.el);
      } else {
        hint.update(props);
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
      releaseFocus(el);
      el.removeEventListener("click", onClick);
      hint?.destroy();
      el.remove();
    },
  };
}
