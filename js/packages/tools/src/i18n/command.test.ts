// The `i18n` command: coverage of the example and of broken copies, and the pseudo-locale it writes.

import { readFile } from "node:fs/promises";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { parseCatalog } from "@velvet/i18n";
import { main } from "../cli";
import { bufferedOutput } from "../output";
import { checkGame } from "../check/check";
import { ENGINE_JS_DIR } from "../paths";
import { copyGame, HELLO_CARDS, readText, tempFolder, writeFiles } from "../test-support";

/** Runs the command and returns its exit code and output. */
async function run(args: readonly string[]): Promise<{ code: number; stdout: string; stderr: string }> {
  const out = bufferedOutput();
  const code = await main(args, out.output);
  return { code, stdout: out.stdout.join("\n"), stderr: out.stderr.join("\n") };
}

/** A copy of the example that can be changed. */
async function copyOfExample(): Promise<string> {
  const folder = await tempFolder();
  await copyGame(HELLO_CARDS, folder);
  return folder;
}

describe("i18n on the example", () => {
  it("reports each language, with no error, and exits 0", async () => {
    const result = await run(["i18n", HELLO_CARDS]);
    expect(result.code).toBe(0);
    expect(result.stdout).toMatch(/^en: 0 missing, \d+ unused, \d+ untranslated$/mu);
    expect(result.stdout).toMatch(/^es \(source\): 0 missing, \d+ unused, 0 untranslated$/mu);
    expect(result.stdout).toMatch(/^hello-cards: ok, 0 errors, \d+ warnings$/mu);
  });

  it("gives the same coverage as JSON, with the source and the languages", async () => {
    const result = await run(["i18n", HELLO_CARDS, "--json"]);
    const parsed = JSON.parse(result.stdout) as { ok: boolean; source: string; languages: { tag: string; isSource: boolean }[] };
    expect(parsed.ok).toBe(true);
    expect(parsed.source).toBe("es");
    expect(parsed.languages.map((language) => [language.tag, language.isSource])).toEqual([
      ["en", false],
      ["es", true],
    ]);
  });
});

describe("i18n on broken copies of the example", () => {
  it("reports a key that a language lacks, as a warning that does not fail the command", async () => {
    const folder = await copyOfExample();
    const en = JSON.parse(await readText(folder, "public/data/locales/en/00_ui.json")) as { entries: Record<string, unknown> };
    delete en.entries["hud.coins"];
    await writeFiles(folder, { "public/data/locales/en/00_ui.json": en });
    const result = await run(["i18n", folder]);
    expect(result.code).toBe(0);
    expect(result.stdout).toContain('public/data/locales/en: entries["hud.coins"]: warning: en has no text for "hud.coins"');
    expect(result.stdout).toMatch(/^en: 1 missing,/mu);
  });

  it("fails when the code asks for an id that the source language does not have", async () => {
    const folder = await copyOfExample();
    await writeFiles(folder, { "src/extra.ts": 'export const label = (texts: { t(key: string): string }) => texts.t("hud.nope");\n' });
    const result = await run(["i18n", folder]);
    expect(result.code).toBe(1);
    expect(result.stdout).toContain('src/extra.ts: (file): error: the code asks for "hud.nope", which the source language (es) does not have');
  });

  it("lists a text the code shows, which the source does not hold, as missing in each other language", async () => {
    const folder = await copyOfExample();
    await writeFiles(folder, { "src/extra.ts": 'export const label = (texts: { t(key: string): string }) => texts.t("Ciao, mundo");\n' });
    const result = await run(["i18n", folder]);
    expect(result.code).toBe(0);
    expect(result.stdout).toContain('public/data/locales/en: entries["Ciao, mundo"]: warning: en has no text for "Ciao, mundo"');
    expect(result.stdout).not.toContain('public/data/locales/es: entries["Ciao, mundo"]');
  });

  it("reports a key that no code, string or data uses, and a text kept as the source wrote it", async () => {
    const folder = await copyOfExample();
    const en = JSON.parse(await readText(folder, "public/data/locales/en/00_ui.json")) as { entries: Record<string, unknown> };
    en.entries["zz.unused"] = "Only in English";
    en.entries["hud.menu"] = "Menú";
    await writeFiles(folder, { "public/data/locales/en/00_ui.json": en });
    const result = await run(["i18n", folder, "--json"]);
    const parsed = JSON.parse(result.stdout) as { languages: { tag: string; unused: string[]; untranslated: string[] }[] };
    const english = parsed.languages.find((language) => language.tag === "en");
    expect(english?.unused).toContain("zz.unused");
    expect(english?.untranslated).toContain("hud.menu");
  });

  it("says so when a game has no locale folder", async () => {
    const folder = await tempFolder();
    await writeFiles(folder, { "velvet-game.json": { id: "plain", title: "Plain" } });
    const result = await run(["i18n", folder]);
    expect(result.code).toBe(0);
    expect(result.stdout).toContain("the game has no locale folder");
  });
});

