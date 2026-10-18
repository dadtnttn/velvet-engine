// Tests for Locale: reading the files of one language, and the lookups on them.

import { describe, expect, it } from "vitest";
import { LocaleError, type LocaleErrorCode } from "./format";
import { Locale, resolveKey, type CatalogFile } from "./locale";

/** A catalog file for `language` with the given entries and extra fields. */
function catalog(language: string, entries: unknown, extra: Record<string, unknown> = {}): string {
  return JSON.stringify({ format: "velvet-locale", version: 1, language, ...extra, entries });
}

/** The code of the LocaleError that `action` throws, or undefined when it throws nothing else. */
function errorCode(action: () => unknown): LocaleErrorCode | undefined {
  try {
    action();
  } catch (error) {
    return error instanceof LocaleError ? error.code : undefined;
  }
  return undefined;
}

describe("Locale: the files", () => {
  it("reads the files in name order, whatever order they are given in, and the later file wins", () => {
    const files: CatalogFile[] = [
      { name: "50_patch.json", text: catalog("en", { "menu.start": "Start game" }) },
      { name: "00_menus.json", text: catalog("en", { "menu.start": "Start", "menu.quit": "Quit" }) },
    ];
    const locale = new Locale("en", files);
    expect(locale.t("menu.start")).toBe("Start game");
    expect(locale.t("menu.quit")).toBe("Quit");
    expect(locale.issues).toEqual([]);
  });

  it("skips files whose names start with an underscore, and files that are not JSON", () => {
    const locale = new Locale("en", [
      { name: "_notes.json", text: "not a catalog at all" },
      { name: "README.md", text: "# not JSON either" },
      { name: "00_menus.json", text: catalog("en", { "menu.start": "Start" }) },
    ]);
    expect(locale.keys()).toEqual(["menu.start"]);
    expect(locale.issues).toEqual([]);
  });

  it("reports a file that names another language and ignores only that file", () => {
    const locale = new Locale("en", [
      { name: "00_menus.json", text: catalog("en", { "menu.start": "Start" }) },
      { name: "10_other.json", text: catalog("fr", { "menu.quit": "Quitter" }) },
    ]);
    expect(locale.keys()).toEqual(["menu.start"]);
    expect(locale.issues).toEqual([expect.objectContaining({ severity: "error", code: "language-mismatch", language: "en", file: "10_other.json" })]);
  });

  it("refuses a tag that is not in canonical case, and two files with one name", () => {
    expect(errorCode(() => new Locale("pt-br", []))).toBe("language");
    expect(
      errorCode(() => new Locale("en", [{ name: "a.json", text: catalog("en", {}) }, { name: "a.json", text: catalog("en", {}) }])),
    ).toBe("duplicate-file");
  });

  it("merges the meta blocks of its files, a later field winning", () => {
    const locale = new Locale("he", [
      { name: "00_a.json", text: catalog("he", {}, { meta: { name: "עברית", direction: "ltr", shaping: true } }) },
      { name: "10_b.json", text: catalog("he", {}, { meta: { direction: "rtl" } }) },
    ]);
    expect(locale.meta).toEqual({ name: "עברית", direction: "rtl", shaping: true });
    expect(locale.direction).toBe("rtl");
  });
});

describe("Locale: direction", () => {
  it("uses the meta direction when there is one, else the primary subtag's", () => {
    expect(new Locale("ar-EG", []).direction).toBe("rtl");
    expect(new Locale("en", []).direction).toBe("ltr");
    expect(new Locale("he", [{ name: "a.json", text: catalog("he", {}, { meta: { direction: "ltr" } }) }]).direction).toBe("ltr");
  });
});

describe("Locale: lookups", () => {
  const locale = new Locale("en", [
    {
      name: "a.json",
      text: catalog("en", {
        "shop.buy": "Buy {item} for {gold} coins",
        "shop.discount": "{0}% off",
        "shop.coins": { one: "{count} coin", other: "{count} coins" },
        "shop.escape": "Type {{name}}: {name}",
        "¿Cuánto cuesta?": "How much does it cost?",
        "Quedan {0} faroles": "{0} lanterns are left",
      }),
    },
  ]);

  it("fills named holes from an object and positional holes from a list", () => {
    expect(locale.t("shop.buy", { item: "lantern", gold: 5 })).toBe("Buy lantern for 5 coins");
    expect(locale.t("shop.discount", [10])).toBe("10% off");
  });

  it("writes a doubled brace as a single brace", () => {
    expect(locale.t("shop.escape", { name: "Ana" })).toBe("Type {name}: Ana");
  });

  it("shows a hole that has no argument as it is written, without a report from t", () => {
    expect(locale.t("shop.buy", { item: "lantern" })).toBe("Buy lantern for {gold} coins");
    expect(locale.resolve("shop.buy", undefined, { item: "lantern" })).toEqual({
      text: "Buy lantern for {gold} coins",
      missing: ["gold"],
    });
  });

  it("uses the other form of a plural entry for t, and the form for the count for plural", () => {
    expect(locale.t("shop.coins")).toBe("{count} coins");
    expect(locale.plural("shop.coins", 1)).toBe("1 coin");
    expect(locale.plural("shop.coins", 0)).toBe("0 coins");
  });

  it("shows a key with no entry as itself, read as a template, so a text key shows its text", () => {
    expect(locale.t("¿Cuánto cuesta?")).toBe("How much does it cost?");
    expect(locale.t("Quedan {0} faroles", [7])).toBe("7 lanterns are left");
    expect(locale.t("menu.unknown")).toBe("menu.unknown");
    expect(locale.resolve("menu.unknown", undefined, undefined)).toBeUndefined();
  });

  it("answers has, keys and holesOf from its own entries only", () => {
    expect(locale.has("shop.buy")).toBe(true);
    expect(locale.has("menu.start")).toBe(false);
    expect([...(locale.holesOf("shop.buy") ?? [])].sort()).toEqual(["gold", "item"]);
    expect(locale.holesOf("menu.start")).toBeUndefined();
  });
});

