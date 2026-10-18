// @vitest-environment happy-dom
// Tests for the dialogue box: the plate and the text, the writing at the speed of the ticks, fast-forward, skip,
// auto-advance, the choices with the keys, the end and the hand-over, the hooks, the backlog, and the cleanup.

import { afterEach, describe, expect, it, vi } from "vitest";
import {
  createDialogueBox,
  type DialogueBox,
  type DialogueBoxOptions,
  type DialogueChoice,
  type DialogueLine,
  type DialogueSession,
  type DialogueShown,
} from "./dialogue-box";

afterEach(() => {
  document.body.innerHTML = "";
});

/** A line of dialogue. */
function line(text: string, name = "Ana"): DialogueLine {
  return { type: "line", who: name.toLowerCase(), name, text, mood: null };
}

/** A choice, with each option as its label and whether it can be taken. */
function choice(prompt: string | null, options: Array<[string, boolean]>): DialogueChoice {
  return { type: "choice", prompt, options: options.map(([label, available]) => ({ label, available })) };
}

/**
 * A session that plays a list of boxes in order. Past the list it is over. It records the options the
 * player took.
 */
function scripted(items: readonly DialogueShown[]): DialogueSession & { chosen: number[] } {
  let index = 0;
  const chosen: number[] = [];
  const at = (): DialogueShown => items[index] ?? { type: "end" };
  return {
    chosen,
    current: at,
    next: () => {
      index += 1;
      return at();
    },
    choose: (option: number) => {
      chosen.push(option);
      index += 1;
      return at();
    },
  };
}

/** A box in the document. Its letters are written at 40 a second unless the options say otherwise. */
function make(options: DialogueBoxOptions = {}): DialogueBox {
  const box = createDialogueBox(options);
  document.body.append(box.el);
  return box;
}

/** The text element of the box. */
function textOf(box: DialogueBox): HTMLElement {
  const node = box.el.querySelector<HTMLElement>(".vu-dialogue-text");
  if (node === null) throw new Error("no text");
  return node;
}

/** The title tab of the speaker's plate. */
function titleOf(box: DialogueBox): HTMLElement {
  const node = box.el.querySelector<HTMLElement>(".vu-dialogue-panel > .vu-plate-title");
  if (node === null) throw new Error("no title");
  return node;
}

/** The button of the box with a given label. */
function buttonNamed(box: DialogueBox, label: string): HTMLButtonElement {
  const found = Array.from(box.el.querySelectorAll<HTMLButtonElement>(".vu-button")).find(
    (button) => button.textContent === label,
  );
  if (found === undefined) throw new Error(`no button ${label}`);
  return found;
}

describe("the line on screen", () => {
  it("plays the first line: the name on the plate, and the text written as the box ticks", () => {
    const box = make();
    box.play(scripted([line("Hola mundo")]));
    expect(box.isOpen).toBe(true);
    expect(box.el.hidden).toBe(false);
    expect(titleOf(box).textContent).toBe("Ana");
    expect(textOf(box).textContent).toBe("");
    box.tick(0.05);
    expect(textOf(box).textContent).toBe("Ho");
    expect(box.shown?.type).toBe("line");
  });

  it("hides the title of the plate for narration, which has no name", () => {
    const box = make({ speed: 0 });
    box.play(scripted([line("Llueve.", "")]));
    expect(titleOf(box).hidden).toBe(true);
  });

  it("writes a line at once when the speed is zero", () => {
    const box = make({ speed: 0 });
    box.play(scripted([line("Todo de golpe")]));
    expect(textOf(box).textContent).toBe("Todo de golpe");
  });

  it("finishes a line that is still being written when advance is called, and goes on at the next call", () => {
    const box = make();
    box.play(scripted([line("Uno"), line("Dos", "Bea")]));
    box.advance();
    expect(textOf(box).textContent).toBe("Uno");
    expect(titleOf(box).textContent).toBe("Ana");
    box.advance();
    expect(titleOf(box).textContent).toBe("Bea");
    expect(textOf(box).textContent).toBe("");
  });

  it("moves the focus to the Next button when a line comes on, so a keyboard or a pad can go on", () => {
    const box = make({ speed: 0 });
    box.play(scripted([line("Uno"), line("Dos")]));
    expect(document.activeElement).toBe(buttonNamed(box, "Next"));
  });

  it("a click on the text goes on", () => {
    const box = make({ speed: 0 });
    box.play(scripted([line("Uno"), line("Dos", "Bea")]));
    textOf(box).click();
    expect(titleOf(box).textContent).toBe("Bea");
  });

  it("the text can be read through the box's shown value", () => {
    const box = make({ speed: 0 });
    const first = line("Uno");
    box.play(scripted([first]));
    expect(box.shown).toEqual(first);
  });
});

