// Tests for validateCatalog: the keys a translation lacks or adds, and the holes that differ.

import { describe, expect, it } from "vitest";
import { Locale, type CatalogFile } from "./locale";
import { validateCatalog } from "./validate";

function locale(language: string, entries: unknown): Locale {
  const file: CatalogFile = { name: "00_all.json", text: JSON.stringify({ format: "velvet-locale", version: 1, language, entries }) };
  return new Locale(language, [file]);
}

describe("validateCatalog", () => {
  const source = locale("es", {
    "menu.start": "Empezar",
    "shop.buy": "Comprar {item} por {gold}",
    "shop.coins": { one: "{count} moneda", other: "{count} monedas" },
  });

  it("returns no issues for a complete translation with the same holes", () => {
    const translation = locale("pt-BR", {
      "menu.start": "Começar",
      "shop.buy": "Comprar {gold} por {item}",
      "shop.coins": { one: "{count} moeda", other: "{count} moedas" },
    });
    expect(validateCatalog(source, translation)).toEqual([]);
  });

  it("reports a key the translation lacks as a missing-key warning", () => {
    const translation = locale("pt-BR", { "menu.start": "Começar" });
    const issues = validateCatalog(source, translation);
    expect(issues.map((issue) => `${issue.severity}:${issue.code}:${issue.key}`).sort()).toEqual([
      "warning:missing-key:shop.buy",
      "warning:missing-key:shop.coins",
    ]);
  });

  it("reports a key the source does not have as an unknown-key warning", () => {
    const translation = locale("pt-BR", {
      "menu.start": "Começar",
      "shop.buy": "Comprar {item} por {gold}",
      "shop.coins": { one: "{count} moeda", other: "{count} moedas" },
      "menu.extra": "Extra",
    });
    expect(validateCatalog(source, translation)).toEqual([
      expect.objectContaining({ severity: "warning", code: "unknown-key", key: "menu.extra", language: "pt-BR" }),
    ]);
  });

  it("includes the translation's own load problems, such as a version it does not read", () => {
    const text = JSON.stringify({ format: "velvet-locale", version: 2, language: "pt-BR", entries: {} });
    const translation = new Locale("pt-BR", [{ name: "00_all.json", text }]);
    const codes = validateCatalog(source, translation).map((issue) => issue.code);
    expect(codes).toContain("version");
    expect(codes.filter((code) => code === "missing-key")).toHaveLength(3);
  });

  it("reports a translation that drops or adds a hole as a holes error", () => {
    const translation = locale("pt-BR", {
      "menu.start": "Começar",
      "shop.buy": "Comprar {item} por moedas",
      "shop.coins": { one: "{count} moeda", other: "{total} moedas" },
    });
    expect(validateCatalog(source, translation).map((issue) => `${issue.code}:${issue.key}`).sort()).toEqual([
      "holes:shop.buy",
      "holes:shop.coins",
    ]);
  });
});
