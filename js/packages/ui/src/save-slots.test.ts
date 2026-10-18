// @vitest-environment happy-dom
// Tests for the save list: the rows in each mode, picking a row to load or to write it, the question that comes
// before a write over a save and before a delete, the keys, the damaged slots, the failures and the autosave.

import { afterEach, describe, expect, it, vi } from "vitest";
import {
  createSaveSlots,
  formatPlayTime,
  formatSaveTime,
  isSafeThumbnail,
  type SaveSlotDraft,
  type SaveSlotRecord,
  type SaveSlotStore,
  type SaveSlots,
} from "./save-slots";

afterEach(() => {
  document.body.innerHTML = "";
});

/** A store in memory, with the write time fixed, and the names of the calls it got. */
function memoryStore(initial: Record<string, unknown> = {}): { store: SaveSlotStore; data: Map<string, unknown>; calls: string[] } {
  const data = new Map<string, unknown>(Object.entries(initial));
  const calls: string[] = [];
  const store: SaveSlotStore = {
    async list() {
      calls.push("list");
      return [...data.keys()].sort();
    },
    async load(slot) {
      calls.push(`load:${slot}`);
      return data.has(slot) ? { savedAt: "2026-10-18T10:00:00.000Z", data: data.get(slot) } : null;
    },
    async save(slot, value) {
      calls.push(`save:${slot}`);
      data.set(slot, value);
    },
    async remove(slot) {
      calls.push(`remove:${slot}`);
      data.delete(slot);
    },
  };
  return { store, data, calls };
}

/** A stored record in the shape the list writes. */
function record(title: string, playTimeMs: number, state: unknown, thumbnail?: string): unknown {
  return { meta: { title, playTimeMs, ...(thumbnail === undefined ? {} : { thumbnail }) }, state };
}

/** Lets pending promises run. */
async function settle(): Promise<void> {
  for (let round = 0; round < 5; round += 1) {
    await new Promise((resolve) => setTimeout(resolve, 0));
  }
}

/** The row element of a slot. */
function row(el: HTMLElement, slot: string): HTMLElement {
  const node = el.querySelector<HTMLElement>(`[data-slot="${slot}"]`);
  if (node === null) throw new Error(`no row ${slot}`);
  return node;
}

/** The rows of the list, in order, by slot name. */
function rowNames(el: HTMLElement): string[] {
  return Array.from(el.querySelectorAll<HTMLElement>(".vu-slot")).map((node) => node.dataset["slot"] ?? "");
}

/** A draft that the game gives when it saves. */
const draft = (title = "Town", playTimeMs = 3723000): SaveSlotDraft => ({ title, playTimeMs, state: { gold: 12 } });

/** Builds the list in the document. */
function setup(options: Parameters<typeof createSaveSlots>[0]): { slots: SaveSlots; el: HTMLElement } {
  const slots = createSaveSlots(options);
  document.body.append(slots.el);
  return { slots, el: slots.el };
}

