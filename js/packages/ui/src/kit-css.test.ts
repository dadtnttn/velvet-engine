// @vitest-environment happy-dom
// Tests for the parts of velvet-ui.css that the screen, save, notice, dialogue, history and bar pieces use. Every
// class those modules create has a rule. An element that sets its own display value also honours the hidden
// attribute, so a hidden element stays hidden. The screen fade changes opacity and transform only.
// css.test.ts already checks that hover, focus and press rules change colour alone, for the whole stylesheet.

import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

const css = readFileSync(`${__dirname}/velvet-ui.css`, "utf8");

/** The stylesheet without its comments, so that a class named in a comment does not count as a rule. */
const rulesText = css.replace(/\/\*[\s\S]*?\*\//g, "");

/** The classes that the new modules create, in the order of the modules. */
const CREATED_CLASSES = [
  // screens.ts
  "vu-screens",
  "vu-screen",
  "vu-screen-overlay",
  // save-slots.ts
  "vu-slots",
  "vu-slots-status",
  "vu-slots-list",
  "vu-slot",
  "vu-slot-main",
  "vu-slot-name",
  "vu-slot-title",
  "vu-slot-detail",
  "vu-slot-thumb",
  "vu-slots-footer",
  // notices.ts
  "vu-notices",
  "vu-notice",
  "vu-notice-text",
  "vu-notice-count",
  "vu-notice-actions",
  "vu-history",
  "vu-history-head",
  "vu-history-title",
  "vu-history-list",
  "vu-history-item",
  "vu-history-text",
  "vu-history-count",
  "vu-history-empty",
  // dialogue-box.ts
  "vu-dialogue",
  "vu-dialogue-portrait",
  "vu-dialogue-panel",
  "vu-dialogue-text",
  "vu-dialogue-choices",
  "vu-dialogue-actions",
  "vu-dialogue-backlog",
  "vu-dialogue-log",
  "vu-dialogue-log-item",
  "vu-dialogue-log-name",
  "vu-dialogue-log-text",
  // bar.ts
  "vu-bar",
  "vu-bar-fill",
  "vu-bar-text",
];

/** The selector that a rule uses for a class, as a regular expression that does not match a longer class name. */
function ruleFor(name: string): RegExp {
  return new RegExp(`\\.${name}(?![\\w-])[^{]*\\{`);
}

/** The body of the first rule whose selector ends with `selector`, or null. */
function bodyOf(selector: string): string | null {
  const index = rulesText.indexOf(`${selector} {`);
  if (index < 0) return null;
  const open = rulesText.indexOf("{", index);
  const close = rulesText.indexOf("}", open);
  return rulesText.slice(open + 1, close);
}

describe("velvet-ui.css: the classes of the new pieces", () => {
  it("has a rule for every class that the new modules create", () => {
    for (const name of CREATED_CLASSES) {
      expect(rulesText, name).toMatch(ruleFor(name));
    }
  });

  it("lets the hidden attribute hide every element that sets its own display", () => {
    const withDisplay = [
      "vu-dialogue",
      "vu-dialogue-portrait",
      "vu-dialogue-backlog",
      "vu-history",
      "vu-bar-text",
      "vu-notice-count",
      "vu-history-count",
    ];
    for (const name of withDisplay) {
      expect(rulesText, name).toMatch(new RegExp(`\\.${name}\\[hidden\\]\\s*\\{\\s*display:\\s*none`));
    }
  });
});

describe("velvet-ui.css: the screen fade", () => {
  it("moves a screen by transform and fades it by opacity, for the length the stack sets", () => {
    const transition = bodyOf(".vu-screen");
    expect(transition).not.toBeNull();
    expect(transition).toContain("--vu-screen-ms");
    const phase = bodyOf('.vu-screen[data-phase="enter"],\n.vu-screen[data-phase="leave"]');
    expect(phase).not.toBeNull();
    const properties = (phase ?? "").split(";").map((part) => part.split(":")[0]?.trim()).filter(Boolean);
    expect(properties.sort()).toEqual(["opacity", "transform"]);
  });

  it("gives an overlay a scrim over the screens below it", () => {
    expect(bodyOf(".vu-screen-overlay")).toContain("var(--vu-scrim)");
  });
});

describe("velvet-ui.css: the save rows, notices and bars keep their size in every state", () => {
  it("sets no size or position in the rules of a save row's states", () => {
    const states = rulesText.match(/\.vu-slot[^{]*:(?:hover|active|focus-visible)[^{]*\{[^}]*\}/g) ?? [];
    expect(states.length).toBeGreaterThanOrEqual(3);
    for (const rule of states) {
      expect(rule, rule).not.toMatch(/(?:^|[\s;{])(?:width|height|padding|margin|transform|top|left|right|bottom)\s*:/);
    }
  });

  it("draws a stat bar's fill as a transform, not as a width", () => {
    expect(bodyOf(".vu-bar-fill")).toContain("transform: scaleX(0)");
    expect(bodyOf(".vu-bar-fill")).not.toMatch(/(?:^|[\s;])width\s*:/);
  });
});
