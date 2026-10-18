// A rebinding panel: the actions of a game, each with the control that each of its slots holds, a
// "press a key" capture state, a line for the last change or conflict, and two reset buttons.
//
// The list is a menu, so it follows the menu's focus rules: the arrows, Home and End move, Enter or
// Space choose, and the mouse moves the focus too. A player who uses a pad can drive it when the game
// forwards the pad as keys (startPadMenuKeys in @velvet/core). Choosing a row starts a capture of the
// next input; Escape cancels it, and Delete clears the focused slot. Every text comes from the `t`
// lookup that the game passes in, so the panel holds no words of its own.
//
// The panel reads and writes bindings through a RebindPort. createRebindPort in @velvet/core builds
// one from an action map. The types here match that port, so the UI package has no import of core.
// Hover and focus change colour only, as the rest of the stylesheet does; the panel sets no geometry
// for them.

import { createButton } from "./components/button";
import { mergeDefined, type VuComponent } from "./components/component";
import { createElement, releaseFocus } from "./components/dom";
import { createMenu, type Menu, type MenuItem } from "./components/menu";

/** A lookup from a text key to its text. Arguments such as {action} fill the text's placeholders. */
export type TranslateFn = (key: string, args?: Readonly<Record<string, string | number>>) => string;

/** The devices that a capture may use. */
export type RebindDevice = "keyboard" | "mouse" | "gamepad";

/** What a conflict does: see the port's `rebind`. */
export type RebindPolicy = "swap" | "refuse" | "allow";

/** How a rebinding ended: "bound" (no conflict), "swapped", "allowed" or "refused" (nothing changed). */
export type RebindStatus = "bound" | "swapped" | "allowed" | "refused";

/** How a control is shown: its device, a short code, and the i18n key of its name. */
export interface RebindDisplay {
  readonly device: RebindDevice;
  readonly code: string;
  readonly labelKey: string;
  /** True for an inverted axis. */
  readonly negative?: boolean;
}

/** One slot of an action. `display` is null for an empty slot. */
export interface RebindSlot {
  readonly slot: number;
  readonly display: RebindDisplay | null;
}

/** A slot that a change found in conflict. */
export interface RebindConflict<Action extends string> {
  readonly action: Action;
  readonly slot: number;
}

/** What a capture asks for. */
export interface RebindRequest {
  readonly devices: readonly RebindDevice[];
  readonly timeoutMs: number;
  readonly policy: RebindPolicy;
  /** Aborting it cancels the capture. */
  readonly signal: AbortSignal;
}

/** A capture that was bound. */
export interface RebindResult<Action extends string> {
  readonly status: RebindStatus;
  readonly conflicts: readonly RebindConflict<Action>[];
  /** The input that was captured. */
  readonly display: RebindDisplay;
}

/** The rebinding operations that the panel drives. */
export interface RebindPort<Action extends string> {
  /** The slots of an action, in order. */
  slots(action: Action): readonly RebindSlot[];
  /** Waits for the next input and binds it to the slot. Resolves with null when cancelled, timed out or aborted. */
  capture(action: Action, slot: number, request: RebindRequest): Promise<RebindResult<Action> | null>;
  /** Empties a slot. */
  clear(action: Action, slot: number): void;
  /** Restores the defaults of one action. */
  resetAction(action: Action): void;
  /** Restores the defaults of every action. */
  resetAll(): void;
}

/** An action that the panel lists: its id, and the i18n key of its name. */
export interface RebindAction<Action extends string> {
  readonly id: Action;
  readonly labelKey: string;
}

/** The props of a rebinding panel. */
export interface RebindPanelProps<Action extends string> {
  /** The actions, in the order they are listed. */
  readonly actions: readonly RebindAction<Action>[];
  /** The text lookup. Change it when the language changes, then the panel shows the new texts. */
  readonly t: TranslateFn;
  /** What a conflict does. Default "swap". */
  readonly policy: RebindPolicy;
  /** The devices a capture may use. Default: keyboard, mouse and gamepad. */
  readonly devices: readonly RebindDevice[];
  /** Milliseconds before a capture gives up. Default 8000. */
  readonly timeoutMs: number;
}

/** Options for {@link createRebindPanel}. */
export interface RebindPanelOptions<Action extends string> {
  readonly port: RebindPort<Action>;
  readonly actions: readonly RebindAction<Action>[];
  readonly t: TranslateFn;
  readonly policy?: RebindPolicy;
  readonly devices?: readonly RebindDevice[];
  readonly timeoutMs?: number;
}

