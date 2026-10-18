// velvet-game.json, read the way the Velvet Desktop host reads it, the icon and the release hints, and the data hints.

import { writeFile } from "node:fs/promises";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { discoverGame } from "./discover";
import { manifestFileIssues, NO_RELEASE, pngSize, readManifest } from "./manifest";
import { entryProblem, gameIdProblem, isValidGameId, titleProblem, versionProblem } from "./rules";
import { tempFolder, writeFiles } from "../test-support";

/** The first bytes of a PNG of the given size: the signature and the IHDR chunk, which the checks read. */
function pngHeader(width: number, height: number): Uint8Array {
  const bytes = new Uint8Array(33);
  bytes.set([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0, 0, 0, 13, 0x49, 0x48, 0x44, 0x52], 0);
  const view = new DataView(bytes.buffer);
  view.setUint32(16, width);
  view.setUint32(20, height);
  bytes.set([8, 6, 0, 0, 0], 24);
  return bytes;
}

/** The codes and paths of the issues of a manifest text, for compact assertions. */
function problemsOf(text: string): string[] {
  return readManifest(text).issues.map((issue) => `${issue.path} ${issue.code}`);
}

describe("readManifest", () => {
  it("fills the defaults of a minimal manifest", () => {
    const reading = readManifest(JSON.stringify({ id: "my-game", title: "My Game" }));
    expect(reading.issues).toEqual([]);
    expect(reading.info).toMatchObject({ id: "my-game", title: "My Game", version: "0.1.0", entry: "index.html" });
    expect(reading.info?.hints).toEqual({ roots: [], guion: [], ignore: [], source: undefined });
  });

  it("accepts a byte order mark, as the host does", () => {
    expect(readManifest(`\uFEFF${JSON.stringify({ id: "my-game", title: "My Game" })}`).issues).toEqual([]);
  });

  it("reports text that is not JSON, and a document that is not an object", () => {
    expect(problemsOf("{")).toEqual(["(file) json"]);
    expect(problemsOf("[]")).toEqual(["(file) structure"]);
  });

  it("requires an id and a title that are valid", () => {
    expect(problemsOf(JSON.stringify({ title: "x" }))).toEqual(["id structure"]);
    expect(problemsOf(JSON.stringify({ id: "My Game", title: "x" }))).toEqual(["id id"]);
    expect(problemsOf(JSON.stringify({ id: "my-game" }))).toEqual(["title structure"]);
    expect(problemsOf(JSON.stringify({ id: "my-game", title: "  " }))).toEqual(["title title"]);
  });

  it("refuses fields that the host does not read, and accepts the data hints, which the host ignores", () => {
    expect(problemsOf(JSON.stringify({ id: "my-game", title: "x", description: "y" }))).toEqual([
      "description unknown-field",
    ]);
    const withData = readManifest(JSON.stringify({ id: "my-game", title: "x", data: { source: "es" } }));
    expect(withData.issues).toEqual([]);
    expect(withData.info?.hints.source).toBe("es");
  });

  it("checks the window like the host does", () => {
    const window = (value: unknown): string[] =>
      problemsOf(JSON.stringify({ id: "my-game", title: "x", window: value }));
    expect(window({ width: 0 })).toContain("window.width value");
    expect(window({ width: 800, height: 600, min_width: 900 })).toContain("window.min_width value");
    expect(window({ width: 800, height: 600, min_height: 700 })).toContain("window.min_height value");
    expect(window({ resizable: "yes" })).toContain("window.resizable structure");
    expect(window({ colour: 1 })).toContain("window.colour unknown-field");
    expect(window({ width: 1920, height: 1080, min_width: 960, min_height: 540, resizable: false })).toEqual([]);
  });

  it("reads the data hints, with forward slashes and no way out of the game", () => {
    const reading = readManifest(
      JSON.stringify({
        id: "my-game",
        title: "x",
        data: {
          roots: ["./public/data/"],
          guion: ["public/data/guion"],
          ignore: ["public/data/old"],
          source: "pt-BR",
        },
      }),
    );
    expect(reading.info?.hints).toEqual({
      roots: ["public/data"],
      guion: ["public/data/guion"],
      ignore: ["public/data/old"],
      source: "pt-BR",
    });
    expect(readManifest(JSON.stringify({ id: "a", title: "x", data: { roots: ["../outside"] } })).issues.map((i) => i.path)).toContain("data.roots");
    expect(readManifest(JSON.stringify({ id: "a", title: "x", data: { source: "PT" } })).issues.map((i) => i.path)).toContain("data.source");
  });
});