describe("the rows", () => {
  it("lists the autosave first and then the manual slots, with the empty ones marked empty", async () => {
    const { store } = memoryStore({ "slot-2": record("Woods", 60000, {}) });
    const { el } = setup({ store, collect: () => draft() });
    await settle();
    expect(rowNames(el)).toEqual(["auto", "slot-1", "slot-2", "slot-3"]);
    expect(row(el, "auto").dataset["state"]).toBe("empty");
    expect(row(el, "slot-2").dataset["state"]).toBe("filled");
    expect(row(el, "slot-2").querySelector(".vu-slot-title")?.textContent).toBe("Woods");
  });

  it("names the rows and shows the play time and the time of each save", async () => {
    const { store } = memoryStore({ "slot-1": record("Port", 3723000, {}) });
    const { el } = setup({ store, locale: "en-US", collect: () => draft() });
    await settle();
    expect(row(el, "slot-1").querySelector(".vu-slot-name")?.textContent).toBe("Slot 1");
    expect(row(el, "slot-1").querySelector(".vu-slot-detail")?.textContent).toContain("1:02:03");
    expect(row(el, "auto").querySelector(".vu-slot-name")?.textContent).toBe("Autosave");
  });

  it("uses the texts of the lookup, and the English fallbacks when a key has none", async () => {
    const { store } = memoryStore();
    const t = vi.fn((key: string) => (key === "save.empty" ? "Vacía" : key));
    const { el } = setup({ store, t, collect: () => draft() });
    await settle();
    expect(row(el, "slot-1").querySelector(".vu-slot-title")?.textContent).toBe("Vacía");
    expect(row(el, "slot-1").querySelector(".vu-slot-name")?.textContent).toBe("Slot 1");
  });

  it("leaves out the autosave row when it is turned off", async () => {
    const { store } = memoryStore();
    const { el } = setup({ store, autosave: null, manualSlots: 2 });
    await settle();
    expect(rowNames(el)).toEqual(["slot-1", "slot-2"]);
  });

  it("draws a thumbnail only from a PNG, JPEG or WebP data URL", async () => {
    const pixel = "data:image/png;base64,iVBORw0KGgo=";
    const { store } = memoryStore({
      "slot-1": record("With picture", 0, {}, pixel),
      "slot-2": record("External", 0, {}, "https://example.com/x.png"),
    });
    const { el } = setup({ store, mode: "load" });
    await settle();
    expect(row(el, "slot-1").querySelector<HTMLImageElement>("img.vu-slot-thumb")?.src).toBe(pixel);
    expect(row(el, "slot-2").querySelector("img")).toBeNull();
  });
});

describe("load mode", () => {
  it("can pick only the saved slots, and hands the saved record to onLoad", async () => {
    const onLoad = vi.fn<(saved: SaveSlotRecord) => void>();
    const { store, calls } = memoryStore({ "slot-2": record("Woods", 90000, { gold: 40 }) });
    const { el } = setup({ store, mode: "load", onLoad });
    await settle();
    expect(row(el, "slot-1").getAttribute("aria-disabled")).toBe("true");
    expect(row(el, "slot-2").hasAttribute("aria-disabled")).toBe(false);
    row(el, "slot-1").click();
    await settle();
    expect(onLoad).not.toHaveBeenCalled();
    row(el, "slot-2").click();
    await settle();
    expect(onLoad).toHaveBeenCalledTimes(1);
    const saved = onLoad.mock.calls[0]?.[0];
    expect(saved?.slot).toBe("slot-2");
    expect(saved?.meta.title).toBe("Woods");
    expect(saved?.state).toEqual({ gold: 40 });
    expect(calls).toContain("load:slot-2");
  });

  it("shows a damaged slot as damaged, and does not let it be loaded", async () => {
    const onLoad = vi.fn();
    const { store } = memoryStore({ "slot-1": "not a record" });
    const { el } = setup({ store, mode: "load", onLoad });
    await settle();
    expect(row(el, "slot-1").dataset["state"]).toBe("damaged");
    expect(row(el, "slot-1").querySelector(".vu-slot-title")?.textContent).toBe("Damaged save");
    row(el, "slot-1").click();
    await settle();
    expect(onLoad).not.toHaveBeenCalled();
  });

  it("moves the focus with the arrows over the rows that can be picked, wrapping at the ends", async () => {
    const { store } = memoryStore({ "slot-1": record("A", 0, {}), "slot-3": record("C", 0, {}) });
    const { el, slots } = setup({ store, mode: "load" });
    await settle();
    expect(slots.focusTarget()).toBe(row(el, "slot-1"));
    row(el, "slot-1").focus();
    const press = (key: string): void => {
      el.querySelector(".vu-slots-list")?.dispatchEvent(new KeyboardEvent("keydown", { key, bubbles: true, cancelable: true }));
    };
    press("ArrowDown");
    expect(document.activeElement).toBe(row(el, "slot-3"));
    press("ArrowDown");
    expect(document.activeElement).toBe(row(el, "slot-1"));
    press("ArrowUp");
    expect(document.activeElement).toBe(row(el, "slot-3"));
    press("Home");
    expect(document.activeElement).toBe(row(el, "slot-1"));
    press("End");
    expect(document.activeElement).toBe(row(el, "slot-3"));
  });

  it("gives the focus to no row when the list has nothing that can be picked", async () => {
    const { store } = memoryStore();
    const { slots } = setup({ store, mode: "load" });
    await settle();
    expect(slots.focusTarget()).toBeNull();
  });
});

