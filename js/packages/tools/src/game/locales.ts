// A game's locale catalogs: one folder per language, read with the i18n package's own Locale class.
// The source language holds every id; the others are checked against it (docs/formats/LOCALE.md).

import { isLanguageTag, Locale, validateCatalog, type CatalogFile, type LocaleIssue } from "@velvet/i18n";
import { makeIssue, WHOLE_FILE, type Issue } from "../issues";
import { compareText, type GameDocument } from "./discover";
import type { DataHints } from "./manifest";
import { MANIFEST_FILE } from "./rules";

/** One language folder: its tag, its folder, and its catalog files. */
export interface LocaleGroup {
  /** The language tag, which is the folder's name. */
  readonly tag: string;
  /** The folder, relative to the game folder. */
  readonly folder: string;
  /** The catalog files, with their names and texts, as the i18n package reads them. */
  readonly files: readonly CatalogFile[];
}

/** The language folders of a game, and the problems found in how they are laid out. */
export interface LocaleLayout {
  /** The language folders, sorted by tag. Two folders with one tag keep the first. */
  readonly groups: readonly LocaleGroup[];
  /** The problems of the layout: a folder name that is not a tag, a source that is missing. */
  readonly issues: Issue[];
  /** The source language's tag, or undefined when the game has no locale folder. */
  readonly source: string | undefined;
}

/** A language, loaded: its locale and the issues that its check found. */
export interface LoadedLocale {
  readonly tag: string;
  readonly folder: string;
  readonly locale: Locale;
  /** The problems of this language, with their files named. Empty for a clean catalog. */
  readonly issues: Issue[];
}

/** Every language of a game, loaded and checked against its source. */
export interface LoadedLocales {
  readonly layout: LocaleLayout;
  /** The languages in tag order. The source is among them. */
  readonly languages: readonly LoadedLocale[];
  /** The source language, when there is one. */
  readonly source: LoadedLocale | undefined;
}

/**
 * Finds the language folders among the locale documents, and picks the source language.
 * @param documents - the game's documents (only the locale ones are used).
 * @param hints - the `data` hints; `source` names the source language.
 */
export function layoutLocales(documents: readonly GameDocument[], hints: DataHints): LocaleLayout {
  const issues: Issue[] = [];
  const byFolder = new Map<string, { tag: string; files: CatalogFile[] }>();
  for (const doc of documents) {
    if (doc.kind !== "locale") continue;
    const tag = doc.folder === "." ? "" : (doc.folder.split("/").at(-1) ?? "");
    const entry = byFolder.get(doc.folder) ?? { tag, files: [] };
    entry.files.push({ name: doc.name, text: doc.text });
    byFolder.set(doc.folder, entry);
  }

  const groups: LocaleGroup[] = [];
  const tags = new Set<string>();
  for (const [folder, entry] of [...byFolder.entries()].sort((a, b) => compareText(a[0], b[0]))) {
    if (!isLanguageTag(entry.tag)) {
      issues.push(
        makeIssue(
          "error",
          folder,
          WHOLE_FILE,
          "language",
          `the folder "${entry.tag}" is not a language tag in canonical case (such as es or pt-BR), so its catalogs are not read`,
        ),
      );
      continue;
    }
    if (tags.has(entry.tag)) {
      issues.push(
        makeIssue("error", folder, WHOLE_FILE, "language", `another folder already holds the language ${entry.tag}`),
      );
      continue;
    }
    tags.add(entry.tag);
    groups.push({ tag: entry.tag, folder, files: entry.files });
  }
  groups.sort((a, b) => compareText(a.tag, b.tag));

  let source: string | undefined;
  if (hints.source !== undefined) {
    if (tags.has(hints.source)) {
      source = hints.source;
    } else {
      issues.push(
        makeIssue("error", MANIFEST_FILE, "data.source", "data-hint", `data.source names ${hints.source}, which has no locale folder`),
      );
    }
  } else if (tags.has("es")) {
    source = "es";
  } else {
    source = groups[0]?.tag;
  }
  return { groups, issues, source };
}

/**
 * Loads every language and checks each against the source. A translation is checked with validateCatalog,
 * so its holes, its missing keys and its unknown keys are reported, with the i18n package's own rules.
 * @param documents - the game's documents.
 * @param hints - the `data` hints.
 */
export function loadLocales(documents: readonly GameDocument[], hints: DataHints): LoadedLocales {
  const layout = layoutLocales(documents, hints);
  const languages: LoadedLocale[] = [];
  let source: LoadedLocale | undefined;

  const sourceGroup = layout.groups.find((group) => group.tag === layout.source);
  if (sourceGroup !== undefined) {
    const locale = new Locale(sourceGroup.tag, sourceGroup.files);
    source = {
      tag: sourceGroup.tag,
      folder: sourceGroup.folder,
      locale,
      issues: localeIssues(locale.issues, sourceGroup.folder),
    };
    languages.push(source);
  }

  for (const group of layout.groups) {
    if (source !== undefined && group.tag === source.tag) continue;
    const locale = new Locale(group.tag, group.files);
    // Without a source there is nothing to compare with, so only the catalog's own problems count.
    const found = source === undefined ? locale.issues : validateCatalog(source.locale, locale);
    languages.push({ tag: group.tag, folder: group.folder, locale, issues: localeIssues(found, group.folder) });
  }
  languages.sort((a, b) => compareText(a.tag, b.tag));
  return { layout, languages, source };
}

/**
 * Turns the i18n package's issues into the tools' issues, with the file's place in the game folder.
 * @param issues - the issues of a locale.
 * @param folder - the language folder, relative to the game folder.
 */
function localeIssues(issues: readonly LocaleIssue[], folder: string): Issue[] {
  return issues.map((issue) => {
    const file = issue.file === undefined ? folder : `${folder}/${issue.file}`;
    const path = issue.key === undefined ? WHOLE_FILE : `entries["${issue.key}"]`;
    return makeIssue(issue.severity, file, path, issue.code, issue.message);
  });
}
