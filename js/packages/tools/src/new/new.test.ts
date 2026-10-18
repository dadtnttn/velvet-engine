// `new`: the scaffold of a game folder from the template, its links to the engine, its versions, and the
// refusals (a folder that is not empty, an id or title that is not valid, a folder that is not the engine).

import { readFile } from "node:fs/promises";
import { join, resolve } from "node:path";
import { describe, expect, it } from "vitest";
import { main } from "../cli";
import { bufferedOutput } from "../output";
import { readManifest } from "../game/manifest";
import { checkGame } from "../check/check";
import { ENGINE_JS_DIR, TEMPLATE_DIR } from "../paths";
import { exists, HELLO_CARDS, readText, tempFolder, writeFiles } from "../test-support";
import { htmlText, jsonText, renderTemplate, scaffoldGame } from "./scaffold";

/** Scaffolds a game into a fresh folder of a temporary folder, with the engine found from the tools. */
async function scaffold(id = "my-game", title = "My Game"): Promise<string> {
  const folder = join(await tempFolder(), "my-game");
  await scaffoldGame({ folder, id, title, engine: ENGINE_JS_DIR, force: false });
  return folder;
}

/** The dev dependencies of a package.json, by name. */
async function devVersions(path: string): Promise<Record<string, string>> {
  const parsed = JSON.parse(await readFile(path, "utf8")) as { devDependencies?: Record<string, string> };
  return parsed.devDependencies ?? {};
}

describe("scaffoldGame", () => {
  it("writes the template's files, with the placeholders filled", async () => {
    const folder = await scaffold();
    for (const path of [
      "package.json",
      "velvet-game.json",
      "index.html",
      "vite.config.ts",
      "tsconfig.json",
      "vitest.config.ts",
      ".gitignore",
      "README.md",
      "src/main.ts",
      "src/game.ts",
      "src/game.test.ts",
      "public/data/locales/es/00_ui.json",
      "public/data/locales/en/00_ui.json",
    ]) {
      expect(await exists(join(folder, path)), path).toBe(true);
    }
    const all = await Promise.all(
      ["package.json", "velvet-game.json", "index.html", "README.md", "src/main.ts", "vite.config.ts", "tsconfig.json"].map((path) =>
        readText(folder, path),
      ),
    );
    for (const text of all) expect(text).not.toMatch(/\{\{[A-Z][A-Z0-9_]*\}\}/u);
  });

  it("writes a manifest that the host reads, with the id and the title", async () => {
    const folder = await scaffold("my-game", "My Game");
    const reading = readManifest(await readText(folder, "velvet-game.json"));
    expect(reading.issues).toEqual([]);
    expect(reading.info).toMatchObject({ id: "my-game", title: "My Game", entry: "index.html" });
  });

  it("links the engine's packages with paths that exist, and pins the workspace's versions", async () => {
    const folder = await scaffold();
    const manifest = JSON.parse(await readText(folder, "package.json")) as {
      dependencies: Record<string, string>;
      devDependencies: Record<string, string>;
      scripts: Record<string, string>;
    };
    for (const name of ["core", "i18n", "ui"]) {
      const link = manifest.dependencies[`@velvet/${name}`] ?? "";
      expect(link.startsWith("link:")).toBe(true);
      expect(resolve(folder, link.slice("link:".length))).toBe(resolve(ENGINE_JS_DIR, "packages", name));
      expect(await exists(join(folder, link.slice("link:".length), "package.json"))).toBe(true);
    }

    const workspace = await devVersions(join(ENGINE_JS_DIR, "package.json"));
    const example = await devVersions(join(HELLO_CARDS, "package.json"));
    expect(manifest.devDependencies["typescript"]).toBe(workspace["typescript"]);
    expect(manifest.devDependencies["vitest"]).toBe(workspace["vitest"]);
    expect(manifest.devDependencies["@types/node"]).toBe(workspace["@types/node"]);
    expect(manifest.devDependencies["vite"]).toBe(example["vite"]);
    expect(manifest.scripts["check:data"]).toContain("packages/tools/bin/velvet-js.mjs check .");
  });

  it("extends the engine's shared compiler options, and lets Vite serve the engine folder", async () => {
    const folder = await scaffold();
    const tsconfig = JSON.parse(await readText(folder, "tsconfig.json")) as { extends: string };
    expect(resolve(folder, tsconfig.extends)).toBe(join(ENGINE_JS_DIR, "tsconfig.base.json"));
    const vite = await readText(folder, "vite.config.ts");
    expect(vite).toContain("resolve(dirname(fileURLToPath(import.meta.url)), ");
    expect(vite).toContain("fs: { allow:");
  });

  it("produces a game that the check accepts, with both languages", async () => {
    const folder = await scaffold();
    const result = await checkGame(folder);
    expect(result.issues).toEqual([]);
    expect(result.counts.languages).toBe(2);
  });

  it("escapes a title with quotes and markup where it is written as JSON and HTML", async () => {
    const title = 'Say "hi" <b>&</b>';
    const folder = await scaffold("quoted-game", title);
    expect(JSON.parse(await readText(folder, "velvet-game.json")) as { title: string }).toMatchObject({ title });
    expect(await readText(folder, "index.html")).toContain("Say &quot;hi&quot; &lt;b&gt;&amp;&lt;/b&gt;");
  });

  it("refuses a folder that is not empty, unless --force is given", async () => {
    const folder = await tempFolder();
    await writeFiles(folder, { "notes.txt": "keep me" });
    await expect(scaffoldGame({ folder, id: "my-game", title: "My Game", engine: ENGINE_JS_DIR, force: false })).rejects.toThrow(
      /not empty/u,
    );
    await scaffoldGame({ folder, id: "my-game", title: "My Game", engine: ENGINE_JS_DIR, force: true });
    expect(await readText(folder, "notes.txt")).toBe("keep me");
    expect(await exists(join(folder, "package.json"))).toBe(true);
  });

  it("refuses an id, a title or an engine that is not valid", async () => {
    const folder = await tempFolder();
    const base = { folder: join(folder, "game"), id: "my-game", title: "My Game", engine: ENGINE_JS_DIR, force: false };
    await expect(scaffoldGame({ ...base, id: "Not Valid" })).rejects.toThrow(/not a valid id/u);
    await expect(scaffoldGame({ ...base, id: "nul" })).rejects.toThrow(/device name/u);
    await expect(scaffoldGame({ ...base, title: "   " })).rejects.toThrow(/title/u);
    await expect(scaffoldGame({ ...base, engine: folder })).rejects.toThrow(/not the js folder of the Velvet engine/u);
  });
});

