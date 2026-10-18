// Tests for Conversation: the boxes in order, the stage directives, choices and their refusals,
// and the story's variables. The guion is small and written inline, with a tavern keeper and a
// traveller.

import { describe, expect, it } from "vitest";
import { Conversation, GuionError, GuionLibrary } from "./index";

/** The tavern guion used by these tests. Its file id is `posada`. */
const TAVERN = [
  "## inicio {en=puerta}",
  "@fondo posada/puerta",
  "**Greta:** —Hola.",
  "? ¿Qué quieres?",
  "- Un té -> te {te_pedido}",
  "- Un vino [oro >= 3] -> vino",
  "- Nada -> fin",
  "",
  "## te",
  "**Greta:** —Aquí tienes.",
  "@vn cierre",
  "",
  "## vino",
  "@var vino_pedido",
  "**Greta:** —Salud.",
  "",
  "## cierre {vn}",
  "**Greta:** —Hasta luego.",
  "",
].join("\n");

/** A library with the tavern guion, and a conversation started at its door. */
async function started(oro: number): Promise<Conversation> {
  const library = await GuionLibrary.create();
  library.add("posada.md", TAVERN);
  return library.start("posada.inicio", { values: { oro }, seen: [] });
}

/** The error that `act` throws, or undefined. */
function thrownBy(act: () => unknown): unknown {
  try {
    act();
  } catch (error) {
    return error;
  }
  return undefined;
}

describe("Conversation", () => {
  it("shows the stage with the first box, then each box after an action", async () => {
    const conversation = await started(3);
    expect(conversation.current()).toEqual({
      type: "line",
      scene: "posada.inicio",
      stage: [{ cmd: "fondo", arg: "posada/puerta" }],
      who: "greta",
      name: "Greta",
      text: "Hola.",
      kind: "dialogue",
      mood: null,
    });
    expect(conversation.next()).toEqual({
      type: "choice",
      scene: "posada.inicio",
      stage: [],
      prompt: "¿Qué quieres?",
      options: [
        { label: "Un té", available: true },
        { label: "Un vino", available: true },
        { label: "Nada", available: true },
      ],
    });
    expect(conversation.choose(0)).toMatchObject({ type: "line", scene: "posada.te", text: "Aquí tienes." });
    expect(conversation.next()).toEqual({
      type: "vn",
      scene: "posada.te",
      stage: [],
      to: "posada.cierre",
    });
  });

  it("leaves the box alone when next is called on a choice", async () => {
    const conversation = await started(0);
    conversation.next();
    const choice = conversation.current();
    expect(conversation.next()).toEqual(choice);
    expect(choice.type).toBe("choice");
  });

  it("refuses a closed option, a box that is not a choice, and an option that does not exist", async () => {
    const conversation = await started(0);
    const codeOf = (act: () => unknown): unknown => (thrownBy(act) as GuionError).code;
    expect(codeOf(() => conversation.choose(0))).toBe("not-a-choice");
    conversation.next();
    expect(codeOf(() => conversation.choose(1))).toBe("unavailable");
    expect(codeOf(() => conversation.choose(7))).toBe("no-such-option");
    expect(codeOf(() => conversation.choose(-1))).toBe("no-such-option");
    expect(codeOf(() => conversation.choose(1.5))).toBe("no-such-option");
    expect(thrownBy(() => conversation.choose(1))).toBeInstanceOf(GuionError);
    expect(conversation.current().type).toBe("choice");
  });

  it("takes an option whose condition holds, and offers it as available", async () => {
    const conversation = await started(3);
    conversation.next();
    expect(conversation.choose(1)).toMatchObject({ type: "line", scene: "posada.vino", text: "Salud." });
    expect(conversation.vars.values).toEqual({ oro: 3, vino_pedido: 1 });
  });

  it("keeps the story's variables: effects change them and every scene entered is seen", async () => {
    const conversation = await started(0);
    conversation.next();
    conversation.choose(0);
    expect(conversation.vars).toEqual({
      values: { oro: 0, te_pedido: 1 },
      seen: ["posada.inicio", "posada.te"],
    });
  });

  it("replaces the variables with setVars, and the box on screen stays", async () => {
    const conversation = await started(0);
    const before = conversation.current();
    const after = conversation.setVars({ values: { oro: 9 }, seen: ["posada.inicio"] });
    expect(after).toEqual({ values: { oro: 9 }, seen: ["posada.inicio"] });
    expect(conversation.vars).toEqual(after);
    expect(conversation.current()).toEqual(before);
    conversation.next();
    expect(conversation.choose(1)).toMatchObject({ type: "line", scene: "posada.vino" });
  });
});