describe("the rules of an id, a title, a version and an entry", () => {
  it("accepts the ids that the host accepts, and refuses the rest", () => {
    expect(isValidGameId("hello-cards")).toBe(true);
    expect(isValidGameId("0day_2")).toBe(true);
    expect(isValidGameId("-leading")).toBe(false);
    expect(isValidGameId("Upper")).toBe(false);
    expect(isValidGameId("a".repeat(65))).toBe(false);
    expect(gameIdProblem("nul")).toContain("device name");
    expect(gameIdProblem("com3")).toContain("device name");
  });

  it("refuses control characters and long titles", () => {
    expect(titleProblem("a\u0007b")).toContain("control");
    expect(titleProblem("x".repeat(121))).toContain("at most 120");
    expect(titleProblem("Title")).toBeUndefined();
  });

  it("refuses a version that cannot name a release folder", () => {
    expect(versionProblem("1.0 beta")).toContain("letters, digits");
    expect(versionProblem("1.0.0-rc.1+build_2")).toBeUndefined();
  });

  it("refuses an entry that leaves the game folder or uses other separators", () => {
    expect(entryProblem("../index.html")).toBeDefined();
    expect(entryProblem("C:/index.html")).toBeDefined();
    expect(entryProblem("game\\start.html")).toBeDefined();
    expect(entryProblem("game/start.html")).toBeUndefined();
  });
});

describe("the icon and the release hints", () => {
  it("reads the icon and the release hints", () => {
    const reading = readManifest(
      JSON.stringify({
        id: "my-game",
        title: "x",
        icon: "art/icon.png",
        release: { exclude: ["**/*.psd"], include: ["LICENSE"], archive: "zip" },
      }),
    );
    expect(reading.issues).toEqual([]);
    expect(reading.info?.icon).toBe("art/icon.png");
    expect(reading.info?.release).toEqual({ exclude: ["**/*.psd"], include: ["LICENSE"], archive: "zip" });
    expect(readManifest(JSON.stringify({ id: "my-game", title: "x" })).info?.release).toEqual(NO_RELEASE);
  });

  it("refuses the release hints that the release step refuses", () => {
    const problems = (release: unknown): string[] =>
      problemsOf(JSON.stringify({ id: "my-game", title: "x", release }));
    expect(problems({ colour: 1 })).toEqual(["release.colour unknown-field"]);
    expect(problems("zip")).toEqual(["release structure"]);
    expect(problems({ exclude: "**/*.psd" })).toEqual(["release.exclude structure"]);
    expect(problems({ exclude: [""] })).toEqual(["release.exclude value"]);
    expect(problems({ exclude: ["dev\\x"] })).toEqual(["release.exclude value"]);
    expect(problems({ include: ["../LICENSE"] })).toEqual(["release.include value"]);
    expect(problems({ include: ["game/index.html"] })).toEqual(["release.include value"]);
    expect(problems({ include: ["my-game.exe"] })).toEqual(["release.include value"]);
    expect(problems({ archive: "tar" })).toEqual(["release.archive value"]);
    expect(problems(null)).toEqual([]);
  });

  it("refuses an icon whose path leaves the game folder, as the host does", () => {
    expect(problemsOf(JSON.stringify({ id: "my-game", title: "x", icon: "../icon.png" }))).toEqual(["icon icon"]);
    expect(problemsOf(JSON.stringify({ id: "my-game", title: "x", icon: 3 }))).toEqual(["icon structure"]);
  });

  it("reads the size of a PNG from its header, and refuses bytes that are not a PNG", () => {
    expect(pngSize(pngHeader(300, 200))).toEqual({ width: 300, height: 200 });
    expect(pngSize(new TextEncoder().encode("not a picture, and long enough to be read"))).toBeUndefined();
    expect(pngSize(pngHeader(0, 10))).toBeUndefined();
  });

  it("checks the icon and the included files against the game folder", async () => {
    const folder = await tempFolder();
    const check = async (icon: string | undefined, include: string[] = []): Promise<string[]> => {
      const reading = readManifest(
        JSON.stringify({ id: "my-game", title: "x", ...(icon === undefined ? {} : { icon }), release: { include } }),
      );
      if (reading.info === undefined) throw new Error("the manifest reads");
      const issues = await manifestFileIssues(folder, reading.info);
      return issues.map((issue) => `${issue.path} ${issue.code} ${issue.severity}`);
    };
    expect(await check("missing.png")).toEqual(["icon missing-file error"]);
    await writeFile(join(folder, "text.png"), "not a picture at all");
    expect(await check("text.png")).toEqual(["icon not-png error"]);
    await writeFile(join(folder, "wide.png"), pngHeader(300, 200));
    expect(await check("wide.png")).toEqual(["icon not-square error"]);
    await writeFile(join(folder, "small.png"), pngHeader(128, 128));
    expect(await check("small.png")).toEqual(["icon small-icon warning"]);
    await writeFile(join(folder, "icon.png"), pngHeader(256, 256));
    expect(await check("icon.png")).toEqual([]);
    expect(await check(undefined, ["LICENSE"])).toEqual(["release.include missing-file error"]);
    await writeFile(join(folder, "LICENSE"), "license");
    expect(await check(undefined, ["LICENSE"])).toEqual([]);
  });

  it("is part of the discovery of a game folder", async () => {
    const folder = await tempFolder();
    await writeFiles(folder, { "velvet-game.json": { id: "my-game", title: "x", icon: "gone.png" } });
    const discovery = await discoverGame(folder);
    expect(discovery.issues.map((issue) => `${issue.file}: ${issue.path} ${issue.code}`)).toEqual([
      "velvet-game.json: icon missing-file",
    ]);
  });
});
