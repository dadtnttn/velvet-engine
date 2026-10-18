// Tests for GuionLibrary and parseGuion: several files, their scenes and problems, the entry
// lookup, the starting of scenes, and the errors for bad input.

import { describe, expect, it } from "vitest";
import { GuionError, GuionLibrary, parseGuion, type CastSpec } from "./index";

/** The error that a promise rejects with, or undefined when it resolves. */
async function rejectionOf(promise: Promise<unknown>): Promise<unknown> {
  try {
    await promise;
  } catch (error) {
    return error;
  }
  return undefined;
}

describe("parseGuion", () => {
  it("reports each problem with its file name and line", async () => {
    const file = await parseGuion("## a\n@nada\n", { file: "mal.md" });
    expect(file.problems).toEqual([
      { file: "mal.md", line: 2, message: "orden desconocida o ilegible: @nada" },
    ]);
  });

  it("names the file guion.md and reads the narrator when no options are given", async () => {
    const file = await parseGuion("**Narrador:** —Llueve.\n");
    expect(file).toMatchObject({ file: "guion.md", id: "guion", problems: [] });
    expect(file.scenes[0]?.id).toBe("guion.inicio");
  });

  it("rejects a cast that is not valid, with the code bad-input", async () => {
    const badCast = { characters: { ana: "Ana" } } as unknown as CastSpec;
    const error = await rejectionOf(parseGuion("## a\n", { cast: badCast }));
    expect(error).toBeInstanceOf(GuionError);
    expect((error as GuionError).code).toBe("bad-input");
  });
});

describe("GuionLibrary", () => {
  it("reads several files, lists their scenes and problems, and finds entries across them", async () => {
    const library = await GuionLibrary.create({ characters: { guardia: ["Bruno"] } });
    const taberna = library.add(
      "taberna.md",
      "## llegada {quien=bruno}\n**Bruno:** —Alto.\n",
    );
    expect(taberna.scenes.map((scene) => scene.id)).toEqual(["taberna.llegada"]);
    library.add("muralla.md", "## ronda {en=puerta, si=alarma}\n@nada\n");

    expect(library.scenes().map((scene) => scene.id)).toEqual([
      "taberna.llegada",
      "muralla.ronda",
    ]);
    expect(library.problems()).toEqual([
      { file: "muralla.md", line: 2, message: "orden desconocida o ilegible: @nada" },
    ]);
    expect(library.entry({ who: "bruno" })).toBe("taberna.llegada");
    expect(library.entry({ place: "puerta" }, { values: {}, seen: [] })).toBeNull();
    expect(library.entry({ place: "puerta" }, { values: { alarma: 1 }, seen: [] })).toBe(
      "muralla.ronda",
    );
  });

  it("starts a scene by its id, and refuses an id that names no scene", async () => {
    const library = await GuionLibrary.create();
    library.add("taberna.md", "## llegada\n**Greta:** —Hola.\n");
    expect(library.start("taberna.llegada").current()).toMatchObject({
      type: "line",
      who: "greta",
    });

    const unknown = await rejectionOf(Promise.resolve().then(() => library.start("taberna.nada")));
    expect((unknown as GuionError).code).toBe("unknown-scene");
    const bare = await rejectionOf(Promise.resolve().then(() => library.start("llegada")));
    expect((bare as GuionError).code).toBe("unknown-scene");
  });

  it("refuses a cast or variables that are not valid, with the code bad-input", async () => {
    const badCast = { thinker: 3 } as unknown as CastSpec;
    const error = await rejectionOf(GuionLibrary.create(badCast));
    expect((error as GuionError).code).toBe("bad-input");

    const library = await GuionLibrary.create();
    library.add("taberna.md", "## llegada\n**Greta:** —Hola.\n");
    const badVars = { values: { oro: 1.5 }, seen: [] };
    expect(() => library.start("taberna.llegada", badVars)).toThrow(GuionError);
  });
});
