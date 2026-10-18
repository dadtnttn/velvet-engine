// @vitest-environment happy-dom
// Tests for the rebinding panel: the rows and their texts, a capture started with Enter or a click, the
// results of a capture (bound, refused, swapped, cancelled, failed), the reset and delete buttons, the
// keyboard moves between rows, a change of language, and the cleanup on destroy. The port is a fake that
// records its calls and lets the test finish each capture by hand.

import { afterEach, describe, expect, it, vi } from "vitest";
import {
  createRebindPanel,
  type RebindAction,
  type RebindDisplay,
  type RebindPanel,
  type RebindPort,
  type RebindRequest,
  type RebindResult,
  type TranslateFn,
} from "./rebind-panel";

type Action = "confirm" | "attack";

/** A capture that the fake port started and that the test finishes. */
interface PendingCapture {
  readonly action: Action;
  readonly slot: number;
  readonly request: RebindRequest;
  resolve(result: RebindResult<Action> | null): void;
  reject(error: Error): void;
}

/** The texts of the Spanish catalog that the tests use. Keys without a text come back as the key. */
const CATALOG: Readonly<Record<string, string>> = {
  "input.rebind.title": "Controles",
  "input.rebind.list": "Acciones y controles",
  "input.rebind.hint": "Elige una fila y pulsa un control",
  "input.rebind.capture": "Pulsa una tecla, un botón o mueve una palanca. Esc cancela.",
  "input.rebind.bound": "Asignado: {control}",
  "input.rebind.swapped": "{control}: intercambiado con {action}",
  "input.rebind.shared": "{control}: también en {action}",
  "input.rebind.refused": "{control} ya es de {action}",
  "input.rebind.cancelled": "Cancelado",
  "input.rebind.failed": "No se pudo asignar",
  "input.rebind.cleared": "Quitado",
  "input.rebind.reset.action": "Restablecer esta acción",
  "input.rebind.reset.all": "Restablecer todo",
  "input.rebind.reset.done": "Restablecido",
  "input.rebind.row": "{action}: {device} {control}",
  "input.rebind.row.empty": "{action}: sin asignar",
  "input.rebind.row.capturing": "{action}: pulsa un control…",
  "input.rebind.inverted": "{control} invertido",
  "input.device.keyboard": "Teclado",
  "input.device.mouse": "Ratón",
  "input.device.gamepad": "Mando",
  "input.key.Enter": "Intro",
  "input.key.KeyQ": "Q",
  "input.key.KeyZ": "Z",
  "input.pad.a": "A",
  "input.pad.axis.leftY": "Palanca izquierda Y",
  "action.confirm": "Confirmar",
  "action.attack": "Atacar",
};

/** The English texts, for the language change. */
const ENGLISH: Readonly<Record<string, string>> = {
  "input.rebind.row": "{action}: {device} {control}",
  "input.device.keyboard": "Keyboard",
  "input.key.Enter": "Enter",
  "action.confirm": "Confirm",
};

/** A text lookup over a catalog: placeholders take the arguments, and a missing key comes back as itself. */
function lookup(catalog: Readonly<Record<string, string>>): TranslateFn {
  return (key, args) => {
    const template = catalog[key];
    if (template === undefined) return key;
    return template.replace(/\{(\w+)\}/g, (match: string, name: string) => String(args?.[name] ?? match));
  };
}

const ENTER: RebindDisplay = { device: "keyboard", code: "Enter", labelKey: "input.key.Enter" };
const KEY_Q: RebindDisplay = { device: "keyboard", code: "KeyQ", labelKey: "input.key.KeyQ" };
const KEY_Z: RebindDisplay = { device: "keyboard", code: "KeyZ", labelKey: "input.key.KeyZ" };
const PAD_A: RebindDisplay = { device: "gamepad", code: "a", labelKey: "input.pad.a" };

