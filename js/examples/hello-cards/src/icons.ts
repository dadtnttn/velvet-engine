// Card art: one small SVG drawing per card, built with DOM calls (no innerHTML). Every shape takes one
// of four fill roles (a, b, c, d), and the stylesheet colours the roles, so a card can be restyled
// without touching the drawings. The drawings are decoration: they are hidden from screen readers,
// and the card's name carries the meaning.

const SVG_NS = "http://www.w3.org/2000/svg";

/** A fill role: `a` is the main colour, `b` the body, `c` the dark detail and `d` the accent. */
type Role = "a" | "b" | "c" | "d";

/** One shape of a drawing: its element, its fill role and its attributes. */
interface Shape {
  readonly tag: "circle" | "rect" | "polygon" | "path";
  readonly role: Role;
  readonly attrs: Readonly<Record<string, string>>;
}

/** The drawings, in a 64 by 64 box, by card id. */
const DRAWINGS: Readonly<Record<string, readonly Shape[]>> = {
  villager: [
    { tag: "path", role: "a", attrs: { d: "M14 60 C14 40 22 36 32 36 C42 36 50 40 50 60 Z" } },
    { tag: "circle", role: "b", attrs: { cx: "32", cy: "23", r: "11" } },
    { tag: "path", role: "c", attrs: { d: "M20 19 Q32 4 44 19 Q40 13 32 13 Q24 13 20 19 Z" } },
  ],
  forest: [
    { tag: "polygon", role: "a", attrs: { points: "32,4 52,36 42,36 58,60 6,60 22,36 12,36" } },
    { tag: "rect", role: "c", attrs: { x: "29", y: "58", width: "6", height: "5" } },
  ],
  quarry: [
    { tag: "polygon", role: "b", attrs: { points: "4,60 18,28 30,42 40,18 60,60" } },
    { tag: "polygon", role: "a", attrs: { points: "40,18 48,36 40,32 34,40" } },
  ],
  berry_bush: [
    { tag: "circle", role: "a", attrs: { cx: "32", cy: "36", r: "24" } },
    { tag: "circle", role: "d", attrs: { cx: "20", cy: "30", r: "5" } },
    { tag: "circle", role: "d", attrs: { cx: "38", cy: "24", r: "5" } },
    { tag: "circle", role: "d", attrs: { cx: "42", cy: "42", r: "5" } },
    { tag: "circle", role: "d", attrs: { cx: "24", cy: "46", r: "5" } },
  ],
  market: [
    { tag: "polygon", role: "a", attrs: { points: "4,22 60,22 64,36 0,36" } },
    { tag: "rect", role: "c", attrs: { x: "8", y: "36", width: "4", height: "22" } },
    { tag: "rect", role: "c", attrs: { x: "52", y: "36", width: "4", height: "22" } },
    { tag: "rect", role: "b", attrs: { x: "14", y: "42", width: "36", height: "14" } },
  ],
  house: [
    { tag: "polygon", role: "d", attrs: { points: "32,4 62,30 2,30" } },
    { tag: "rect", role: "b", attrs: { x: "12", y: "30", width: "40", height: "30" } },
    { tag: "rect", role: "c", attrs: { x: "28", y: "42", width: "8", height: "18" } },
  ],
  wood: [
    { tag: "rect", role: "b", attrs: { x: "6", y: "22", width: "50", height: "22", rx: "6" } },
    { tag: "circle", role: "a", attrs: { cx: "52", cy: "33", r: "10" } },
    { tag: "circle", role: "c", attrs: { cx: "52", cy: "33", r: "4" } },
  ],
  plank: [
    { tag: "rect", role: "b", attrs: { x: "8", y: "16", width: "48", height: "32" } },
    { tag: "rect", role: "c", attrs: { x: "8", y: "28", width: "48", height: "2" } },
    { tag: "rect", role: "c", attrs: { x: "8", y: "38", width: "48", height: "2" } },
  ],
  stone: [
    { tag: "polygon", role: "b", attrs: { points: "10,44 16,22 36,14 54,24 58,44 40,56 18,54" } },
    { tag: "polygon", role: "a", attrs: { points: "24,28 36,22 42,30 30,34" } },
  ],
  berries: [
    { tag: "circle", role: "d", attrs: { cx: "22", cy: "28", r: "11" } },
    { tag: "circle", role: "d", attrs: { cx: "42", cy: "30", r: "11" } },
    { tag: "circle", role: "d", attrs: { cx: "32", cy: "46", r: "11" } },
    { tag: "circle", role: "a", attrs: { cx: "19", cy: "24", r: "3" } },
  ],
  coin: [
    { tag: "circle", role: "b", attrs: { cx: "32", cy: "32", r: "26" } },
    { tag: "circle", role: "a", attrs: { cx: "32", cy: "32", r: "17" } },
  ],
};

/**
 * The drawing of a card, or null when the card has none. The SVG is hidden from assistive
 * technology, because the card's name already says what it is.
 *
 * @param def - the card's id in the catalog.
 */
export function createIcon(def: string): SVGSVGElement | null {
  const shapes = DRAWINGS[def];
  if (shapes === undefined) return null;
  const svg = document.createElementNS(SVG_NS, "svg");
  svg.setAttribute("class", "hc-icon");
  svg.setAttribute("viewBox", "0 0 64 64");
  svg.setAttribute("aria-hidden", "true");
  svg.setAttribute("focusable", "false");
  for (const shape of shapes) {
    const node = document.createElementNS(SVG_NS, shape.tag);
    node.setAttribute("class", `hc-fill-${shape.role}`);
    for (const [name, value] of Object.entries(shape.attrs)) {
      node.setAttribute(name, value);
    }
    svg.append(node);
  }
  return svg;
}