describe("save mode", () => {
  it("writes an empty manual slot at once, with the draft's meta and state", async () => {
    const onSaved = vi.fn<(saved: SaveSlotRecord) => void>();
    const collect = vi.fn((slot: string) => draft(`Save ${slot}`, 1000));
    const { store, data } = memoryStore();
    const { el } = setup({ store, mode: "save", collect, onSaved });
    await settle();
    row(el, "slot-1").click();
    await settle();
    expect(collect).toHaveBeenCalledWith("slot-1");
    expect(data.get("slot-1")).toEqual({ meta: { title: "Save slot-1", playTimeMs: 1000 }, state: { gold: 12 } });
    expect(onSaved).toHaveBeenCalledTimes(1);
    expect(onSaved.mock.calls[0]?.[0].state).toEqual({ gold: 12 });
    expect(row(el, "slot-1").dataset["state"]).toBe("filled");
  });

  it("keeps the autosave read-only in save mode", async () => {
    const collect = vi.fn(() => draft());
    const { store } = memoryStore();
    const { el } = setup({ store, mode: "save", collect });
    await settle();
    expect(row(el, "auto").getAttribute("aria-disabled")).toBe("true");
    row(el, "auto").click();
    await settle();
    expect(collect).not.toHaveBeenCalled();
  });

  it("asks before it writes over a save, and writes only after a yes", async () => {
    const collect = vi.fn(() => draft());
    const { store, calls } = memoryStore({ "slot-2": record("Old", 0, {}) });
    const { el } = setup({ store, mode: "save", collect });
    await settle();
    row(el, "slot-2").click();
    await settle();
    const modal = el.querySelector<HTMLElement>(".vu-modal");
    expect(modal?.hidden).toBe(false);
    expect(calls).not.toContain("save:slot-2");
    el.querySelector<HTMLButtonElement>('[data-action="yes"]')?.click();
    await settle();
    expect(calls).toContain("save:slot-2");
    expect(modal?.hidden).toBe(true);
  });

  it("does not write over a save when the player says no", async () => {
    const collect = vi.fn(() => draft());
    const { store, calls } = memoryStore({ "slot-2": record("Old", 0, {}) });
    const { el } = setup({ store, mode: "save", collect });
    await settle();
    row(el, "slot-2").click();
    await settle();
    el.querySelector<HTMLButtonElement>('[data-action="no"]')?.click();
    await settle();
    expect(calls.some((call) => call.startsWith("save:"))).toBe(false);
  });

  it("refuses a draft whose play time is not a number of milliseconds, and reports it", async () => {
    const onError = vi.fn();
    const { store, data } = memoryStore();
    const { el } = setup({ store, mode: "save", collect: () => draft("x", Number.NaN), onError });
    await settle();
    row(el, "slot-1").click();
    await settle();
    expect(data.has("slot-1")).toBe(false);
    expect(onError).toHaveBeenCalledWith(expect.any(RangeError), "slot-1");
    expect(el.querySelector(".vu-slots-status")?.textContent).toBe("Something went wrong.");
  });

  it("refuses a thumbnail that is not an image data URL", async () => {
    const onError = vi.fn();
    const { store, data } = memoryStore();
    const bad: SaveSlotDraft = { title: "x", playTimeMs: 0, thumbnail: "https://example.com/a.png", state: {} };
    const { el } = setup({ store, mode: "save", collect: () => bad, onError });
    await settle();
    row(el, "slot-1").click();
    await settle();
    expect(data.has("slot-1")).toBe(false);
    expect(onError).toHaveBeenCalledWith(expect.any(RangeError), "slot-1");
  });

  it("writes the autosave slot from collect, without a question, when autosave is called", async () => {
    const collect = vi.fn((slot: string) => draft(`Auto ${slot}`));
    const { store, data } = memoryStore();
    const { slots } = setup({ store, mode: "save", collect });
    await settle();
    await slots.autosave();
    expect(collect).toHaveBeenCalledWith("auto");
    expect(data.get("auto")).toEqual({ meta: { title: "Auto auto", playTimeMs: 3723000 }, state: { gold: 12 } });
  });

  it("rejects from autosave when the store fails, so the game can react", async () => {
    const { store } = memoryStore();
    store.save = () => Promise.reject(new Error("disk full"));
    const { slots } = setup({ store, mode: "save", collect: () => draft() });
    await settle();
    await expect(slots.autosave()).rejects.toThrow("disk full");
  });
});

