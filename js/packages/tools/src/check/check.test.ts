// `check` on the example (which must pass), on broken copies of it (which must fail with located errors),
// and on games built from the shared format fixtures (which must pass until they are broken).

import { readdir, readFile } from "node:fs/promises";
import { join, resolve } from "node:path";
import { describe, expect, it } from "vitest";
import { formatIssue, type Issue } from "../issues";
import { keyFieldsOf } from "../game/keys";
import { ENGINE_JS_DIR } from "../paths";
import { copyGame, HELLO_CARDS, tempFolder, writeFiles } from "../test-support";
import { checkGame } from "./check";

/** The shared fixtures of the format documents (docs/formats/fixtures). */
const FIXTURES: string = resolve(ENGINE_JS_DIR, "..", "docs", "formats", "fixtures");

/** The issues of a result as `file path code` lines, for compact assertions. */
function lines(issues: readonly Issue[]): string[] {
  return issues.map((issue) => `${issue.file} ${issue.path} ${issue.code}`);
}

/** Reads a JSON file of a game folder, for an edit. */
async function readJson(folder: string, path: string): Promise<Record<string, unknown>> {
  return JSON.parse(await readFile(join(folder, path), "utf8")) as Record<string, unknown>;
}

/** A folder with a copy of the example's game, to break in one way per test. */
async function brokenCopy(): Promise<string> {
  const folder = await tempFolder();
  await copyGame(HELLO_CARDS, folder);
  return folder;
}

/** A folder that holds the given fixture files (target path to source path), with a manifest. */
async function fixtureGame(files: Readonly<Record<string, string>>, extra: Readonly<Record<string, unknown>> = {}): Promise<string> {
  const folder = await tempFolder();
  await writeFiles(folder, { "velvet-game.json": { id: "fixture-game", title: "Fixture game" } });
  for (const [target, source] of Object.entries(files)) {
    await writeFiles(folder, { [target]: await readFile(source, "utf8") });
  }
  await writeFiles(folder, extra);
  return folder;
}

/**
 * The two locale catalogs (Spanish and English) that hold every id key of the given JSON files, each id
 * under its own text, so that the keys of the data are all in the source.
 * @param paths - the JSON files whose key fields must exist.
 */
async function localesFor(paths: readonly string[]): Promise<Record<string, unknown>> {
  const entries: Record<string, string> = {};
  for (const path of paths) {
    for (const field of keyFieldsOf(JSON.parse(await readFile(path, "utf8")))) {
      if (/^[a-z][a-z0-9_-]*(?:\.[a-z0-9_-]+)+$/u.test(field.key)) entries[field.key] = field.key;
    }
  }
  const catalog = (language: string): unknown => ({ format: "velvet-locale", version: 1, language, entries });
  return {
    "public/data/locales/es/00_ui.json": catalog("es"),
    "public/data/locales/en/00_ui.json": catalog("en"),
  };
}

describe("check on the example", () => {
  it("passes without an error, and counts each kind of file", async () => {
    const result = await checkGame(HELLO_CARDS);
    expect(result.issues.filter((issue) => issue.severity === "error")).toEqual([]);
    expect(result.errors).toBe(0);
    expect(result.game).toBe("hello-cards");
    expect(result.counts).toEqual({
      catalogs: 1,
      cases: 0,
      stat_schemas: 0,
      characters: 0,
      art_manifests: 0,
      artprep: 0,
      guiones: 2,
      locale_files: 4,
      languages: 2,
    });
  });
});

describe("check on broken copies of the example", () => {
  it("reports a catalog that breaks a rule of velvet-stacks, against its file", async () => {
    const folder = await brokenCopy();
    const catalog = await readJson(folder, "public/data/catalog.json");
    const cards = catalog["cards"] as Record<string, unknown>[];
    cards[1] = { ...cards[1], id: "villager" };
    await writeFiles(folder, { "public/data/catalog.json": catalog });
    const result = await checkGame(folder);
    expect(lines(result.issues)).toContain("public/data/catalog.json (file) duplicate-card");
    expect(result.errors).toBeGreaterThan(0);
  });

  it("reports a translation whose holes differ from its source, against the key", async () => {
    const folder = await brokenCopy();
    const en = await readJson(folder, "public/data/locales/en/00_ui.json");
    (en["entries"] as Record<string, unknown>)["hud.day"] = "Day {date}";
    await writeFiles(folder, { "public/data/locales/en/00_ui.json": en });
    const result = await checkGame(folder);
    const holes = result.issues.find((issue) => issue.code === "holes");
    expect(holes).toMatchObject({ severity: "error", file: "public/data/locales/en", path: 'entries["hud.day"]' });
    expect(holes === undefined ? "" : formatIssue(holes)).toMatch(/^public\/data\/locales\/en: entries\["hud\.day"\]: error: en "hud\.day" uses/u);
  });

  it("reports a data key that the source language does not have, at its path", async () => {
    const folder = await brokenCopy();
    const catalog = await readJson(folder, "public/data/catalog.json");
    const cards = catalog["cards"] as Record<string, unknown>[];
    cards[0] = { ...cards[0], name: "card.villagers" };
    await writeFiles(folder, { "public/data/catalog.json": catalog });
    const result = await checkGame(folder);
    expect(lines(result.issues)).toContain("public/data/catalog.json cards[0].name unknown-key");
  });

  it("reports a guion with a problem at its line, and two guiones with one id", async () => {
    const folder = await brokenCopy();
    await writeFiles(folder, {
      "public/data/guion/broken.md": "---\nid: broken\n---\n\n## inicio\n? ¿Qué?\n",
      "public/data/guion/other/intro.md": "---\nid: intro\n---\n\n## inicio\n**Lupe:** Hola.\n",
    });
    const result = await checkGame(folder);
    const problem = result.issues.find((issue) => issue.file === "public/data/guion/broken.md");
    expect(problem).toMatchObject({ severity: "error", code: "guion" });
    expect(problem?.path).toMatch(/^line \d+$/u);
    expect(lines(result.issues)).toContain("public/data/guion/other/intro.md (file) guion-id");
  });

  it("keeps a missing translation a warning, which does not fail the check", async () => {
    const folder = await brokenCopy();
    const en = await readJson(folder, "public/data/locales/en/00_ui.json");
    delete (en["entries"] as Record<string, unknown>)["hud.day"];
    await writeFiles(folder, { "public/data/locales/en/00_ui.json": en });
    const result = await checkGame(folder);
    expect(result.issues).toEqual([
      expect.objectContaining({ severity: "warning", code: "missing-key", path: 'entries["hud.day"]' }),
    ]);
    expect(result.errors).toBe(0);
  });
});

