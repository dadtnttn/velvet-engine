// velvet-game.json, read the way the Velvet Desktop host reads it, and the data hints.

import { describe, expect, it } from "vitest";
import { readManifest } from "./manifest";
import { entryProblem, gameIdProblem, isValidGameId, titleProblem, versionProblem } from "./rules";

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