describe("speed, fast-forward and skip", () => {
  it("fast-forward multiplies the writing speed while it is on", () => {
    const box = make({ speed: 10 });
    box.play(scripted([line("abcdefghijklmnop")]));
    box.setFastForward(true);
    expect(box.fastForward).toBe(true);
    box.tick(0.1);
    expect(textOf(box).textContent).toBe("abcd");
    box.setFastForward(false);
    box.tick(0.1);
    expect(textOf(box).textContent).toBe("abcde");
  });

  it("skip writes each line at once, and goes on through the lines until a choice", () => {
    const box = make({ speed: 40 });
    box.setSkipping(true);
    expect(box.skipping).toBe(true);
    box.play(scripted([line("Uno"), line("Dos", "Bea"), choice("¿Y ahora?", [["Ir", true]])]));
    expect(textOf(box).textContent).toBe("Uno");
    box.tick(0.016);
    expect(titleOf(box).textContent).toBe("Bea");
    box.tick(0.016);
    expect(box.shown?.type).toBe("choice");
    box.tick(0.016);
    expect(box.shown?.type).toBe("choice");
  });

  it("turning skip on finishes the line that is being written", () => {
    const box = make({ speed: 2 });
    box.play(scripted([line("Lento")]));
    box.setSkipping(true);
    expect(textOf(box).textContent).toBe("Lento");
  });

  it("writes nothing while the time passed is negative or not a number", () => {
    const box = make({ speed: 10 });
    box.play(scripted([line("abc")]));
    box.tick(-1);
    box.tick(Number.NaN);
    expect(textOf(box).textContent).toBe("");
    box.tick(0.1);
    expect(textOf(box).textContent).toBe("a");
  });
});

describe("auto-advance", () => {
  it("goes on by itself after the delay, and not before it", () => {
    const box = make({ autoAdvance: true, autoDelaySeconds: 1, speed: 0 });
    box.play(scripted([line("Uno"), line("Dos", "Bea")]));
    box.tick(0.5);
    expect(titleOf(box).textContent).toBe("Ana");
    box.tick(0.6);
    expect(titleOf(box).textContent).toBe("Bea");
  });

  it("fast-forward shortens the wait", () => {
    const box = make({ autoAdvance: true, autoDelaySeconds: 1, speed: 0, fastForwardFactor: 4 });
    box.play(scripted([line("Uno"), line("Dos", "Bea")]));
    box.setFastForward(true);
    box.tick(0.3);
    expect(titleOf(box).textContent).toBe("Bea");
  });

  it("the Auto button shows the setting, and turns it on and off", () => {
    const box = make({ speed: 0 });
    const auto = buttonNamed(box, "Auto");
    expect(auto.getAttribute("aria-pressed")).toBe("false");
    auto.click();
    expect(auto.getAttribute("aria-pressed")).toBe("true");
    box.update({ autoAdvance: false });
    expect(auto.getAttribute("aria-pressed")).toBe("false");
  });
});