describe("check on games built from the shared fixtures", () => {
  it("passes a character schema, its sheets and an art manifest that names their art", async () => {
    const schema = join(FIXTURES, "characters", "schema.json");
    const sheetFolder = join(FIXTURES, "characters", "sheets");
    const sheetNames = (await readdir(sheetFolder)).filter((name) => name.endsWith(".json"));
    const sheetPaths = sheetNames.map((name) => join(sheetFolder, name));
    const artIds = new Set<string>();
    for (const path of sheetPaths) {
      const art = (JSON.parse(await readFile(path, "utf8")) as { art: Record<string, string> }).art;
      for (const id of Object.values(art)) artIds.add(id);
    }
    const files: Record<string, string> = { "public/data/characters/schema.json": schema };
    for (const [index, name] of sheetNames.entries()) files[`public/data/characters/sheets/${name}`] = sheetPaths[index] ?? "";
    const folder = await fixtureGame(files, {
      ...(await localesFor([schema, ...sheetPaths])),
      "public/data/art/manifest.json": {
        format: "velvet-art",
        version: 1,
        entries: [...artIds].map((id) => ({ id, kind: id.endsWith("fullbody") ? "fullbody" : "portrait", size: [480, 640] })),
      },
    });
    const result = await checkGame(folder);
    expect(lines(result.issues)).toEqual([]);
    expect(result.counts.characters).toBe(sheetNames.length);
    expect(result.counts.stat_schemas).toBe(1);

    const sheet = await readJson(folder, "public/data/characters/sheets/bram.json");
    sheet["art"] = { portrait: "nobody_portrait" };
    await writeFiles(folder, { "public/data/characters/sheets/bram.json": sheet });
    const broken = await checkGame(folder);
    expect(lines(broken.issues)).toContain("public/data/characters/sheets/bram.json art.portrait unknown-art-id");
  });

  it("passes artprep documents whose shots name identities and art that exist", async () => {
    const folder = await fixtureGame({
      "public/data/artprep/profiles.json": join(FIXTURES, "artprep", "profiles.json"),
      "public/data/artprep/style.json": join(FIXTURES, "artprep", "style.json"),
      "public/data/artprep/shots.json": join(FIXTURES, "artprep", "shots.json"),
      "public/data/artprep/identities/ada.json": join(FIXTURES, "artprep", "identities", "ada.json"),
      "public/data/artprep/identities/bex.json": join(FIXTURES, "artprep", "identities", "bex.json"),
      "public/data/art/manifest.json": join(FIXTURES, "artprep", "manifest.json"),
    });
    const result = await checkGame(folder);
    expect(lines(result.issues)).toEqual([]);
    expect(result.counts.artprep).toBe(5);

    const shots = await readJson(folder, "public/data/artprep/shots.json");
    const list = shots["shots"] as Record<string, unknown>[];
    list[0] = { ...list[0], characters: ["nobody"] };
    await writeFiles(folder, { "public/data/artprep/shots.json": shots });
    const broken = await checkGame(folder);
    expect(lines(broken.issues)).toContain("public/data/artprep/shots.json shots[0].characters[0] unknown-character");
  });

  it("runs the stacks cases against the game's catalog, and reports a case that does not match", async () => {
    const casesFolder = join(FIXTURES, "stacks", "cases");
    const caseNames = (await readdir(casesFolder)).filter((name) => name.endsWith(".json"));
    const files: Record<string, string> = { "public/data/catalog.json": join(FIXTURES, "stacks", "catalog.json") };
    for (const name of caseNames) files[`public/data/cases/${name}`] = join(casesFolder, name);
    const folder = await fixtureGame(files);

    const passing = await checkGame(folder);
    expect(passing.counts.cases).toBe(caseNames.length);
    expect(passing.issues.filter((issue) => issue.severity === "error")).toEqual([]);

    const first = await readJson(folder, "public/data/cases/01_gather_repeats.json");
    first["expect"] = { cards: { no_such_card: 1 } };
    await writeFiles(folder, { "public/data/cases/01_gather_repeats.json": first });
    const failing = await checkGame(folder);
    expect(lines(failing.issues)).toContain("public/data/cases/01_gather_repeats.json expect.cards case-table");
  });

  it("warns, and skips the key checks, when a game has data but no locale catalogs", async () => {
    const folder = await fixtureGame({ "public/data/catalog.json": join(FIXTURES, "stacks", "catalog.json") });
    const result = await checkGame(folder);
    expect(result.errors).toBe(0);
    expect(lines(result.issues)).toEqual(["public/data/catalog.json (file) no-source-locale"]);
  });
});