describe("i18n --pseudo", () => {
  it("writes a pseudo-locale that the i18n package reads, and that the check accepts", async () => {
    const folder = await copyOfExample();
    const result = await run(["i18n", folder, "--pseudo", "en-XA"]);
    expect(result.code).toBe(0);
    expect(result.stdout).toContain("wrote");

    const text = await readText(folder, "public/data/locales/en-XA/00_pseudo.json");
    const parsed = parseCatalog(text, "00_pseudo.json");
    expect(parsed.usable).toBe(true);
    expect(parsed.issues.filter((issue) => issue.severity === "error")).toEqual([]);
    expect(parsed.entries.get("hud.coins")).toBeDefined();
    const coins = parsed.entries.get("hud.coins")?.value;
    expect(typeof coins === "object" && coins !== null && "one" in coins).toBe(true);

    const check = await checkGame(folder);
    expect(check.errors).toBe(0);
    expect(check.counts.languages).toBe(3);
  });

  it("keeps the holes of every text, so the pseudo-locale has no hole error", async () => {
    const folder = await copyOfExample();
    await run(["i18n", folder, "--pseudo", "en-XA"]);
    const parsed = parseCatalog(await readText(folder, "public/data/locales/en-XA/00_pseudo.json"), "00_pseudo.json");
    const day = parsed.entries.get("hud.day");
    expect(day?.holes.has("day")).toBe(true);
  });

  it("writes a text key of the code into the pseudo-locale too", async () => {
    const folder = await copyOfExample();
    await writeFiles(folder, { "src/extra.ts": 'export const label = (texts: { t(key: string): string }) => texts.t("Ciao, mundo");\n' });
    await run(["i18n", folder, "--pseudo", "en-XA", "--json"]);
    const parsed = parseCatalog(await readText(folder, "public/data/locales/en-XA/00_pseudo.json"), "00_pseudo.json");
    expect(parsed.entries.has("Ciao, mundo")).toBe(true);
  });

  it("writes over an earlier pseudo-locale of its own, and reports it as JSON", async () => {
    const folder = await copyOfExample();
    await run(["i18n", folder, "--pseudo", "en-XA"]);
    const again = await run(["i18n", folder, "--pseudo", "en-XA", "--json"]);
    expect(again.code).toBe(0);
    expect(JSON.parse(again.stdout)).toMatchObject({ ok: true, tag: "en-XA", folder: "public/data/locales/en-XA" });
  });

  it("refuses a tag that is not a language tag, the source language, and a real language", async () => {
    const folder = await copyOfExample();
    const bad = await run(["i18n", folder, "--pseudo", "qps-ploc"]);
    expect(bad.code).toBe(1);
    expect(bad.stderr).toContain("canonical case");

    const source = await run(["i18n", folder, "--pseudo", "es"]);
    expect(source.code).toBe(1);
    expect(source.stderr).toContain("is the source language");

    const real = await run(["i18n", folder, "--pseudo", "en"]);
    expect(real.code).toBe(1);
    expect(real.stderr).toContain("is not a pseudo-locale");
    expect(await readFile(join(folder, "public/data/locales/en/00_ui.json"), "utf8")).toContain('"language": "en"');
  });
});

describe("i18n: the texts a language keeps as the source text on purpose", () => {
  /** Lists keys in the `same_as_source` meta of the English catalog of a copy of the example. */
  async function markEnglish(folder: string, keys: string[]): Promise<void> {
    const en = JSON.parse(await readText(folder, "public/data/locales/en/00_ui.json")) as { meta: Record<string, unknown> };
    en.meta["same_as_source"] = keys;
    await writeFiles(folder, { "public/data/locales/en/00_ui.json": en });
  }

  it("reports a text that equals the source as untranslated, until the language marks its key", async () => {
    const folder = await copyOfExample();
    const es = JSON.parse(await readText(folder, "public/data/locales/es/00_ui.json")) as { entries: Record<string, unknown> };
    const en = JSON.parse(await readText(folder, "public/data/locales/en/00_ui.json")) as { entries: Record<string, unknown> };
    en.entries["hud.menu"] = es.entries["hud.menu"];
    await writeFiles(folder, { "public/data/locales/en/00_ui.json": en });
    const flagged = await run(["i18n", folder]);
    expect(flagged.stdout).toContain('public/data/locales/en: entries["hud.menu"]: warning: en keeps the source text of "hud.menu"');

    await markEnglish(folder, ["hud.menu"]);
    const marked = await run(["i18n", folder]);
    expect(marked.code).toBe(0);
    expect(marked.stdout).not.toContain('entries["hud.menu"]');
    expect(marked.stdout).toMatch(/^en: \d+ missing, \d+ unused, \d+ untranslated$/mu);
  });

  it("warns about a marked key that no longer exists, and one whose text no longer equals the source", async () => {
    const folder = await copyOfExample();
    await markEnglish(folder, ["no.such.key", "hud.coins"]);
    const result = await run(["i18n", folder]);
    expect(result.code).toBe(0);
    expect(result.stdout).toContain(
      'public/data/locales/en: meta.same_as_source: warning: en marks "no.such.key" as the source text, but it has no entry for it',
    );
    expect(result.stdout).toContain(
      'public/data/locales/en: meta.same_as_source: warning: en marks "hud.coins" as the source text, but its text is "{count} coins"',
    );
  });

  it("reads the visual novel example with no untranslated text, since its English marks two keys", async () => {
    const result = await run(["i18n", join(ENGINE_JS_DIR, "examples", "hello-vn")]);
    expect(result.code).toBe(0);
    expect(result.stdout).toContain("en: 0 missing, 0 unused, 0 untranslated");
    expect(result.stdout).toContain("hello-vn: ok, 0 errors, 0 warnings");
  });
});