describe("delete", () => {
  it("asks before it deletes the focused row, and deletes after a yes", async () => {
    const onDeleted = vi.fn();
    const { store, data } = memoryStore({ "slot-1": record("Delete me", 0, {}) });
    const { el } = setup({ store, mode: "load", onDeleted });
    await settle();
    row(el, "slot-1").focus();
    el.querySelector(".vu-slots-list")?.dispatchEvent(new KeyboardEvent("keydown", { key: "Delete", bubbles: true, cancelable: true }));
    await settle();
    expect(el.querySelector<HTMLElement>(".vu-modal")?.hidden).toBe(false);
    expect(data.has("slot-1")).toBe(true);
    el.querySelector<HTMLButtonElement>('[data-action="yes"]')?.click();
    await settle();
    expect(data.has("slot-1")).toBe(false);
    expect(onDeleted).toHaveBeenCalledWith("slot-1");
    expect(row(el, "slot-1").dataset["state"]).toBe("empty");
  });

  it("does not delete when the player says no", async () => {
    const { store, data } = memoryStore({ "slot-1": record("Keep me", 0, {}) });
    const { el } = setup({ store, mode: "load" });
    await settle();
    row(el, "slot-1").focus();
    el.querySelector(".vu-slots-list")?.dispatchEvent(new KeyboardEvent("keydown", { key: "Delete", bubbles: true, cancelable: true }));
    await settle();
    el.querySelector<HTMLButtonElement>('[data-action="no"]')?.click();
    await settle();
    expect(data.has("slot-1")).toBe(true);
  });

  it("deletes with the footer button, which acts on the row that had the focus", async () => {
    const { store, data } = memoryStore({ "slot-3": record("Footer", 0, {}) });
    const { el } = setup({ store, mode: "load" });
    await settle();
    row(el, "slot-3").focus();
    const footerButton = el.querySelector<HTMLButtonElement>(".vu-slots-footer .vu-button");
    footerButton?.click();
    await settle();
    el.querySelector<HTMLButtonElement>('[data-action="yes"]')?.click();
    await settle();
    expect(data.has("slot-3")).toBe(false);
  });
});