/** A port over two actions, which records the calls made to it. */
function fakePort(): {
  port: RebindPort<Action>;
  captures: PendingCapture[];
  cleared: Array<[Action, number]>;
  resets: Action[];
  resetAllCalls: () => number;
  setSlot(action: Action, slot: number, display: RebindDisplay | null): void;
} {
  const slots = new Map<Action, Array<RebindDisplay | null>>([
    ["confirm", [ENTER, null]],
    ["attack", [PAD_A]],
  ]);
  const captures: PendingCapture[] = [];
  const cleared: Array<[Action, number]> = [];
  const resets: Action[] = [];
  let resetAll = 0;
  const port: RebindPort<Action> = {
    slots: (action) => (slots.get(action) ?? []).map((display, slot) => ({ slot, display })),
    capture: (action, slot, request) =>
      new Promise((resolve, reject) => {
        const entry: PendingCapture = {
          action,
          slot,
          request,
          resolve: (result) => resolve(result),
          reject: (error) => reject(error),
        };
        captures.push(entry);
        request.signal.addEventListener("abort", () => resolve(null));
      }),
    clear: (action, slot) => {
      cleared.push([action, slot]);
    },
    resetAction: (action) => {
      resets.push(action);
    },
    resetAll: () => {
      resetAll += 1;
    },
  };
  return {
    port,
    captures,
    cleared,
    resets,
    resetAllCalls: () => resetAll,
    setSlot(action, slot, display) {
      const list = [...(slots.get(action) ?? [])];
      list[slot] = display;
      slots.set(action, list);
    },
  };
}

const ACTIONS: RebindAction<Action>[] = [
  { id: "confirm", labelKey: "action.confirm" },
  { id: "attack", labelKey: "action.attack" },
];

const mounted: RebindPanel<Action>[] = [];

afterEach(() => {
  for (const panel of mounted.splice(0)) panel.destroy();
  vi.useRealTimers();
  document.body.replaceChildren();
});

/** A panel in the page, over a fresh fake port. */
function setup(extra: { t?: TranslateFn; policy?: "swap" | "refuse" | "allow" } = {}): {
  panel: RebindPanel<Action>;
  fake: ReturnType<typeof fakePort>;
} {
  const fake = fakePort();
  const panel = createRebindPanel({
    port: fake.port,
    actions: ACTIONS,
    t: extra.t ?? lookup(CATALOG),
    policy: extra.policy,
  });
  mounted.push(panel);
  document.body.append(panel.el);
  return { panel, fake };
}

/** The menu rows, as their texts. */
function rowTexts(panel: RebindPanel<Action>): string[] {
  return Array.from(panel.el.querySelectorAll<HTMLElement>('[role="menuitem"]')).map((row) => row.textContent ?? "");
}

/** The menu row at an index. */
function rowAt(panel: RebindPanel<Action>, index: number): HTMLElement {
  const row = panel.el.querySelectorAll<HTMLElement>('[role="menuitem"]')[index];
  if (row === undefined) throw new Error(`no row ${index}`);
  return row;
}

/** The status line. */
function statusText(panel: RebindPanel<Action>): string {
  return panel.el.querySelector(".vu-rebind-status")?.textContent ?? "";
}

/** The button with the given text. */
function buttonNamed(panel: RebindPanel<Action>, text: string): HTMLButtonElement {
  const found = Array.from(panel.el.querySelectorAll("button")).find((button) => button.textContent === text);
  if (found === undefined) throw new Error(`no button "${text}"`);
  return found;
}

/** Presses a key on an element, as the keyboard does: it bubbles, and it can be cancelled. */
function press(target: HTMLElement, key: string): void {
  target.dispatchEvent(new KeyboardEvent("keydown", { key, bubbles: true, cancelable: true }));
}

/** Lets the pending promises run. */
async function settle(): Promise<void> {
  for (let step = 0; step < 5; step += 1) await Promise.resolve();
}

