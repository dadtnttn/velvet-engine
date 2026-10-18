// Runs every case of the velvet-locale conformance suite, docs/formats/fixtures/locale/cases.json.
// The suite is shared with the Rust reader, so each case states its results in the format's own
// terms: keys, texts, the language that answered, missing reports and issue codes.

import { readdirSync, readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import {
  I18n,
  Locale,
  validateCatalog,
  type CatalogFile,
  type LocaleArgs,
  type LocaleIssue,
} from "./index";

const FIXTURES = new URL("../../../../docs/formats/fixtures/locale/", import.meta.url);

/** An issue the case expects. `key` and `language` are compared only when given. */
interface ExpectedIssue {
  readonly severity: "error" | "warning";
  readonly code: string;
  readonly key?: string;
  readonly language?: string;
}

interface FileCase {
  readonly name: string;
  readonly language: string;
  readonly file: string;
  readonly expect: {
    readonly keys: readonly string[];
    readonly issues: readonly ExpectedIssue[];
    readonly meta?: Record<string, unknown>;
  };
}

interface ValidationCase {
  readonly name: string;
  readonly source: string;
  readonly sourceLanguage: string;
  readonly translation: string;
  readonly translationLanguage: string;
  readonly expect: { readonly issues: readonly ExpectedIssue[] };
}

interface LookupCase {
  readonly language: string;
  readonly key: string;
  readonly args?: LocaleArgs;
  readonly count?: number;
  readonly expect: {
    readonly text: string;
    readonly via: string;
    readonly missing?: boolean;
    readonly issues?: readonly string[];
  };
}

interface ScenarioCase {
  readonly name: string;
  readonly source: string;
  readonly fallback: string;
  readonly order?: "reversed";
  readonly languages: Readonly<Record<string, string>>;
  readonly loadIssues: readonly ExpectedIssue[];
  readonly lookups: readonly LookupCase[];
  readonly directions?: readonly { readonly language: string; readonly direction: "ltr" | "rtl" }[];
}

interface Cases {
  readonly files: readonly FileCase[];
  readonly validations: readonly ValidationCase[];
  readonly scenarios: readonly ScenarioCase[];
}

const cases = JSON.parse(readFileSync(new URL("cases.json", FIXTURES), "utf8")) as Cases;

/** Reads the catalog files of a folder, in name order, or in reverse name order. */
function readFolder(folder: string, order: "name" | "reversed" = "name"): CatalogFile[] {
  const dir = new URL(`${folder}/`, FIXTURES);
  const names = readdirSync(dir)
    .filter((name) => name.endsWith(".json"))
    .sort();
  if (order === "reversed") {
    names.reverse();
  }
  return names.map((name) => ({ name, text: readFileSync(new URL(name, dir), "utf8") }));
}

/**
 * Checks that the actual issues are exactly the expected ones: each expected issue matches one
 * actual issue, and no actual issue is left over. Expected issues with a key match first, so a
 * key-less expectation never takes the issue a keyed one needs.
 */
function expectIssues(actual: readonly LocaleIssue[], expected: readonly ExpectedIssue[]): void {
  const left = [...actual];
  const unmatched: string[] = [];
  const ordered = [...expected].sort((a, b) => Number(b.key !== undefined) - Number(a.key !== undefined));
  for (const want of ordered) {
    const index = left.findIndex(
      (got) =>
        got.severity === want.severity &&
        got.code === want.code &&
        (want.key === undefined || got.key === want.key) &&
        (want.language === undefined || got.language === want.language),
    );
    if (index < 0) {
      unmatched.push(describeExpected(want));
    } else {
      left.splice(index, 1);
    }
  }
  expect(unmatched).toEqual([]);
  expect(left.map((issue) => `${issue.severity} ${issue.code} ${issue.key ?? ""}`.trim())).toEqual([]);
}

function describeExpected(issue: ExpectedIssue): string {
  return `${issue.severity} ${issue.code} ${issue.key ?? ""}`.trim();
}

describe("velvet-locale conformance: single files", () => {
  for (const testCase of cases.files) {
    it(testCase.name, () => {
      const text = readFileSync(new URL(testCase.file, FIXTURES), "utf8");
      const name = testCase.file.slice(testCase.file.lastIndexOf("/") + 1);
      const locale = new Locale(testCase.language, [{ name, text }]);
      expect(locale.keys().sort()).toEqual([...testCase.expect.keys].sort());
      expectIssues(locale.issues, testCase.expect.issues);
      if (testCase.expect.meta !== undefined) {
        expect(locale.meta).toEqual(testCase.expect.meta);
      }
    });
  }
});

describe("velvet-locale conformance: validation", () => {
  for (const testCase of cases.validations) {
    it(testCase.name, () => {
      const source = new Locale(testCase.sourceLanguage, readFolder(testCase.source));
      const translation = new Locale(testCase.translationLanguage, readFolder(testCase.translation));
      expectIssues(validateCatalog(source, translation), testCase.expect.issues);
    });
  }
});

describe("velvet-locale conformance: scenarios", () => {
  for (const scenario of cases.scenarios) {
    it(scenario.name, () => {
      const reports: { key: string; language: string }[] = [];
      const issues: LocaleIssue[] = [];
      const i18n = new I18n({
        source: scenario.source,
        fallback: scenario.fallback,
        onMissing: (key, language) => reports.push({ key, language }),
        onIssue: (issue) => issues.push(issue),
      });
      const order = scenario.order === "reversed" ? "reversed" : "name";
      const others = Object.keys(scenario.languages).filter((tag) => tag !== scenario.source);
      for (const tag of [scenario.source, ...others]) {
        const folder = scenario.languages[tag];
        if (folder === undefined) {
          throw new Error(`scenario "${scenario.name}" has no folder for ${tag}`);
        }
        i18n.addLanguage(tag, readFolder(folder, order));
      }
      expectIssues(issues, scenario.loadIssues);
      issues.length = 0;

      for (const lookup of scenario.lookups) {
        i18n.setLanguage(lookup.language);
        const reportsBefore = reports.length;
        const issuesBefore = issues.length;
        const result = i18n.lookup(lookup.key, lookup.args, lookup.count);
        const newReports = reports.slice(reportsBefore);
        const newIssues = issues.slice(issuesBefore);
        const label = `${lookup.language} ${lookup.key}`;

        expect({ label, text: result.text, via: result.language ?? "key" }).toEqual({
          label,
          text: lookup.expect.text,
          via: lookup.expect.via,
        });
        const wantReport = lookup.expect.missing === true;
        expect(newReports).toEqual(wantReport ? [{ key: lookup.key, language: lookup.language }] : []);
        expectIssues(newIssues, (lookup.expect.issues ?? []).map((code) => ({ severity: "warning", code })));

        const shown = lookup.count === undefined ? i18n.t(lookup.key, lookup.args) : i18n.plural(lookup.key, lookup.count, lookup.args);
        expect(shown).toBe(result.text);
      }

      for (const want of scenario.directions ?? []) {
        i18n.setLanguage(want.language);
        expect({ language: want.language, direction: i18n.direction }).toEqual({
          language: want.language,
          direction: want.direction,
        });
      }
    });
  }
});
