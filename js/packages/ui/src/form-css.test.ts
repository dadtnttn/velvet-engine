// @vitest-environment happy-dom
// Tests for the parts of velvet-ui.css that the form controls, the keycap case and the settings panel use. Every
// class those modules create has a rule. The fill and the knob move by transform, not by width. The keycap rule
// that keeps the case as written comes after the rule that capitalises, so that it wins.
// css.test.ts already checks that hover, focus and press rules change colour alone, for the whole stylesheet.

import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

const css = readFileSync(`${__dirname}/velvet-ui.css`, "utf8");

/** The stylesheet without its comments, so that a class named in a comment does not count as a rule. */
const rulesText = css.replace(/\/\*[\s\S]*?\*\//g, "");

/** The classes that the form controls, the keycap case and the settings panel create, in the order of the modules. */
const CREATED_CLASSES = [
  // components/slider.ts
  "vu-slider",
  "vu-slider-track",
  "vu-slider-fill",
  "vu-slider-thumb",
  "vu-slider-value",
  // components/toggle.ts
  "vu-toggle",
  "vu-toggle-knob",
  // components/choice.ts
  "vu-choice",
  "vu-choice-arrow",
  "vu-choice-value",
  // components/keycap.ts
  "vu-keycap-as-is",
  // settings-panel.ts
  "vu-settings",
  "vu-settings-section",
  "vu-settings-title",
  "vu-settings-row",
  "vu-settings-label",
  "vu-settings-control",
  "vu-settings-footer",
];

/** The selector that a rule uses for a class, as a regular expression that does not match a longer class name. */
function ruleFor(name: string): RegExp {
  return new RegExp(`\\.${name}(?![\\w-])[^{]*\\{`);
}

/** The body of the first rule whose selector is exactly `selector`, or null. */
function bodyOf(selector: string): string | null {
  const index = rulesText.indexOf(`${selector} {`);
  if (index < 0) return null;
  const open = rulesText.indexOf("{", index);
  const close = rulesText.indexOf("}", open);
  return rulesText.slice(open + 1, close);
}

describe("velvet-ui.css: the classes of the form controls and the settings panel", () => {
  it("has a rule for every class that the new modules create", () => {
    for (const name of CREATED_CLASSES) {
      expect(rulesText, name).toMatch(ruleFor(name));
    }
  });

  it("draws the slider's fill with a transform, and places its thumb with a left offset, never a width", () => {
    expect(bodyOf(".vu-slider-fill")).toContain("transform: scaleX(0)");
    expect(bodyOf(".vu-slider-fill")).not.toMatch(/(?:^|[\s;])width\s*:/);
    expect(bodyOf(".vu-slider-thumb")).toContain("transform: translate(-50%, -50%)");
  });

  it("moves the toggle's knob by a transform when the switch is on, and keeps the track's size", () => {
    expect(rulesText).toMatch(/\.vu-toggle\[aria-checked="true"\] \.vu-toggle-knob\s*\{\s*transform:\s*translateX\(48px\)/);
    expect(bodyOf(".vu-toggle")).toContain("width: 96px");
    expect(bodyOf(".vu-toggle")).toContain("height: 48px");
  });

  it("keeps the case of a keycap as written, and that rule comes after the one that capitalises", () => {
    expect(bodyOf(".vu-keycap-as-is")).toContain("text-transform: none");
    const capitals = rulesText.indexOf("text-transform: uppercase");
    const asIs = rulesText.indexOf(".vu-keycap-as-is {");
    expect(capitals).toBeGreaterThan(-1);
    expect(asIs).toBeGreaterThan(capitals);
  });

  it("keeps each slider, toggle and choice at its size when it is hovered, pressed or focused", () => {
    const states = rulesText.match(/\.vu-(?:slider|toggle|choice)[^{]*:(?:hover|active|focus-visible)[^{]*\{[^}]*\}/g) ?? [];
    expect(states.length).toBeGreaterThanOrEqual(4);
    for (const rule of states) {
      expect(rule, rule).not.toMatch(/(?:^|[\s{;])(?:width|height|padding|margin|top|left|right|bottom|border)\s*:/);
    }
  });
});
