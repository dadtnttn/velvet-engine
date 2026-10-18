// @velvet/i18n: the locale catalogs of Velvet JS games, in the velvet-locale format
// (docs/formats/LOCALE.md). Import from the package root, for example:
//   import { I18n, loadLocaleFromUrls } from "@velvet/i18n";
// Games look texts up with t() and plural(); the package reads the catalog files, fills the
// holes and falls back from one language to the next.

export { LOCALE_FORMAT, LOCALE_VERSION, LocaleError, PLURAL_CATEGORIES } from "./format";
export type {
  Direction,
  EntryValue,
  LocaleArgs,
  LocaleErrorCode,
  LocaleIssue,
  LocaleIssueCode,
  LocaleIssueSeverity,
  LocaleMeta,
  LocaleValue,
  PluralCategory,
  PluralForms,
} from "./format";
export { baseLanguageTag, isLanguageTag, textDirection } from "./tags";
export { parseTemplate, renderTemplate } from "./template";
export type { RenderedTemplate, Template, TemplateParse, TemplatePart } from "./template";
export { parseCatalog } from "./catalog";
export type { ParsedCatalog, ParsedEntry } from "./catalog";
export { Locale } from "./locale";
export type { CatalogFile, LocaleOptions, Resolution } from "./locale";
export { validateCatalog } from "./validate";
export { I18n } from "./i18n";
export type { I18nOptions, LanguageListener, Lookup } from "./i18n";
export { loadLocaleFromUrls, readCatalogUrls } from "./load";
export type { CatalogFetch, CatalogResponse } from "./load";