describe("choices", () => {
  it("shows the prompt, the options, and hides the Next button", () => {
    const box = make();
    box.play(scripted([choice("¿Qué haces?", [["Ir", true], ["Esperar", false], ["Huir", true]])]));
    expect(textOf(box).textContent).toBe("¿Qué haces?");
    expect(box.el.querySelectorAll(".vu-dialogue-choices [data-id]")).toHaveLength(3);
    expect(box.el.querySelector('[data-id="1"]')?.getAttribute("aria-disabled")).toBe("true");
    expect(buttonNamed(box, "Next").hidden).toBe(true);
  });

  it("gives the focus to the first option that can be taken", () => {
    const box = make();
    box.play(scripted([choice(null, [["Esperar", false], ["Ir", true]])]));
    expect(document.activeElement?.textContent).toBe("Ir");
  });

  it("hands the index of an option back to the session, and ignores an option that is closed", () => {
    const box = make();
    const session = scripted([choice(null, [["Ir", true], ["Esperar", false], ["Huir", true]]), line("Fuera")]);
    box.play(session);
    box.choose(1);
    expect(session.chosen).toEqual([]);
    box.choose(2);
    expect(session.chosen).toEqual([2]);
    expect(titleOf(box).textContent).toBe("Ana");
  });

  it("choosing with Enter on the focused option takes it", () => {
    const box = make();
    const session = scripted([choice(null, [["Ir", true], ["Huir", true]])]);
    box.play(session);
    const focused = document.activeElement;
    focused?.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true, cancelable: true }));
    expect(session.chosen).toEqual([0]);
  });

  it("writes the option that the player took into the backlog", () => {
    const box = make({ speed: 0 });
    box.play(scripted([choice(null, [["Ir", true]]), line("Fuera")]));
    box.choose(0);
    expect(box.backlog.at(-2)).toEqual({ name: "", text: "Ir", kind: "choice" });
    expect(box.backlog.at(-1)).toEqual({ name: "Ana", text: "Fuera", kind: "line" });
  });

  it("calls onChoice with the choice on screen", () => {
    const onChoice = vi.fn();
    const box = make({ onChoice });
    const picked = choice("¿?", [["Sí", true]]);
    box.play(scripted([picked]));
    expect(onChoice).toHaveBeenCalledWith(picked);
  });
});

describe("the end and the hand-over", () => {
  it("hides the box at the end, and calls onEnd once", () => {
    const onEnd = vi.fn();
    const box = make({ speed: 0, onEnd });
    box.play(scripted([line("Fin")]));
    box.advance();
    expect(box.el.hidden).toBe(true);
    expect(box.isOpen).toBe(false);
    expect(onEnd).toHaveBeenCalledTimes(1);
    expect(onEnd).toHaveBeenCalledWith({ type: "end" });
    box.advance();
    expect(onEnd).toHaveBeenCalledTimes(1);
  });

  it("ends at a hand-over to a visual-novel scene, and passes the scene's name", () => {
    const onEnd = vi.fn();
    const box = make({ speed: 0, onEnd });
    box.play(scripted([line("Vamos"), { type: "vn", to: "intro.start" }]));
    box.advance();
    expect(onEnd).toHaveBeenCalledWith({ type: "vn", to: "intro.start" });
    expect(box.shown).toEqual({ type: "vn", to: "intro.start" });
  });

  it("a conversation that has already ended calls onEnd straight away", () => {
    const onEnd = vi.fn();
    const box = make({ onEnd });
    box.play(scripted([]));
    expect(onEnd).toHaveBeenCalledWith({ type: "end" });
    expect(box.el.hidden).toBe(true);
  });

  it("stop hides the box without calling onEnd", () => {
    const onEnd = vi.fn();
    const box = make({ onEnd });
    box.play(scripted([line("Uno")]));
    box.stop();
    expect(box.el.hidden).toBe(true);
    expect(box.isOpen).toBe(false);
    expect(onEnd).not.toHaveBeenCalled();
  });

  it("a new conversation replaces one that is playing", () => {
    const box = make({ speed: 0 });
    box.play(scripted([line("Primera")]));
    box.play(scripted([line("Segunda", "Bea")]));
    expect(titleOf(box).textContent).toBe("Bea");
    expect(textOf(box).textContent).toBe("Segunda");
  });
});

describe("the hooks and the portrait", () => {
  it("calls onLine for each line, and onLetter for each letter but not for a space", () => {
    const onLine = vi.fn();
    const onLetter = vi.fn();
    const box = make({ speed: 0, onLine, onLetter });
    const first = line("a b");
    box.play(scripted([first]));
    expect(onLine).toHaveBeenCalledWith(first);
    expect(onLetter.mock.calls.map((call) => call[0])).toEqual(["a", "b"]);
  });

  it("the portrait comes from the portrait function, and no portrait hides the slot", () => {
    const portrait = vi.fn((spoken: DialogueLine) => {
      if (spoken.who !== "ana") return null;
      const image = document.createElement("img");
      image.className = "pic";
      return image;
    });
    const box = make({ speed: 0, portrait });
    box.play(scripted([line("Uno"), line("Dos", "Bea")]));
    const slot = box.el.querySelector<HTMLElement>(".vu-dialogue-portrait");
    expect(slot?.querySelector(".pic")).not.toBeNull();
    expect(slot?.hidden).toBe(false);
    box.advance();
    expect(slot?.hidden).toBe(true);
    expect(portrait).toHaveBeenCalledWith(expect.objectContaining({ who: "bea" }));
  });
});