describe("rows", () => {
  it("lists each slot with its action, device and control, and an empty row for a new binding", () => {
    const { panel } = setup();
    expect(rowTexts(panel)).toEqual([
      "Confirmar: Teclado Intro",
      "Confirmar: sin asignar",
      "Atacar: Mando A",
      "Atacar: sin asignar",
    ]);
  });

  it("shows the title, the hint and the two reset buttons with their texts", () => {
    const { panel } = setup();
    expect(panel.el.querySelector(".vu-plate-title")?.textContent).toBe("Controles");
    expect(statusText(panel)).toBe("Elige una fila y pulsa un control");
    expect(buttonNamed(panel, "Restablecer esta acción")).toBeTruthy();
    expect(buttonNamed(panel, "Restablecer todo")).toBeTruthy();
  });

  it("marks an inverted axis, and falls back to the code of a control whose text is missing", () => {
    const { panel, fake } = setup();
    fake.setSlot("attack", 0, { device: "gamepad", code: "leftY", labelKey: "input.pad.axis.leftY", negative: true });
    fake.setSlot("confirm", 1, { device: "keyboard", code: "KeyP", labelKey: "input.key.KeyP" });
    panel.refresh();
    expect(rowTexts(panel)).toEqual([
      "Confirmar: Teclado Intro",
      "Confirmar: Teclado KeyP",
      "Confirmar: sin asignar",
      "Atacar: Mando Palanca izquierda Y invertido",
      "Atacar: sin asignar",
    ]);
  });
});

describe("capture", () => {
  it("starts a capture for the row when Enter is pressed on it, and shows the prompt", () => {
    const { panel, fake } = setup();
    rowAt(panel, 0).focus();
    press(rowAt(panel, 0), "Enter");
    expect(fake.captures).toHaveLength(1);
    expect(fake.captures[0]).toMatchObject({ action: "confirm", slot: 0 });
    expect(fake.captures[0]?.request).toMatchObject({
      devices: ["keyboard", "mouse", "gamepad"],
      timeoutMs: 8000,
      policy: "swap",
    });
    expect(statusText(panel)).toBe("Pulsa una tecla, un botón o mueve una palanca. Esc cancela.");
    expect(rowTexts(panel)[0]).toBe("Confirmar: pulsa un control…");
    expect(panel.capturing).toBe(true);
  });

  it("starts a capture when a row is clicked", () => {
    const { panel, fake } = setup();
    rowAt(panel, 2).click();
    expect(fake.captures[0]).toMatchObject({ action: "attack", slot: 0 });
  });

  it("uses the policy that the panel was made with", () => {
    const { panel, fake } = setup({ policy: "refuse" });
    rowAt(panel, 1).click();
    expect(fake.captures[0]?.request.policy).toBe("refuse");
  });

  it("shows a bound input and redraws the row with it", async () => {
    const { panel, fake } = setup();
    rowAt(panel, 0).click();
    fake.setSlot("confirm", 0, KEY_Q);
    fake.captures[0]?.resolve({ status: "bound", conflicts: [], display: KEY_Q });
    await settle();
    expect(statusText(panel)).toBe("Asignado: Q");
    expect(rowTexts(panel)[0]).toBe("Confirmar: Teclado Q");
    expect(panel.capturing).toBe(false);
  });

  it("names the action that a refused conflict clashes with", async () => {
    const { panel, fake } = setup();
    rowAt(panel, 0).click();
    fake.captures[0]?.resolve({ status: "refused", conflicts: [{ action: "attack", slot: 0 }], display: KEY_Z });
    await settle();
    expect(statusText(panel)).toBe("Z ya es de Atacar");
  });

  it("names the action that took the control in a swap, and the one that shares it", async () => {
    const { panel, fake } = setup();
    rowAt(panel, 0).click();
    fake.captures[0]?.resolve({ status: "swapped", conflicts: [{ action: "attack", slot: 0 }], display: KEY_Z });
    await settle();
    expect(statusText(panel)).toBe("Z: intercambiado con Atacar");

    rowAt(panel, 0).click();
    fake.captures[1]?.resolve({ status: "allowed", conflicts: [{ action: "attack", slot: 0 }], display: KEY_Z });
    await settle();
    expect(statusText(panel)).toBe("Z: también en Atacar");
  });

  it("says that a cancelled capture was cancelled, and that a failed one failed", async () => {
    const { panel, fake } = setup();
    rowAt(panel, 0).click();
    fake.captures[0]?.resolve(null);
    await settle();
    expect(statusText(panel)).toBe("Cancelado");

    rowAt(panel, 0).click();
    fake.captures[1]?.reject(new Error("the port failed"));
    await settle();
    expect(statusText(panel)).toBe("No se pudo asignar");
    expect(panel.capturing).toBe(false);
  });

  it("is cancelled by cancel(), which aborts the request", async () => {
    const { panel, fake } = setup();
    rowAt(panel, 0).click();
    panel.cancel();
    expect(fake.captures[0]?.request.signal.aborted).toBe(true);
    await settle();
    expect(statusText(panel)).toBe("Cancelado");
    expect(panel.capturing).toBe(false);
  });

  it("does not start a second capture while one runs", () => {
    const { panel, fake } = setup();
    rowAt(panel, 0).click();
    rowAt(panel, 2).click();
    press(rowAt(panel, 2), "Enter");
    expect(fake.captures).toHaveLength(1);
  });

  it("ignores the click that follows a capture made with the mouse, and accepts a choice later", async () => {
    vi.useFakeTimers();
    const { panel, fake } = setup();
    rowAt(panel, 0).click();
    fake.captures[0]?.resolve({ status: "bound", conflicts: [], display: { device: "mouse", code: "left", labelKey: "input.mouse.left" } });
    await settle();
    rowAt(panel, 1).click();
    expect(fake.captures).toHaveLength(1);

    vi.advanceTimersByTime(500);
    rowAt(panel, 1).click();
    expect(fake.captures).toHaveLength(2);
  });
});

