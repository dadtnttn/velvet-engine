// Tests for I18n: the fallback chain, the language in use, the listeners and the reports.

import { describe, expect, it } from "vitest";
import { LocaleError, type LocaleErrorCode, type LocaleIssue } from "./format";
import { I18n } from "./i18n";
import type { CatalogFile } from "./locale";

function catalog(language: string, entries: unknown, extra: Record<string, unknown> = {}): CatalogFile {
  return {
    name: "00_all.json",
    text: JSON.stringify({ format: "velvet-locale", version: 1, language, ...extra, entries }),
  };
}

function errorCode(action: () => unknown): LocaleErrorCode | undefined {
  try {
    action();
  } catch (error) {
    return error instanceof LocaleError ? error.code : undefined;
  }
  return undefined;
}

/** An I18n with es (source), en (fallback), pt, pt-BR and he loaded, each with a few entries. */
interface GameHooks {
  onMissing?: (key: string, language: string) => void;
  onIssue?: (issue: LocaleIssue) => void;
}

function game(hooks: GameHooks = {}): I18n {
  const i18n = new I18n({ source: "es", fallback: "en", ...hooks });
  i18n.addLanguage("es", [
    catalog("es", {
      "menu.start": "Empezar",
      "menu.title": "Plaza",
      "shop.buy": "Comprar {item} por {gold} monedas",
      "shop.coins": { one: "{count} moneda", other: "{count} monedas" },
    }),
  ]);
  i18n.addLanguage("en", [
    catalog("en", {
      "menu.start": "Start",
      "shop.buy": "Buy {item} for {gold} coins",
      "shop.coins": { one: "{count} coin", other: "{count} coins" },
    }),
  ]);
  i18n.addLanguage("pt", [catalog("pt", { "menu.title": "Praça" }, { meta: { name: "Português" } })]);
  i18n.addLanguage("pt-BR", [catalog("pt-BR", { "menu.start": "Começar" }, { meta: { direction: "ltr" } })]);
  i18n.addLanguage("he", [catalog("he", { "menu.start": "התחל" }, { meta: { direction: "rtl" } })]);
  return i18n;
}

describe("I18n: the fallback chain", () => {
  it("tries the language, its base, the fallback and the source, without repeats", () => {
    const i18n = game();
    i18n.setLanguage("pt-BR");
    expect(i18n.chain).toEqual(["pt-BR", "pt", "en", "es"]);
    i18n.setLanguage("en");
    expect(i18n.chain).toEqual(["en", "es"]);
    i18n.setLanguage("he");
    expect(i18n.chain).toEqual(["he", "en", "es"]);
  });

  it("stops at the source for the source language and for a variant of it", () => {
    const i18n = game();
    expect(i18n.chain).toEqual(["es"]);
    i18n.addLanguage("es-419", [catalog("es-419", {})]);
    i18n.setLanguage("es-419");
    expect(i18n.chain).toEqual(["es-419", "es"]);
  });

  it("uses the base of a language that has no catalogs of its own", () => {
    const i18n = game();
    i18n.setLanguage("he-IL");
    expect(i18n.chain).toEqual(["he-IL", "he", "en", "es"]);
    expect(i18n.t("menu.start")).toBe("התחל");
    expect(i18n.direction).toBe("rtl");
  });

  it("answers from the first catalog that has the key, and says which one", () => {
    const i18n = game();
    i18n.setLanguage("pt-BR");
    expect(i18n.lookup("menu.title")).toEqual({ text: "Praça", language: "pt" });
    expect(i18n.lookup("menu.start")).toEqual({ text: "Começar", language: "pt-BR" });
    expect(i18n.lookup("shop.buy", { item: "x", gold: 1 })).toEqual({ text: "Buy x for 1 coins", language: "en" });
    expect(i18n.lookup("shop.unknown")).toEqual({ text: "shop.unknown", language: undefined });
  });

  it("answers plurals through the chain, with the categories of the language that answered", () => {
    const i18n = game();
    i18n.setLanguage("pt-BR");
    expect(i18n.plural("shop.coins", 1)).toBe("1 coin");
    expect(i18n.plural("shop.coins", 0)).toBe("0 coins");
  });

  it("reports whether any catalog of the chain has a key", () => {
    const i18n = game();
    i18n.setLanguage("pt-BR");
    expect(i18n.has("menu.title")).toBe(true);
    expect(i18n.has("shop.unknown")).toBe(false);
  });
});

