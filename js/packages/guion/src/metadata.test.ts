// The front matter's metadata through this package: the other keys in file order, their values as
// written, and the problems that a bad key or a repeated key gives. The shared fixtures
// (cabecera.md, cabecera-errores.md) check the same results against the Rust runtime.

import { describe, expect, it } from "vitest";
import { GuionLibrary, parseGuion } from "./index";

/** A guion whose front matter has three metadata keys, one of them a list. */
const TEXT =
  "---\nid: escena\nzeta: 1\ncategoria: escena\nparticipantes: Lira, Nyxara\n---\n## inicio\n**Lira:** —Hola.\n";

describe("guion metadata", () => {
  it("keeps the other front matter keys in file order, with their values as written", async () => {
    const file = await parseGuion(TEXT, { file: "escena.md" });
    expect(Object.keys(file.meta)).toEqual(["zeta", "categoria", "participantes"]);
    expect(file.meta).toEqual({ zeta: "1", categoria: "escena", participantes: "Lira, Nyxara" });
    expect(file.problems).toEqual([]);
  });

  it("keeps the known keys out of the metadata, and answers with the same fields as before", async () => {
    const file = await parseGuion("---\nid: ana\nTitle: Ana\nsala: posada\nhora: 10:30\n---\n", {
      file: "ana.md",
    });
    expect(file).toMatchObject({ id: "ana", title: "Ana", sala: "posada" });
    expect(file.meta).toEqual({ hora: "10:30" });
  });

  it("gives an empty metadata object when the file has no front matter", async () => {
    const file = await parseGuion("## inicio\n**Lira:** —Hola.\n", { file: "sin.md" });
    expect(file.meta).toEqual({});
  });

  it("leaves a list as one string, which the game splits", async () => {
    const file = await parseGuion(TEXT, { file: "escena.md" });
    const names = file.meta["participantes"]?.split(",").map((name) => name.trim());
    expect(names).toEqual(["Lira", "Nyxara"]);
  });

  it("keeps the colons of a value and an empty value, and tells an empty value from a missing key", async () => {
    const file = await parseGuion("---\nhora: 10:30\nvacio:\n---\n## inicio\n**Lira:** —Hola.\n", {
      file: "hora.md",
    });
    expect(file.meta["hora"]).toBe("10:30");
    expect(file.meta["vacio"]).toBe("");
    expect(Object.hasOwn(file.meta, "vacio")).toBe(true);
    expect(Object.hasOwn(file.meta, "nada")).toBe(false);
  });

  it("gives a library's file the same metadata", async () => {
    const library = await GuionLibrary.create();
    const file = library.add("escena.md", TEXT);
    expect(Object.keys(file.meta)).toEqual(["zeta", "categoria", "participantes"]);
    expect(library.problems()).toEqual([]);
  });

  it("reports a key that breaks the name rule, and a key given twice, with their lines", async () => {
    const text = "---\nCategoria: x\n2da: x\nok_1: uno\nok_1: dos\n---\n## inicio\n**Ana:** —Hola.\n";
    const file = await parseGuion(text, { file: "mal.md" });
    expect(file.problems.map((problem) => problem.line)).toEqual([2, 3, 5]);
    expect(file.problems[2]?.message).toBe("clave repetida en la cabecera: ok_1");
    expect(file.meta).toEqual({ ok_1: "uno" });
  });
});
