// The DOM helper: the element that shows an art id. A file becomes an <img>, and a placeholder becomes
// an inline <svg>. Both are sized in logical pixels with CSS, so the 16:9 stage scales them with the
// rest of the page.

import { ArtError } from "./error";
import { placeholderTextOf, shortLabel } from "./placeholder";
import type { ArtResolver, ResolvedArt } from "./resolver";
import type { ArtAnchor, ArtSize } from "./types";

/**
 * Sizes an element in logical pixels and sets its pivot. The pivot is the CSS transform origin, so a
 * scale on hover or press grows from the anchor. Only the transform origin is set; the box is not moved.
 */
function applyBox(element: HTMLElement | SVGElement, size: ArtSize, anchor: ArtAnchor): void {
  element.style.width = `${size[0]}px`;
  element.style.height = `${size[1]}px`;
  element.style.transformOrigin = `${anchor[0] * 100}% ${anchor[1] * 100}%`;
}

/** Points an image at a picture, and records which source it came from, for QA tools. */
function showPicture(image: HTMLImageElement, picture: ResolvedArt): void {
  image.src = picture.url;
  image.dataset.artSource = picture.source;
}

/**
 * Turns a placeholder's SVG text into an element of the page, in the SVG namespace. The text is the one
 * in the picture's URL, so the element shows what the resolver resolved, the generic placeholder or a
 * runtime's own.
 * @throws ArtError with code "value" if the text is not well-formed SVG. The generic generator never
 *   makes such text, but a runtime's own placeholder may.
 */
function placeholderElement(picture: ResolvedArt): SVGElement {
  const text = placeholderTextOf(picture.url);
  const parsed = new DOMParser().parseFromString(text, "image/svg+xml");
  const root = parsed.documentElement;
  if (root.localName !== "svg" || parsed.getElementsByTagName("parsererror").length > 0) {
    throw new ArtError("value", `the placeholder of "${picture.id}" is not well-formed SVG`);
  }
  const element = document.importNode(root, true);
  if (!(element instanceof SVGElement)) {
    throw new ArtError("value", `the placeholder of "${picture.id}" did not become an SVG element`);
  }
  applyBox(element, picture.size, picture.anchor);
  element.dataset.artId = picture.id;
  element.dataset.artSource = "placeholder";
  return element;
}

/**
 * The element that shows an art id.
 *
 * A file is shown in an `<img>`, and a placeholder in an inline `<svg>`. Both are sized in logical
 * pixels with CSS width and height. The image's alt text is the short label of the id. If the
 * browser refuses a file, the element moves on to the next format and then to the placeholder: the
 * refused file is marked broken in the resolver, so later lookups skip it too.
 * @param resolver - the resolver of the game's art.
 * @param id - an art id of the manifest.
 * @param variant - a variant that the id declares. Left out, the base picture is shown.
 * @returns an `<img>` for a file, or an `<svg>` for a placeholder.
 * @throws ArtError with code "unknown-id" or "unknown-variant" (see ArtResolver.resolve).
 */
export function artElement(resolver: ArtResolver, id: string, variant?: string): HTMLImageElement | SVGElement {
  const first = resolver.resolve(id, variant);
  if (first.source === "placeholder") return placeholderElement(first);

  const image = document.createElement("img");
  image.alt = shortLabel(first.id, first.kind);
  image.setAttribute("draggable", "false");
  image.dataset.artId = first.id;
  applyBox(image, first.size, first.anchor);
  showPicture(image, first);

  let current = first;
  image.addEventListener("error", () => {
    // A placeholder that fails has no other picture to try.
    if (current.path === null) return;
    resolver.markBroken(current.path);
    current = resolver.resolve(id, variant);
    showPicture(image, current);
  });
  return image;
}