describe("I18n: loading", () => {
  it("refuses a language before the source language is loaded", () => {
    const i18n = new I18n({ source: "es" });
    expect(errorCode(() => i18n.addLanguage("en", [catalog("en", {})]))).toBe("source-missing");
  });

  it("refuses a language that is already loaded", () => {
    const i18n = game();
    expect(errorCode(() => i18n.addLanguage("en", [catalog("en", {})]))).toBe("duplicate-language");
  });

  it("passes the issues of a loaded language to onIssue, and keeps the rest of it", () => {
    const issues: LocaleIssue[] = [];
    const i18n = new I18n({ source: "es", onIssue: (issue) => issues.push(issue) });
    i18n.addLanguage("es", [catalog("es", { "a.b": "Hola" })]);
    i18n.addLanguage("en", [catalog("en", { "a.b": "Hi", "c.d": 4 })]);
    expect(issues.map((issue) => `${issue.language}:${issue.code}:${issue.key}`)).toEqual(["en:value:c.d"]);
    expect(i18n.t("a.b")).toBe("Hola");
    i18n.setLanguage("en");
    expect(i18n.t("a.b")).toBe("Hi");
  });

  it("ignores a translation whose holes differ from the source's, so the lookup falls back", () => {
    const i18n = new I18n({ source: "es", fallback: "en" });
    i18n.addLanguage("es", [catalog("es", { "shop.buy": "Comprar {item} por {gold}" })]);
    i18n.addLanguage("en", [catalog("en", { "shop.buy": "Buy {item} for {gold}" })]);
    i18n.addLanguage("pt-BR", [catalog("pt-BR", { "shop.buy": "Comprar {item}" })]);
    i18n.setLanguage("pt-BR");
    expect(i18n.lookup("shop.buy", { item: "lamp", gold: 3 })).toEqual({ text: "Buy lamp for 3", language: "en" });
  });

  it("loads a language from URLs with a fetch function", async () => {
    const i18n = new I18n({ source: "es" });
    const answers: Record<string, string> = {
      "locales/es/00_a.json": JSON.stringify({ format: "velvet-locale", version: 1, language: "es", entries: { "a.b": "Hola" } }),
      "locales/es/10_b.json": JSON.stringify({ format: "velvet-locale", version: 1, language: "es", entries: { "c.d": "Adiós" } }),
    };
    const fetchFn = async (url: string) => {
      const text = answers[url];
      return { ok: text !== undefined, status: text === undefined ? 404 : 200, text: async () => text ?? "" };
    };
    await i18n.addLanguageFromUrls("es", ["locales/es/00_a.json", "locales/es/10_b.json"], fetchFn);
    expect(i18n.t("c.d")).toBe("Adiós");
    await expect(i18n.addLanguageFromUrls("en", ["locales/en/00_a.json"], fetchFn)).rejects.toMatchObject({ code: "fetch" });
    expect(i18n.languages).toEqual(["es"]);
  });
});

describe("I18n: the language in use", () => {
  it("refuses a malformed tag, and a tag with no catalogs of it or its base", () => {
    const i18n = game();
    expect(errorCode(() => i18n.setLanguage("pt-br"))).toBe("language");
    expect(errorCode(() => i18n.setLanguage("zh-Hans"))).toBe("unknown-language");
    expect(i18n.language).toBe("es");
  });

  it("notifies listeners with the new language, only when it changes, and stops after unsubscribing", () => {
    const i18n = game();
    const seen: string[] = [];
    const stop = i18n.onChange((language) => seen.push(language));
    i18n.setLanguage("en");
    i18n.setLanguage("en");
    i18n.setLanguage("pt-BR");
    stop();
    i18n.setLanguage("es");
    expect(seen).toEqual(["en", "pt-BR"]);
  });

  it("reads the direction of the language in use, from its catalog or its primary subtag", () => {
    const i18n = game();
    i18n.setLanguage("he");
    expect(i18n.direction).toBe("rtl");
    i18n.setLanguage("pt-BR");
    expect(i18n.direction).toBe("ltr");
    expect(i18n.languages).toEqual(["es", "en", "pt", "pt-BR", "he"]);
    expect(i18n.locale("pt")?.meta.name).toBe("Português");
  });
});

describe("I18n: missing keys and missing holes", () => {
  it("reports a missing key once per language, and never for the source language", () => {
    const missing: string[] = [];
    const i18n = game({ onMissing: (key, language) => missing.push(`${language}:${key}`) });
    i18n.setLanguage("en");
    i18n.t("shop.unknown");
    i18n.t("shop.unknown");
    i18n.setLanguage("es");
    i18n.t("shop.unknown");
    i18n.setLanguage("pt-BR");
    i18n.t("shop.unknown");
    expect(missing).toEqual(["en:shop.unknown", "pt-BR:shop.unknown"]);
  });

  it("does not report a key that a fallback catalog has", () => {
    const missing: string[] = [];
    const i18n = game({ onMissing: (key) => missing.push(key) });
    i18n.setLanguage("pt-BR");
    expect(i18n.t("shop.buy", { item: "a", gold: 2 })).toBe("Buy a for 2 coins");
    expect(missing).toEqual([]);
  });

  it("reports a hole with no argument once per language, key and name, and shows it", () => {
    const issues: LocaleIssue[] = [];
    const i18n = game({ onIssue: (issue) => issues.push(issue) });
    i18n.setLanguage("en");
    expect(i18n.t("shop.buy", { item: "lamp" })).toBe("Buy lamp for {gold} coins");
    expect(i18n.t("shop.buy", { item: "lamp" })).toBe("Buy lamp for {gold} coins");
    expect(issues.map((issue) => `${issue.code}:${issue.language}:${issue.key}`)).toEqual(["missing-argument:en:shop.buy"]);
  });
});