describe("the keys and the list", () => {
  it("keeps the focus on the same row after the list is drawn again", async () => {
    const { store } = memoryStore({ "slot-1": record("A", 0, {}) });
    const { el, slots } = setup({ store, mode: "load" });
    await settle();
    row(el, "slot-1").focus();
    await slots.refresh();
    expect(document.activeElement).toBe(row(el, "slot-1"));
  });

  it("draws the rows again for a new mode: empty manual slots can be written in save mode", async () => {
    const { store } = memoryStore({ "slot-2": record("B", 0, {}) });
    const { el, slots } = setup({ store, mode: "load", collect: () => draft() });
    await settle();
    expect(row(el, "slot-1").getAttribute("aria-disabled")).toBe("true");
    expect(row(el, "auto").getAttribute("aria-disabled")).toBe("true");
    slots.update({ mode: "save" });
    await settle();
    expect(row(el, "slot-1").hasAttribute("aria-disabled")).toBe(false);
    expect(row(el, "auto").getAttribute("aria-disabled")).toBe("true");
  });

  it("can not be saved into in save mode when the game gives no collect function", async () => {
    const { store } = memoryStore();
    const { el } = setup({ store, mode: "save" });
    await settle();
    expect(row(el, "slot-1").getAttribute("aria-disabled")).toBe("true");
  });

  it("reports a failed list to onError with no slot, and shows the rows as empty", async () => {
    const onError = vi.fn();
    const { store } = memoryStore();
    store.list = () => Promise.reject(new Error("offline"));
    const { el } = setup({ store, onError });
    await settle();
    expect(onError).toHaveBeenCalledWith(expect.any(Error), null);
    expect(row(el, "slot-1").dataset["state"]).toBe("empty");
    expect(el.querySelector(".vu-slots-status")?.textContent).toBe("Something went wrong.");
  });

  it("reports a failed write to onError with its slot, and keeps the list", async () => {
    const onError = vi.fn();
    const { store } = memoryStore();
    store.save = () => Promise.reject(new Error("quota"));
    const { el } = setup({ store, mode: "save", collect: () => draft(), onError });
    await settle();
    row(el, "slot-2").click();
    await settle();
    expect(onError).toHaveBeenCalledWith(expect.any(Error), "slot-2");
    expect(rowNames(el)).toEqual(["auto", "slot-1", "slot-2", "slot-3"]);
  });

  it("refuses a slot name that the save store would refuse", () => {
    const { store } = memoryStore();
    expect(() => createSaveSlots({ store, slotPrefix: "bad name " })).toThrow(RangeError);
    expect(() => createSaveSlots({ store, autosave: "no/good" })).toThrow(RangeError);
    expect(() => createSaveSlots({ store, manualSlots: 0 })).toThrow(RangeError);
    expect(() => createSaveSlots({ store, manualSlots: 100 })).toThrow(RangeError);
  });

  it("removes its element on destroy", async () => {
    const { store } = memoryStore();
    const { slots, el } = setup({ store });
    await settle();
    slots.destroy();
    expect(el.isConnected).toBe(false);
  });
});

describe("the small helpers", () => {
  it("writes a play time as hours, minutes and seconds", () => {
    expect(formatPlayTime(0)).toBe("0:00:00");
    expect(formatPlayTime(3723000)).toBe("1:02:03");
    expect(formatPlayTime(59999)).toBe("0:00:59");
    expect(formatPlayTime(-5)).toBe("0:00:00");
    expect(formatPlayTime(Number.NaN)).toBe("0:00:00");
  });

  it("writes a save time for a language, and gives empty text for a time that cannot be read", () => {
    expect(formatSaveTime("not a date", null)).toBe("");
    expect(formatSaveTime("2026-10-18T10:00:00.000Z", "en-US")).not.toBe("");
  });

  it("accepts only PNG, JPEG and WebP data URLs as thumbnails", () => {
    expect(isSafeThumbnail("data:image/png;base64,iVBORw0KGgo=")).toBe(true);
    expect(isSafeThumbnail("data:image/webp;base64,UklGRg==")).toBe(true);
    expect(isSafeThumbnail("data:image/svg+xml;base64,PHN2Zz4=")).toBe(false);
    expect(isSafeThumbnail("javascript:alert(1)")).toBe(false);
    expect(isSafeThumbnail(12)).toBe(false);
  });
});