describe("reset and delete", () => {
  it("the reset button restores the action of the focused row", () => {
    const { panel, fake } = setup();
    rowAt(panel, 2).focus();
    buttonNamed(panel, "Restablecer esta acción").click();
    expect(fake.resets).toEqual(["attack"]);
    expect(statusText(panel)).toBe("Restablecido");
  });

  it("the reset-all button restores every action", () => {
    const { panel, fake } = setup();
    buttonNamed(panel, "Restablecer todo").click();
    expect(fake.resetAllCalls()).toBe(1);
    expect(statusText(panel)).toBe("Restablecido");
  });

  it("does not reset while a capture runs", () => {
    const { panel, fake } = setup();
    rowAt(panel, 0).click();
    buttonNamed(panel, "Restablecer todo").click();
    expect(fake.resetAllCalls()).toBe(0);
  });

  it("Delete does nothing when the focus is on a button rather than the list", () => {
    const { panel, fake } = setup();
    rowAt(panel, 0).focus();
    const reset = buttonNamed(panel, "Restablecer todo");
    reset.focus();
    press(reset, "Delete");
    expect(fake.cleared).toEqual([]);
  });

  it("Delete clears the focused slot, and does nothing while a capture runs", () => {
    const { panel, fake } = setup();
    rowAt(panel, 0).focus();
    press(rowAt(panel, 0), "Delete");
    expect(fake.cleared).toEqual([["confirm", 0]]);
    expect(statusText(panel)).toBe("Quitado");

    rowAt(panel, 0).click();
    press(rowAt(panel, 0), "Delete");
    expect(fake.cleared).toHaveLength(1);
  });
});

describe("keyboard and language", () => {
  it("moves between the rows with the arrows, as a menu does", () => {
    const { panel } = setup();
    rowAt(panel, 0).focus();
    press(rowAt(panel, 0), "ArrowDown");
    expect(document.activeElement).toBe(rowAt(panel, 1));
  });

  it("shows the new texts when the lookup changes, and the row's focus stays put", () => {
    const { panel } = setup();
    rowAt(panel, 1).focus();
    panel.update({ t: lookup(ENGLISH) });
    expect(rowTexts(panel)[0]).toBe("Confirm: Keyboard Enter");
    expect(document.activeElement).toBe(rowAt(panel, 1));
  });

  it("redraws after the port changed outside the panel, when refresh is called", () => {
    const { panel, fake } = setup();
    fake.setSlot("confirm", 1, KEY_Q);
    panel.refresh();
    expect(rowTexts(panel)[1]).toBe("Confirmar: Teclado Q");
  });
});

describe("destroy", () => {
  it("aborts a capture in progress, removes the element, and ignores what comes after", async () => {
    const { panel, fake } = setup();
    rowAt(panel, 0).click();
    panel.destroy();
    expect(fake.captures[0]?.request.signal.aborted).toBe(true);
    expect(panel.el.isConnected).toBe(false);
    expect(panel.capturing).toBe(false);
    fake.captures[0]?.resolve({ status: "bound", conflicts: [], display: KEY_Q });
    await settle();
    expect(statusText(panel)).not.toBe("Asignado: Q");
    panel.refresh();
    panel.destroy();
  });
});
