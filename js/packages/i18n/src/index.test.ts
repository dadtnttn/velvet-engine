// Tests for the package entry point: the names it exports.

import { describe, expect, it } from "vitest";
import * as i18n from "./index";

describe("@velvet/i18n entry point", () => {
  it("exports the format constants and the classes games use", () => {
    expect(i18n.LOCALE_FORMAT).toBe("velvet-locale");
    expect(i18n.LOCALE_VERSION).toBe(1);
    expect(i18n.PLURAL_CATEGORIES).toEqual(["zero", "one", "two", "few", "many", "other"]);
    expect(typeof i18n.I18n).toBe("function");
    expect(typeof i18n.Locale).toBe("function");
    expect(typeof i18n.LocaleError).toBe("function");
  });

  it("exports the readers, the checks and the helpers", () => {
    expect(typeof i18n.parseCatalog).toBe("function");
    expect(typeof i18n.validateCatalog).toBe("function");
    expect(typeof i18n.parseTemplate).toBe("function");
    expect(typeof i18n.renderTemplate).toBe("function");
    expect(typeof i18n.loadLocaleFromUrls).toBe("function");
    expect(typeof i18n.readCatalogUrls).toBe("function");
    expect(typeof i18n.isLanguageTag).toBe("function");
    expect(typeof i18n.baseLanguageTag).toBe("function");
    expect(typeof i18n.textDirection).toBe("function");
  });

  it("does not export the internal helpers of the modules", () => {
    const names = Object.keys(i18n);
    expect(names).not.toContain("makeIssue");
    expect(names).not.toContain("argumentValue");
    expect(names).not.toContain("resolveKey");
  });
});
