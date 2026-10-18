// Tests for language tags and text direction.

import { describe, expect, it } from "vitest";
import { baseLanguageTag, isLanguageTag, textDirection } from "./tags";

describe("isLanguageTag", () => {
  it("accepts tags in canonical case, with a script and a region when they have one", () => {
    for (const tag of ["en", "pt-BR", "zh-Hans", "zh-Hant-TW", "es-419", "sr-Latn"]) {
      expect(isLanguageTag(tag)).toBe(true);
    }
  });

  it("rejects other casings, variants, underscores and malformed tags", () => {
    for (const tag of ["pt-br", "PT-BR", "pt_BR", "zh-hans", "de-AT-1996", "", "english", "e", "en-", "en-US-x"]) {
      expect(isLanguageTag(tag)).toBe(false);
    }
  });
});

describe("baseLanguageTag", () => {
  it("drops the last subtag", () => {
    expect(baseLanguageTag("pt-BR")).toBe("pt");
    expect(baseLanguageTag("zh-Hant-TW")).toBe("zh-Hant");
    expect(baseLanguageTag("zh-Hant")).toBe("zh");
    expect(baseLanguageTag("es-419")).toBe("es");
  });

  it("has no base for a tag with one subtag", () => {
    expect(baseLanguageTag("en")).toBeUndefined();
  });
});

describe("textDirection", () => {
  it("reads right to left for he, ar, fa and ur, whatever their region", () => {
    for (const tag of ["he", "he-IL", "ar", "ar-EG", "fa", "ur"]) {
      expect(textDirection(tag)).toBe("rtl");
    }
  });

  it("reads left to right for the other languages", () => {
    for (const tag of ["en", "pt-BR", "zh-Hans", "ja", "hi"]) {
      expect(textDirection(tag)).toBe("ltr");
    }
  });
});
