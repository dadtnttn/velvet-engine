// @vitest-environment happy-dom
// Tests for velvet-ui.css. Hover, focus and press may change colour, brightness or opacity, but
// never the size, position or padding of an element. The test reads the stylesheet as text and
// checks every rule that depends on a state. It also checks the theme tokens, the reduced-motion
// block, and that every class the components create has a rule.

import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

const css = readFileSync(`${__dirname}/velvet-ui.css`, "utf8");

interface Declaration {
  readonly property: string;
  readonly value: string;
}

interface Rule {
  readonly selector: string;
  readonly declarations: readonly Declaration[];
}

/** The index of the brace that closes the block opened at `open`. */
function closingBrace(text: string, open: number): number {
  let depth = 0;
  for (let index = open; index < text.length; index += 1) {
    const character = text[index];
    if (character === "{") depth += 1;
    if (character === "}") {
      depth -= 1;
      if (depth === 0) return index;
    }
  }
  return text.length;
}

/** The declarations of a rule body, as property and value pairs. */
function declarationsOf(body: string): Declaration[] {
  return body
    .split(";")
    .map((part) => part.trim())
    .filter((part) => part.includes(":"))
    .map((part) => {
      const colon = part.indexOf(":");
      return { property: part.slice(0, colon).trim().toLowerCase(), value: part.slice(colon + 1).trim() };
    });
}

/**
 * Reads the plain rules of a stylesheet. Comments are removed first. The rules inside @media
 * blocks are included, and other at-rules such as @keyframes are skipped.
 */
function readRules(source: string): Rule[] {
  const text = source.replace(/\/\*[\s\S]*?\*\//g, "");
  const rules: Rule[] = [];
  const walk = (block: string): void => {
    let index = 0;
    while (index < block.length) {
      const open = block.indexOf("{", index);
      if (open < 0) break;
      const prelude = block.slice(index, open).trim();
      const close = closingBrace(block, open);
      const body = block.slice(open + 1, close);
      index = close + 1;
      if (prelude.startsWith("@media")) walk(body);
      else if (!prelude.startsWith("@")) rules.push({ selector: prelude, declarations: declarationsOf(body) });
    }
  };
  walk(text);
  return rules;
}

/** The selectors that describe a state: hover, focus, press, toggles, disabled, and the is-* classes. */
const STATE_SELECTOR = /:hover|:focus|:active|\[aria-(?:pressed|selected|disabled)|\.is-[a-z]|:disabled/;

/** The only properties a state may change: colour, brightness and opacity, and the cursor. */
const STATE_PROPERTIES = new Set([
  "background",
  "background-color",
  "color",
  "fill",
  "filter",
  "opacity",
  "cursor",
  "transition",
]);

/** The properties that would move or resize an element, which a state must never change. */
const GEOMETRY_PROPERTIES = /^(?:transform|scale|translate|rotate|width|height|min-|max-|top|left|right|bottom|inset|margin|padding|border(?!-color)|font-size|letter-spacing|line-height|clip-path|display|position|flex|gap|zoom)/;

const rules = readRules(css);

describe("velvet-ui.css theme", () => {
  it("defines the colour, font and size tokens on the root", () => {
    const rootProperties = rules
      .filter((rule) => rule.selector === ".vu-root")
      .flatMap((rule) => rule.declarations.map((declaration) => declaration.property));
    for (const token of ["--vu-ink", "--vu-paper", "--vu-accent", "--vu-font", "--vu-size", "--vu-size-small"]) {
      expect(rootProperties, token).toContain(token);
    }
  });

  it("puts no stage geometry on the root, because the stage writes it inline", () => {
    const rootProperties = rules
      .filter((rule) => rule.selector === ".vu-root")
      .flatMap((rule) => rule.declarations.map((declaration) => declaration.property));
    for (const property of ["width", "height", "left", "top", "transform", "transform-origin", "position"]) {
      expect(rootProperties, property).not.toContain(property);
    }
  });
});

describe("velvet-ui.css states", () => {
  const stateRules = rules.filter((rule) => STATE_SELECTOR.test(rule.selector));

  it("has state rules to check", () => {
    expect(stateRules.length).toBeGreaterThanOrEqual(10);
  });

  it("changes only colour, brightness, opacity and cursor in hover, focus and press rules", () => {
    for (const rule of stateRules) {
      for (const declaration of rule.declarations) {
        expect(STATE_PROPERTIES.has(declaration.property), `${rule.selector} sets ${declaration.property}`).toBe(true);
        expect(GEOMETRY_PROPERTIES.test(declaration.property), `${rule.selector} sets ${declaration.property}`).toBe(
          false,
        );
      }
    }
  });

  it("uses brightness on hover and press, not a size change", () => {
    const hover = stateRules.find((rule) => rule.selector.includes(".vu-button:hover"));
    expect(hover?.declarations.some((declaration) => declaration.property === "filter")).toBe(true);
    const press = stateRules.find((rule) => rule.selector.includes(".vu-button:active"));
    expect(press?.declarations.some((declaration) => declaration.property === "filter")).toBe(true);
  });
});

describe("velvet-ui.css motion and classes", () => {
  it("turns off transitions and animations for reduced motion", () => {
    expect(css).toMatch(/@media \(prefers-reduced-motion: reduce\)\s*\{[\s\S]*transition:\s*none\s*!important/);
    expect(css).toMatch(/@media \(prefers-reduced-motion: reduce\)\s*\{[\s\S]*animation:\s*none\s*!important/);
  });

  it("has a rule for every class that the components create", () => {
    const classes = [
      "vu-viewport",
      "vu-root",
      "vu-plate",
      "vu-plate-title",
      "vu-plate-body",
      "vu-button",
      "vu-button-label",
      "vu-keycap",
      "vu-keycap-mouse",
      "vu-mouse",
      "vu-mouse-body",
      "vu-mouse-part",
      "vu-chip",
      "vu-chip-label",
      "vu-chip-count",
      "vu-meter",
      "vu-meter-fill",
      "vu-meter-segments",
      "vu-meter-cell",
      "vu-tooltip",
      "vu-toasts",
      "vu-toast",
      "vu-modal",
      "vu-modal-dialog",
      "vu-modal-title",
      "vu-modal-body",
      "vu-modal-actions",
      "vu-menu",
      "vu-menu-item",
    ];
    for (const name of classes) {
      expect(css, name).toMatch(new RegExp(`\\.${name}(?![\\w-])`));
    }
  });

  it("keeps the hidden attribute hidden inside the root", () => {
    expect(css).toMatch(/\.vu-root \[hidden\]\s*\{\s*display:\s*none\s*!important/);
  });
});
