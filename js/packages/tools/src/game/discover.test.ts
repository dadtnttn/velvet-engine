// Where the data is: the conventions of discovery (by format, not by place), the folders that are skipped,
// the tooling files, the byte order marks, and the data hints.

import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { discoverGame } from "./discover";
import { HELLO_CARDS, tempFolder, writeFiles } from "../test-support";

/** A minimal manifest for a test game. */
const MANIFEST = { id: "test-game", title: "Test game" };

/** The documents of a discovery, as `kind file` pairs. */
function documentsOf(discovery: Awaited<ReturnType<typeof discoverGame>>): string[] {
  return discovery.documents.map((doc) => `${doc.kind} ${doc.file}`);
}

describe("discoverGame", () => {
  it("finds the example's documents by their formats", async () => {
    const discovery = await discoverGame(HELLO_CARDS);
    expect(discovery.issues.filter((issue) => issue.severity === "error")).toEqual([]);
    // Documents are listed in file-path order: catalog.json, then the guiones, then the locales.
    expect(documentsOf(discovery)).toEqual([
      "stacks-catalog public/data/catalog.json",
      "guion public/data/guion/intro.en.md",
      "guion public/data/guion/intro.md",
      "locale public/data/locales/en/00_ui.json",
      "locale public/data/locales/en/10_cards.json",
      "locale public/data/locales/es/00_ui.json",
      "locale public/data/locales/es/10_cards.json",
    ]);
  });

  it("reports a missing manifest as the only problem", async () => {
    const folder = await tempFolder();
    const discovery = await discoverGame(folder);
    expect(discovery.info).toBeUndefined();
    expect(discovery.issues.map((issue) => issue.code)).toEqual(["manifest-missing"]);
  });

  it("skips tooling files, node_modules, dist and the Markdown outside a guion folder", async () => {
    const folder = await tempFolder();
    await writeFiles(folder, {
      "velvet-game.json": MANIFEST,
      "package.json": { name: "test-game", scripts: {} },
      "README.md": "# Not a guion\n",
      "docs/notes.md": "# Not a guion either\n",
      "public/data/guion/_draft.md": "---\nid: draft\n---\n",
      "public/data/guion/scene.md": "---\nid: scene\n---\n",
      "public/data/locales/es/_notes.json": { format: "velvet-locale", version: 1, language: "es", entries: {} },
      "node_modules/pkg/data/x.json": { format: "velvet-stacks", version: 1, kind: "catalog", cards: [] },
      "dist/data/y.json": { format: "velvet-stacks", version: 1, kind: "catalog", cards: [] },
      "public/data/save.json": { format: "velvet-stacks", version: 1, kind: "board" },
    });
    const discovery = await discoverGame(folder);
    expect(documentsOf(discovery)).toEqual(["guion public/data/guion/scene.md"]);
    expect(discovery.files.some((file) => file.file.startsWith("node_modules"))).toBe(false);
    expect(discovery.issues).toEqual([]);
  });

  it("reports a broken JSON file in a data folder, and ignores one elsewhere", async () => {
    const folder = await tempFolder();
    await writeFiles(folder, {
      "velvet-game.json": MANIFEST,
      "public/data/catalog.json": "{ not json",
      "tools/config.json": "{ not json either",
    });
    const discovery = await discoverGame(folder);
    expect(discovery.issues.map((issue) => `${issue.file} ${issue.code}`)).toEqual(["public/data/catalog.json json"]);
  });

  it("reports a byte order mark and reads the file without it", async () => {
    const folder = await tempFolder();
    await writeFiles(folder, {
      "velvet-game.json": MANIFEST,
      "public/data/art.json": `\uFEFF${JSON.stringify({ format: "velvet-art", version: 1, entries: [] })}`,
    });
    const discovery = await discoverGame(folder);
    expect(discovery.issues.map((issue) => issue.code)).toEqual(["bom"]);
    expect(documentsOf(discovery)).toEqual(["art-manifest public/data/art.json"]);
  });

  it("warns about a velvet document of a kind it does not read", async () => {
    const folder = await tempFolder();
    await writeFiles(folder, {
      "velvet-game.json": MANIFEST,
      "public/data/mystery.json": { format: "velvet-mystery", version: 1 },
    });
    const discovery = await discoverGame(folder);
    expect(discovery.issues.map((issue) => `${issue.severity} ${issue.code}`)).toEqual(["warning unknown-document"]);
  });

  it("reads the Tiled files by their extension, and a JSON file by its type, and leaves other JSON as data", async () => {
    const folder = await tempFolder();
    await writeFiles(folder, {
      "velvet-game.json": MANIFEST,
      "public/maps/town.tmj": { type: "map" },
      "public/maps/tilesets/forest.tsj": { type: "tileset" },
      "public/maps/old.json": { type: "map", width: 1 },
      "public/data/plain.json": { type: "item", name: "lamp" },
      "public/audio/manifest.json": { format: "velvet-audio", version: 1, sounds: [] },
    });
    const discovery = await discoverGame(folder);
    expect(discovery.issues).toEqual([]);
    expect(documentsOf(discovery)).toEqual([
      "audio-manifest public/audio/manifest.json",
      "tile-map public/maps/old.json",
      "tileset public/maps/tilesets/forest.tsj",
      "tile-map public/maps/town.tmj",
    ]);
  });

  it("reports a broken JSON file in the audio folder, which holds sounds, but not one elsewhere", async () => {
    const folder = await tempFolder();
    await writeFiles(folder, {
      "velvet-game.json": MANIFEST,
      "public/audio/manifest.json": "{ nope",
      "public/tools/notes.json": "{ nope",
    });
    const discovery = await discoverGame(folder);
    expect(discovery.issues.map((issue) => `${issue.severity} ${issue.code} ${issue.file}`)).toEqual([
      "error json public/audio/manifest.json",
    ]);
  });

  it("searches only the hinted roots, and reports a root that does not exist", async () => {
    const folder = await tempFolder();
    await writeFiles(folder, {
      "velvet-game.json": { ...MANIFEST, data: { roots: ["content"] } },
      "content/guion/a.md": "---\nid: a\n---\n",
      "outside/guion/b.md": "---\nid: b\n---\n",
    });
    const discovery = await discoverGame(folder);
    expect(documentsOf(discovery)).toEqual(["guion content/guion/a.md"]);
    expect(discovery.issues).toEqual([]);

    await writeFiles(folder, { "velvet-game.json": { ...MANIFEST, data: { roots: ["missing"] } } });
    const missing = await discoverGame(folder);
    expect(missing.issues.map((issue) => issue.path)).toContain("data.roots");
  });

  it("reads a guion from a hinted folder, and ignores the hinted-out paths", async () => {
    const folder = await tempFolder();
    await writeFiles(folder, {
      "velvet-game.json": { ...MANIFEST, data: { guion: ["story"], ignore: ["old"] } },
      "story/chapter-1.md": "---\nid: one\n---\n",
      "old/chapter-0.md": "---\nid: zero\n---\n",
    });
    const discovery = await discoverGame(folder);
    expect(documentsOf(discovery)).toEqual(["guion story/chapter-1.md"]);
  });

  it("keeps the search inside the game folder", async () => {
    const folder = join(await tempFolder(), "game");
    await writeFiles(folder, { "velvet-game.json": MANIFEST, "public/data/x.json": { format: "velvet-art", version: 1, entries: [] } });
    const discovery = await discoverGame(folder);
    expect(discovery.files.map((file) => file.file)).toEqual(["public/data/x.json", "velvet-game.json"]);
  });
});