/** A rebinding panel. */
export interface RebindPanel<Action extends string> extends VuComponent<RebindPanelProps<Action>> {
  /** Reads the slots again from the port and redraws the list. Call it after a change made elsewhere. */
  refresh(): void;
  /** Cancels the capture in progress. Does nothing when none runs. */
  cancel(): void;
  /** True while a capture waits for an input. */
  readonly capturing: boolean;
}

/** One row of the list: a slot of an action. */
interface Row<Action extends string> {
  readonly action: RebindAction<Action>;
  readonly slot: number;
  readonly display: RebindDisplay | null;
}

/** The capture in progress: its row, a token that identifies it, and the controller that cancels it. */
interface ActiveCapture {
  readonly index: number;
  readonly token: object;
  readonly controller: AbortController;
}

const DEFAULT_DEVICES: readonly RebindDevice[] = ["keyboard", "mouse", "gamepad"];
const DEFAULT_TIMEOUT_MS = 8000;

/**
 * After a capture that used the mouse, the click that follows it must not start another capture.
 * Choices within this many milliseconds of such a capture are ignored.
 */
const CLICK_COOLDOWN_MS = 400;

/** The width of the panel, in logical pixels. */
const PANEL_WIDTH = 960;

/** The tallest the list gets before it scrolls, in logical pixels. */
const LIST_MAX_HEIGHT = 640;

/**
 * Creates a rebinding panel. It starts with the list drawn. Append `el` to the stage's root or to any
 * element of the page.
 * @param options - the port, the actions and the text lookup.
 */
