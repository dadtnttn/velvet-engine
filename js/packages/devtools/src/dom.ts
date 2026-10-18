// Small DOM helpers for the overlay. Text always goes in with textContent, never with innerHTML, so a
// value from the game cannot inject markup into the panel.

/** Creates an element, with an optional class and optional text. */
export function element<Tag extends keyof HTMLElementTagNameMap>(
  doc: Document,
  tag: Tag,
  className?: string,
  text?: string,
): HTMLElementTagNameMap[Tag] {
  const node = doc.createElement(tag);
  if (className !== undefined) node.className = className;
  if (text !== undefined) node.textContent = text;
  return node;
}

/** Creates a button that calls `onPress` when it is clicked, or activated from the keyboard. */
export function button(doc: Document, label: string, onPress: () => void, className = "vdt-button"): HTMLButtonElement {
  const node = element(doc, "button", className, label);
  node.type = "button";
  node.addEventListener("click", () => {
    onPress();
  });
  return node;
}
