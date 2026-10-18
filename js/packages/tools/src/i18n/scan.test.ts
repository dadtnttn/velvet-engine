// The key scanner: literal calls, keys built at run time, quoted strings, and the text that is not a call.

import { describe, expect, it } from "vitest";
import { scanCode } from "./scan";

describe("scanCode", () => {
  it("finds the literal keys of t(), plural() and lookup() calls, with their file", () => {
    const scan = scanCode([
      {
        file: "src/ui.ts",
        text: 'const a = this.texts.t("menu.start");\nconst b = texts.plural("shop.coins", n);\nlookup("x.y", 1);\n',
      },
    ]);
    expect(scan.calls).toEqual([
      { key: "menu.start", file: "src/ui.ts" },
      { key: "shop.coins", file: "src/ui.ts" },
      { key: "x.y", file: "src/ui.ts" },
    ]);
  });

  it("records the start of a key built at run time, in a call or in a template", () => {
    const scan = scanCode([
      {
        file: "a.ts",
        text: 'texts.t(`recipe.${name}`);\nconst toast = { key: `done.${event.recipe}` };\nt("pause." + mode);\n',
      },
    ]);
    expect(scan.prefixes).toEqual(["done.", "pause.", "recipe."]);
    expect(scan.calls).toEqual([]);
  });

  it("counts a key that appears in any string as used, but makes no call of it", () => {
    const scan = scanCode([{ file: "table.ts", text: 'const toasts = { saved: "toast.saved", text: "Hola" };' }]);
    expect(scan.strings.has("toast.saved")).toBe(true);
    expect(scan.calls).toEqual([]);
  });

  it("ignores a method that only ends in t, and a call inside a comment that has an apostrophe", () => {
    const scan = scanCode([
      {
        file: "b.ts",
        text: "const parts = text.split(\"menu.start\");\n// don't translate this\nt(\"ok.key\");\n",
      },
    ]);
    expect(scan.calls).toEqual([{ key: "ok.key", file: "b.ts" }]);
  });

  it("keeps the text of a template with a hole out of the strings", () => {
    const scan = scanCode([{ file: "c.ts", text: "const label = `Hola ${name}`;" }]);
    expect(scan.strings.has("Hola ${name}")).toBe(false);
    expect(scan.prefixes).toEqual([]);
  });
});