describe("the backlog", () => {
  it("keeps the lines and the choices so far, oldest first", () => {
    const box = make({ speed: 0 });
    box.play(scripted([line("Uno", "Ana"), line("Dos", "Bea"), choice(null, [["Ir", true]]), line("Fin")]));
    box.advance();
    box.advance();
    box.choose(0);
    expect(box.backlog).toEqual([
      { name: "Ana", text: "Uno", kind: "line" },
      { name: "Bea", text: "Dos", kind: "line" },
      { name: "", text: "Ir", kind: "choice" },
      { name: "Ana", text: "Fin", kind: "line" },
    ]);
  });

  it("the Log button opens the backlog, and Escape closes it with the focus back on the Next button", () => {
    const box = make({ speed: 0 });
    box.play(scripted([line("Uno"), line("Dos")]));
    const next = buttonNamed(box, "Next");
    next.focus();
    buttonNamed(box, "Log").click();
    const log = box.el.querySelector<HTMLElement>(".vu-dialogue-backlog");
    expect(log?.hidden).toBe(false);
    expect(log?.querySelectorAll(".vu-dialogue-log-item")).toHaveLength(1);
    expect(document.activeElement?.textContent).toBe("Close");
    log?.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true, cancelable: true }));
    expect(log?.hidden).toBe(true);
    expect(document.activeElement).toBe(next);
  });

  it("opens and closes from the box's methods, and the close button closes it", () => {
    const box = make({ speed: 0 });
    box.play(scripted([line("Uno")]));
    box.openBacklog();
    const log = box.el.querySelector<HTMLElement>(".vu-dialogue-backlog");
    expect(log?.hidden).toBe(false);
    box.closeBacklog();
    expect(log?.hidden).toBe(true);
    box.openBacklog();
    buttonNamed(box, "Close").click();
    expect(log?.hidden).toBe(true);
  });
});

describe("a pad", () => {
  it("A goes on: Enter on the focused Next button, followed by the click that startPadMenuKeys makes", () => {
    const box = make({ speed: 0 });
    box.play(scripted([line("Uno"), line("Dos", "Bea")]));
    const next = buttonNamed(box, "Next");
    next.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true, cancelable: true }));
    next.click();
    expect(titleOf(box).textContent).toBe("Bea");
  });

  it("the d-pad moves between the options, and A takes the focused one", () => {
    const box = make();
    const session = scripted([choice(null, [["Ir", true], ["Huir", true]])]);
    box.play(session);
    document.activeElement?.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowDown", bubbles: true, cancelable: true }));
    expect(document.activeElement?.textContent).toBe("Huir");
    document.activeElement?.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true, cancelable: true }));
    expect(session.chosen).toEqual([1]);
  });
});

describe("props and cleanup", () => {
  it("refuses a speed, a fast-forward factor or a delay that is out of range", () => {
    expect(() => createDialogueBox({ speed: -1 })).toThrow(RangeError);
    expect(() => createDialogueBox({ fastForwardFactor: 0.5 })).toThrow(RangeError);
    expect(() => createDialogueBox({ autoDelaySeconds: Number.POSITIVE_INFINITY })).toThrow(RangeError);
    const box = make();
    expect(() => box.update({ speed: -2 })).toThrow(RangeError);
  });

  it("writes its buttons in the language of the lookup, with the English fallbacks otherwise", () => {
    const box = make({ t: (key) => (key === "dialogue.next" ? "Siguiente" : key) });
    expect(buttonNamed(box, "Siguiente").textContent).toBe("Siguiente");
    expect(buttonNamed(box, "Auto").textContent).toBe("Auto");
  });

  it("does nothing when no conversation plays, and destroy removes the box", () => {
    const box = make();
    expect(() => box.tick(1)).not.toThrow();
    box.destroy();
    expect(box.el.isConnected).toBe(false);
    expect(box.isOpen).toBe(false);
    box.play(scripted([line("Nada")]));
    expect(box.isOpen).toBe(false);
  });
});