describe("the template helpers", () => {
  it("escapes JSON and HTML text, and refuses a placeholder with no value", () => {
    expect(jsonText('a "b" \\ c')).toBe('a \\"b\\" \\\\ c');
    expect(htmlText(`<a href="x">'&'</a>`)).toBe("&lt;a href=&quot;x&quot;&gt;&#39;&amp;&#39;&lt;/a&gt;");
    expect(renderTemplate("id={{GAME_ID}}", { GAME_ID: "x" })).toBe("id=x");
    expect(() => renderTemplate("{{MISSING}}", {})).toThrow(/MISSING/u);
  });

  it("has a template folder with the files of a game", async () => {
    expect(await exists(join(TEMPLATE_DIR, "package.json"))).toBe(true);
    expect(await exists(join(TEMPLATE_DIR, "_gitignore"))).toBe(true);
  });
});

describe("new on the command line", () => {
  it("creates the game and says what to do next, or the same as JSON", async () => {
    const parent = await tempFolder();
    const folder = join(parent, "cli-game");
    const out = bufferedOutput();
    expect(await main(["new", folder, "--id", "cli-game", "--title", "CLI Game"], out.output)).toBe(0);
    expect(out.stdout[0]).toContain("created");
    expect(out.stdout.join("\n")).toContain("pnpm install");

    const json = bufferedOutput();
    const again = join(parent, "json-game");
    expect(await main(["new", again, "--id", "json-game", "--title", "JSON Game", "--json"], json.output)).toBe(0);
    expect(JSON.parse(json.stdout.join("\n"))).toMatchObject({ ok: true, id: "json-game", title: "JSON Game" });
  });

  it("asks for --id and --title, and for one folder", async () => {
    const parent = await tempFolder();
    const missing = bufferedOutput();
    expect(await main(["new", join(parent, "x"), "--title", "X"], missing.output)).toBe(1);
    expect(missing.stderr.join("\n")).toContain("--id is required");
    const noTitle = bufferedOutput();
    expect(await main(["new", join(parent, "x"), "--id", "x"], noTitle.output)).toBe(1);
    expect(noTitle.stderr.join("\n")).toContain("--title is required");
    const noFolder = bufferedOutput();
    expect(await main(["new", "--id", "x", "--title", "X"], noFolder.output)).toBe(1);
  });
});