export function createRebindPanel<Action extends string>(options: RebindPanelOptions<Action>): RebindPanel<Action> {
  const port = options.port;
  let current: RebindPanelProps<Action> = {
    actions: options.actions,
    t: options.t,
    policy: options.policy ?? "swap",
    devices: options.devices ?? DEFAULT_DEVICES,
    timeoutMs: options.timeoutMs ?? DEFAULT_TIMEOUT_MS,
  };
  let rows: Row<Action>[] = [];
  let capture: ActiveCapture | null = null;
  let ignoreChoicesUntil = 0;
  let destroyed = false;

  const el = createElement("div", "vu-plate vu-rebind");
  el.setAttribute("role", "region");
  el.style.width = `${PANEL_WIDTH}px`;
  el.style.maxWidth = "100%";
  const title = createElement("div", "vu-plate-title");
  const status = createElement("p", "vu-plate-body vu-rebind-status");
  status.setAttribute("aria-live", "polite");
  const menu: Menu = createMenu({ items: [], label: "", onSelect: (id) => startCapture(Number(id)) });
  menu.el.style.maxHeight = `${LIST_MAX_HEIGHT}px`;
  menu.el.style.overflowY = "auto";
  const buttons = createElement("div", "vu-rebind-actions");
  buttons.style.display = "flex";
  buttons.style.flexWrap = "wrap";
  buttons.style.gap = "14px";
  const resetOne = createButton({ label: "", onPress: () => resetFocused() });
  const resetEvery = createButton({ label: "", onPress: () => resetAll() });
  buttons.append(resetOne.el, resetEvery.el);
  el.append(title, status, menu.el, buttons);
  el.addEventListener("keydown", onKeyDown);

  /** The text of a key, or the fallback text when the lookup has none (it then returns the key). */
  function templated(key: string, args: Readonly<Record<string, string | number>>, fallback: string): string {
    const text = current.t(key, args);
    return text === key ? fallback : text;
  }

  /** The text of an action's name, or its id when the lookup has none. */
  function actionText(id: Action): string {
    const action = current.actions.find((candidate) => candidate.id === id);
    if (action === undefined) return id;
    const text = current.t(action.labelKey);
    return text === action.labelKey ? id : text;
  }

  /** The text of a device, such as keyboard or gamepad. */
  function deviceText(device: RebindDevice): string {
    const key = `input.device.${device}`;
    const text = current.t(key);
    return text === key ? device : text;
  }

  /** The text of a bound control: its name, marked when the axis is inverted. */
  function controlText(display: RebindDisplay): string {
    const key = display.labelKey;
    const name = current.t(key);
    const base = name === key ? display.code : name;
    if (display.negative !== true) return base;
    return templated("input.rebind.inverted", { control: base }, `${base} -`);
  }

  /** The label of a row: the action with its slot's control, or the capture or empty state. */
  function rowLabel(row: Row<Action>, index: number): string {
    const name = actionText(row.action.id);
    if (capture !== null && capture.index === index) {
      return templated("input.rebind.row.capturing", { action: name }, `${name}: ...`);
    }
    if (row.display === null) {
      return templated("input.rebind.row.empty", { action: name }, `${name}: -`);
    }
    const device = deviceText(row.display.device);
    const control = controlText(row.display);
    return templated(
      "input.rebind.row",
      { action: name, device, control },
      `${name}: ${device} ${control}`,
    );
  }

  /** The line that says what a finished capture did. */
  function messageFor(result: RebindResult<Action>): string {
    const control = controlText(result.display);
    const first = result.conflicts[0];
    const other = first === undefined ? "" : actionText(first.action);
    switch (result.status) {
      case "bound":
        return current.t("input.rebind.bound", { control });
      case "swapped":
        return current.t("input.rebind.swapped", { control, action: other });
      case "allowed":
        return current.t("input.rebind.shared", { control, action: other });
      case "refused":
        return current.t("input.rebind.refused", { control, action: other });
    }
  }

  /** The rows of the list: one per slot, and one empty row for a new binding when every slot is taken. */
  function buildRows(): Row<Action>[] {
    const next: Row<Action>[] = [];
    for (const action of current.actions) {
      const slots = port.slots(action.id);
      for (const slot of slots) next.push({ action, slot: slot.slot, display: slot.display });
      if (slots.every((slot) => slot.display !== null)) {
        next.push({ action, slot: slots.length, display: null });
      }
    }
    return next;
  }

  function setStatus(text: string): void {
    status.textContent = text;
  }

  /** Reads the rows again and redraws the list and the buttons. Keeps the focus where it is. */
  function refresh(): void {
    if (destroyed) return;
    rows = buildRows();
    const items: MenuItem[] = rows.map((row, index) => ({ id: String(index), label: rowLabel(row, index) }));
    menu.update({ items, label: current.t("input.rebind.list") });
    title.textContent = current.t("input.rebind.title");
    resetOne.update({ label: current.t("input.rebind.reset.action") });
    resetEvery.update({ label: current.t("input.rebind.reset.all") });
  }

  /** Starts a capture for the row at `index`. Nothing happens while one runs or for an unknown row. */
  function startCapture(index: number): void {
    const row = rows[index];
    if (row === undefined || capture !== null || destroyed) return;
    if (Date.now() < ignoreChoicesUntil) return;
    const token = {};
    const controller = new AbortController();
    capture = { index, token, controller };
    setStatus(current.t("input.rebind.capture"));
    refresh();
    port
      .capture(row.action.id, row.slot, {
        devices: current.devices,
        timeoutMs: current.timeoutMs,
        policy: current.policy,
        signal: controller.signal,
      })
      .then(
        (result) => finishCapture(token, result, false),
        () => finishCapture(token, null, true),
      );
  }

  /** Ends the capture that owns `token`, and reports its result. A capture that was replaced is ignored. */
  function finishCapture(token: object, result: RebindResult<Action> | null, failed: boolean): void {
    if (capture === null || capture.token !== token || destroyed) return;
    capture = null;
    if (result === null) {
      setStatus(current.t(failed ? "input.rebind.failed" : "input.rebind.cancelled"));
    } else {
      setStatus(messageFor(result));
      if (result.display.device === "mouse") ignoreChoicesUntil = Date.now() + CLICK_COOLDOWN_MS;
    }
    refresh();
  }

  /** Resets the action of the focused row to its defaults. */
  function resetFocused(): void {
    if (capture !== null || destroyed) return;
    const row = rows[menu.focusedIndex];
    if (row === undefined) return;
    port.resetAction(row.action.id);
    refresh();
    setStatus(current.t("input.rebind.reset.done"));
  }

  /** Resets every action to its defaults. */
  function resetAll(): void {
    if (capture !== null || destroyed) return;
    port.resetAll();
    refresh();
    setStatus(current.t("input.rebind.reset.done"));
  }

  /** Delete clears the focused slot, when no capture runs and the key came from the list. */
  function onKeyDown(event: KeyboardEvent): void {
    if (event.key !== "Delete" || capture !== null || destroyed) return;
    if (!(event.target instanceof Node) || !menu.el.contains(event.target)) return;
    const row = rows[menu.focusedIndex];
    if (row === undefined) return;
    event.preventDefault();
    port.clear(row.action.id, row.slot);
    refresh();
    setStatus(current.t("input.rebind.cleared"));
  }

  setStatus(current.t("input.rebind.hint"));
  refresh();

  return {
    el,
    get capturing(): boolean {
      return capture !== null;
    },
    refresh,
    cancel(): void {
      capture?.controller.abort();
    },
    update(next: Partial<RebindPanelProps<Action>>): void {
      current = mergeDefined(current, next);
      refresh();
    },
    destroy(): void {
      if (destroyed) return;
      destroyed = true;
      const active = capture;
      capture = null;
      active?.controller.abort();
      el.removeEventListener("keydown", onKeyDown);
      releaseFocus(el);
      menu.destroy();
      resetOne.destroy();
      resetEvery.destroy();
      el.remove();
    },
  };
}