describe("Locale: plural categories", () => {
  it("chooses the category with the rules of the language", () => {
    const pt = new Locale("pt-BR", [{ name: "a.json", text: catalog("pt-BR", { "coins": { one: "{count} moeda", other: "{count} moedas" } }) }]);
    expect(pt.plural("coins", 0)).toBe("0 moeda");
    expect(pt.plural("coins", 2)).toBe("2 moedas");

    const he = new Locale("he", [
      {
        name: "a.json",
        text: catalog("he", { coins: { one: "{count} מטבע", two: "שני מטבעות", many: "{count} מטבעות", other: "{count} מטבעות" } }),
      },
    ]);
    expect(he.plural("coins", 1)).toBe("1 מטבע");
    expect(he.plural("coins", 2)).toBe("שני מטבעות");
    expect(he.plural("coins", 3)).toBe("3 מטבעות");
  });

  it("falls back to other when the entry has no form for the category", () => {
    const locale = new Locale("en", [{ name: "a.json", text: catalog("en", { items: { other: "{count} items" } }) }]);
    expect(locale.plural("items", 1)).toBe("1 items");
  });

  it("uses a plain string for every count", () => {
    const locale = new Locale("en", [{ name: "a.json", text: catalog("en", { items: "{count} things" }) }]);
    expect(locale.plural("items", 1)).toBe("1 things");
    expect(locale.plural("items", 5)).toBe("5 things");
  });

  it("gives the count to the hole {count} even when args has another count", () => {
    const locale = new Locale("en", [{ name: "a.json", text: catalog("en", { items: { one: "{count} item", other: "{count} items" } }) }]);
    expect(locale.plural("items", 2, { count: 99 })).toBe("2 items");
  });
});

describe("Locale: the source reference", () => {
  const source = new Locale("es", [
    {
      name: "a.json",
      text: catalog("es", {
        "shop.buy": "Comprar {item} por {gold} monedas",
        "shop.plain": "Hola",
      }),
    },
  ]);

  it("drops an entry whose holes differ from the source's, and reports it as an error", () => {
    const locale = new Locale(
      "pt-BR",
      [
        {
          name: "a.json",
          text: catalog("pt-BR", { "shop.buy": "Comprar {item} por moedas", "shop.plain": "Oi" }),
        },
      ],
      { reference: source },
    );
    expect(locale.has("shop.buy")).toBe(false);
    expect(locale.has("shop.plain")).toBe(true);
    expect(locale.issues).toEqual([expect.objectContaining({ severity: "error", code: "holes", key: "shop.buy", language: "pt-BR" })]);
  });

  it("checks a text key against the key itself, when the source has no entry for it", () => {
    const text = (key: string, value: string) =>
      new Locale("en", [{ name: "a.json", text: catalog("en", { [key]: value }) }], { reference: source });
    expect(text("Quedan {0} faroles", "{0} lanterns are left").has("Quedan {0} faroles")).toBe(true);
    // "Hola" has no hole in its own text, so a translation that adds one is dropped.
    expect(text("Hola", "Hi {0}").has("Hola")).toBe(false);
    expect(text("Hola", "Hi {0}").issues.map((issue) => issue.code)).toEqual(["holes"]);
  });

  it("keeps an empty text for punctuation whatever the source says", () => {
    const locale = new Locale("en", [{ name: "a.json", text: catalog("en", { "¡": "" }) }], { reference: source });
    expect(locale.t("¡")).toBe("");
    expect(locale.issues).toEqual([]);
  });
});

describe("resolveKey", () => {
  it("shows a key that is not a valid template as written", () => {
    expect(resolveKey("a } b", undefined, undefined)).toEqual({ text: "a } b", missing: [] });
  });
});
