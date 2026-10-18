// Tests for loading catalog files over URLs, with a fake fetch.

import { describe, expect, it } from "vitest";
import { LocaleError } from "./format";
import { fileNameOf, loadLocaleFromUrls, readCatalogUrls, type CatalogFetch } from "./load";

/** A fetch that answers from a map of URL to text; any other URL answers 404. */
function fakeFetch(files: Record<string, string>): CatalogFetch {
  return async (url) => {
    const text = files[url];
    return {
      ok: text !== undefined,
      status: text === undefined ? 404 : 200,
      text: async () => text ?? "",
    };
  };
}

function catalog(language: string, entries: unknown): string {
  return JSON.stringify({ format: "velvet-locale", version: 1, language, entries });
}

describe("fileNameOf", () => {
  it("takes the name at the end of a URL, without its query or fragment", () => {
    expect(fileNameOf("https://game.example/locales/en/10_shop.json")).toBe("10_shop.json");
    expect(fileNameOf("locales/en/10_shop.json?v=3#top")).toBe("10_shop.json");
    expect(fileNameOf("10_shop.json")).toBe("10_shop.json");
  });
});

describe("readCatalogUrls", () => {
  it("fetches each file and keeps its name", async () => {
    const files = await readCatalogUrls(
      ["locales/en/00_menus.json", "locales/en/10_shop.json"],
      fakeFetch({ "locales/en/00_menus.json": "{}", "locales/en/10_shop.json": "{\"a\":1}" }),
    );
    expect(files).toEqual([
      { name: "00_menus.json", text: "{}" },
      { name: "10_shop.json", text: "{\"a\":1}" },
    ]);
  });

  it("refuses a file that does not answer ok, with the fetch error code", async () => {
    await expect(readCatalogUrls(["missing/00.json"], fakeFetch({}))).rejects.toBeInstanceOf(LocaleError);
    await expect(readCatalogUrls(["missing/00.json"], fakeFetch({}))).rejects.toMatchObject({ code: "fetch" });
  });
});

describe("loadLocaleFromUrls", () => {
  it("builds the locale with its files in name order, whatever order the URLs come in", async () => {
    const locale = await loadLocaleFromUrls(
      "en",
      ["locales/en/50_patch.json", "locales/en/00_menus.json"],
      fakeFetch({
        "locales/en/50_patch.json": catalog("en", { "menu.start": "Start game" }),
        "locales/en/00_menus.json": catalog("en", { "menu.start": "Start", "menu.quit": "Quit" }),
      }),
    );
    expect(locale.language).toBe("en");
    expect(locale.t("menu.start")).toBe("Start game");
    expect(locale.t("menu.quit")).toBe("Quit");
  });
});
