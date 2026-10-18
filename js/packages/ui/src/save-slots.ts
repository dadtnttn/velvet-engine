// Save slots: the list of a game's save slots, for saving and for loading. The autosave is one row, and
// each manual slot is another. A row shows the slot's title, its play time, when it was written and a
// thumbnail, or says that the slot is empty or damaged.
//
// Keys: the arrows move between the rows (Home and End go to the first and the last), and Enter or Space
// picks the focused row. Delete asks to delete it. A pad drives the list through the same keys (see
// startPadMenuKeys in @velvet/core). Writing into a slot that holds a save, and every delete, asks first.
//
// The list works over a store of the save slots: createSaveStore from @velvet/core fits the SaveSlotStore
// port, so this module imports nothing from core. Each slot holds a record { meta, state }. The meta is what
// the list shows (title, play time, thumbnail), and the state is the game's own data. The game gives both
// through `collect` when it saves, and reads the state back in `onLoad`.

import { createButton } from "./components/button";
import type { VuComponent } from "./components/component";
import { createElement, releaseFocus } from "./components/dom";
import { createModal, type Modal } from "./components/modal";
import type { TranslateFn } from "./rebind-panel";

/** The two uses of the list: "save" writes a manual slot, "load" reads a slot that holds a save. */
export type SaveSlotsMode = "save" | "load";

/** What the list shows for a slot. */
export interface SaveSlotMeta {
  /** The title of the save, such as the place or the day. */
  readonly title: string;
  /** The play time, in milliseconds. */
  readonly playTimeMs: number;
  /** A thumbnail, as a PNG, JPEG or WebP data URL. Other sources are not shown. */
  readonly thumbnail?: string;
}

/** What the game gives the list when it saves. */
export interface SaveSlotDraft {
  readonly title: string;
  readonly playTimeMs: number;
  /** A PNG, JPEG or WebP data URL, or null for no thumbnail. */
  readonly thumbnail?: string | null;
  /** The game's data. It must be JSON-serialisable and must not be undefined. */
  readonly state: unknown;
}

/** A slot that holds a save, as the list reads it. */
export interface SaveSlotRecord {
  /** The name of the slot. */
  readonly slot: string;
  /** When the save was written, as an ISO 8601 string. */
  readonly savedAt: string;
  readonly meta: SaveSlotMeta;
  /** The game's data, as it was saved. */
  readonly state: unknown;
}

/**
 * The part of a save store that the list uses. `createSaveStore` from @velvet/core fits it: its `load`
 * gives an object with `savedAt` and `data`, or null for an empty slot.
 */
export interface SaveSlotStore {
  /** The names of the slots that hold a save. */
  list(): Promise<string[]>;
  /** Reads a slot. Resolves to null when it is empty. Rejects when the save cannot be read. */
  load(slot: string): Promise<{ readonly savedAt: string; readonly data: unknown } | null>;
  /** Writes a slot, replacing any save there. */
  save(slot: string, data: unknown): Promise<void>;
  /** Deletes a slot. Deleting an empty slot is not an error. */
  remove(slot: string): Promise<void>;
}

/** Props of the save list. */
export interface SaveSlotsProps {
  /** "load" (the default) or "save". */
  readonly mode?: SaveSlotsMode;
  /** The text lookup. Without one, the English fallbacks are used. */
  readonly t?: TranslateFn;
  /** The language tag for the dates, such as "es-ES". Null (the default) uses the page's language. */
  readonly locale?: string | null;
}

/** Options for {@link createSaveSlots}. */
export interface SaveSlotsOptions extends SaveSlotsProps {
  /** The store that holds the slots. */
  readonly store: SaveSlotStore;
  /** How many manual slots there are, from 1 to 99. Default 3. */
  readonly manualSlots?: number;
  /** The name of the autosave slot. Default "auto". Null for no autosave row. */
  readonly autosave?: string | null;
  /** The start of the manual slot names: slot i is `slotPrefix` followed by i. Default "slot-". */
  readonly slotPrefix?: string;
  /** Gives what to save into a slot. Needed to save, and for the autosave. */
  readonly collect?: (slot: string) => SaveSlotDraft | Promise<SaveSlotDraft>;
  /** Runs with the record of a slot that was read, after the player picked it in load mode. */
  readonly onLoad?: (record: SaveSlotRecord) => void;
  /** Runs with the record of a slot that was written, after the player picked it in save mode. */
  readonly onSaved?: (record: SaveSlotRecord) => void;
  /** Runs with the name of a slot that was deleted. */
  readonly onDeleted?: (slot: string) => void;
  /** Runs when a read, write or delete fails. `slot` is null when the list of slots could not be read. */
  readonly onError?: (error: unknown, slot: string | null) => void;
}

/** The save list. It is a component: append `el` to a screen, or to any element of the page. */
export interface SaveSlots extends VuComponent<SaveSlotsProps> {
  /** Reads every slot again and draws the rows. Called by the list itself when it is made and when a value changes. */
  refresh(): Promise<void>;
  /**
   * Writes the autosave slot now, with no prompt, from `collect`. Rejects when the save fails, so the game
   * can react. It does nothing when the list has no autosave.
   */
  autosave(): Promise<void>;
  /** The row that takes the focus when the list opens: the first row that can be picked, or null. */
  focusTarget(): HTMLElement | null;
}

/** A row of the list, with what was read from its slot. */
interface Row {
  readonly slot: string;
  readonly isAutosave: boolean;
  /** The row's name, such as "Slot 2" or "Autosave". */
  readonly label: string;
  readonly record: SaveSlotRecord | null;
  /** True when the slot holds something that cannot be read. */
  readonly damaged: boolean;
}

/** The shape of a slot name, the same as the core store accepts. */
const SLOT_NAME = /^[A-Za-z0-9_-]{1,64}$/;

/** The default number of manual slots, and the largest number that the list takes. */
const DEFAULT_MANUAL_SLOTS = 3;
const MAX_MANUAL_SLOTS = 99;

/** The shape of a thumbnail that the list shows: a PNG, JPEG or WebP data URL, and nothing else. */
const THUMBNAIL = /^data:image\/(?:png|jpeg|webp);base64,[A-Za-z0-9+/]+=*$/;

/** Creates the save list. It starts reading the slots at once. */
export function createSaveSlots(options: SaveSlotsOptions): SaveSlots {
  const store = options.store;
  const manualCount = options.manualSlots ?? DEFAULT_MANUAL_SLOTS;
  if (!Number.isInteger(manualCount) || manualCount < 1 || manualCount > MAX_MANUAL_SLOTS) {
    throw new RangeError(`manualSlots must be a whole number from 1 to ${MAX_MANUAL_SLOTS}, got ${manualCount}`);
  }
  const autosaveName = options.autosave === undefined ? "auto" : options.autosave;
  const prefix = options.slotPrefix ?? "slot-";
  const manualNames = Array.from({ length: manualCount }, (_, index) => `${prefix}${index + 1}`);
  for (const name of autosaveName === null ? manualNames : [autosaveName, ...manualNames]) {
    if (!SLOT_NAME.test(name)) {
      throw new RangeError(`slot "${name}" is not a valid name: use 1 to 64 letters, digits, "_" or "-"`);
    }
  }
  const collect = options.collect;
  const initialMode: SaveSlotsMode = options.mode ?? "load";
  let state: { mode: SaveSlotsMode; t: TranslateFn | undefined; locale: string | null } = {
    mode: initialMode,
    t: options.t,
    locale: options.locale ?? null,
  };
  let destroyed = false;
  let busy = false;
  let rows: Row[] = [];
  let focusedSlot: string | null = null;
  let refreshCount = 0;

  const el = createElement("div", "vu-slots");
  const status = createElement("p", "vu-slots-status");
  status.setAttribute("aria-live", "polite");
  const list = createElement("div", "vu-slots-list");
  list.setAttribute("role", "group");
  const footer = createElement("div", "vu-slots-footer");
  const deleteButton = createButton({ label: "", onPress: () => void deleteFocused() });
  footer.append(deleteButton.el);
  const modal: Modal = createModal({ title: "", body: "", actions: [] });
  el.append(status, list, footer, modal.el);

  /** The text of a key, or its English fallback when the lookup has no text for it. */
  const text = (key: string, args: Readonly<Record<string, string | number>> | undefined, fallback: string): string => {
    if (state.t === undefined) return fallback;
    const value = state.t(key, args);
    return value === key ? fallback : value;
  };

  const setStatus = (key: string, fallback: string): void => {
    status.textContent = text(key, undefined, fallback);
  };

  /** Reports a failure to the game, and says so on the list. */
  const report = (error: unknown, slot: string | null): void => {
    options.onError?.(error, slot);
    setStatus("save.status.failed", "Something went wrong.");
  };

  /** The name of a row: its label, with the slot's number for a manual slot. */
  const labelOf = (isAutosave: boolean, index: number): string =>
    isAutosave ? text("save.autosave", undefined, "Autosave") : text("save.slot", { number: index + 1 }, `Slot ${index + 1}`);

  /** True when the row can be picked in the current mode. */
  const pickable = (row: Row): boolean => {
    if (state.mode === "save") return !row.isAutosave && collect !== undefined;
    return row.record !== null;
  };

  /** The line that says what a filled row holds: its play time and when it was written. */
  const detailOf = (row: Row): string => {
    const record = row.record;
    if (record === null) return "";
    const parts = [formatPlayTime(record.meta.playTimeMs), formatSaveTime(record.savedAt, state.locale)];
    return parts.filter((part) => part !== "").join(" · ");
  };

  /** The title line of a row: the save's title, or that the slot is empty or damaged. */
  const titleOf = (row: Row): string => {
    if (row.damaged) return text("save.damaged", undefined, "Damaged save");
    if (row.record === null) return text("save.empty", undefined, "Empty");
    return row.record.meta.title === "" ? text("save.untitled", undefined, "Untitled") : row.record.meta.title;
  };

  /** Draws one row as a button, with its name, its title, its detail and its thumbnail. */
  const rowElement = (row: Row): HTMLElement => {
    const node = createElement("button", "vu-slot");
    node.type = "button";
    node.dataset["slot"] = row.slot;
    node.dataset["state"] = row.damaged ? "damaged" : row.record === null ? "empty" : "filled";
    if (!pickable(row)) node.setAttribute("aria-disabled", "true");
    const main = createElement("span", "vu-slot-main");
    main.append(
      createElement("span", "vu-slot-name", row.label),
      createElement("span", "vu-slot-title", titleOf(row)),
    );
    const detail = detailOf(row);
    if (detail !== "") main.append(createElement("span", "vu-slot-detail", detail));
    node.append(main);
    const thumbnail = row.record?.meta.thumbnail;
    if (thumbnail !== undefined) {
      const image = document.createElement("img");
      image.className = "vu-slot-thumb";
      image.alt = "";
      image.src = thumbnail;
      node.append(image);
    }
    node.addEventListener("click", () => {
      if (pickable(row)) void pick(row);
    });
    return node;
  };

  /** Draws the rows again. The focus stays on the same row when the list had it. */
  const paint = (): void => {
    const hadFocus = el.contains(document.activeElement);
    const wanted = focusedSlot;
    list.replaceChildren(...rows.map(rowElement));
    deleteButton.update({ label: text("save.delete", undefined, "Delete") });
    if (!hadFocus) return;
    const target = findRow(wanted) ?? focusTarget();
    target?.focus({ preventScroll: true });
  };

  /** The row element of a slot, or null. */
  const findRow = (slot: string | null): HTMLElement | null => {
    if (slot === null) return null;
    return list.querySelector<HTMLElement>(`[data-slot="${slot}"]`);
  };

  const focusTarget = (): HTMLElement | null => {
    const rowsNow = Array.from(list.querySelectorAll<HTMLElement>(".vu-slot"));
    return rowsNow.find((node) => node.getAttribute("aria-disabled") !== "true") ?? null;
  };

  /** Reads one slot. A slot that cannot be read is a damaged row, and is shown as one. */
  const readRow = async (slot: string, isAutosave: boolean, label: string, stored: ReadonlySet<string>): Promise<Row> => {
    const empty: Row = { slot, isAutosave, label, record: null, damaged: false };
    if (!stored.has(slot)) return empty;
    try {
      const loaded = await store.load(slot);
      if (loaded === null) return empty;
      const record = decodeRecord(slot, loaded);
      if (record === null) return { ...empty, damaged: true };
      return { ...empty, record };
    } catch {
      return { ...empty, damaged: true };
    }
  };

  const refresh = async (): Promise<void> => {
    const token = ++refreshCount;
    let stored: string[] = [];
    try {
      stored = await store.list();
    } catch (error) {
      if (!destroyed) report(error, null);
    }
    if (destroyed) return;
    const names = new Set(stored);
    const wanted: Array<{ slot: string; isAutosave: boolean; label: string }> = [];
    if (autosaveName !== null) wanted.push({ slot: autosaveName, isAutosave: true, label: labelOf(true, 0) });
    manualNames.forEach((slot, index) => wanted.push({ slot, isAutosave: false, label: labelOf(false, index) }));
    const read = await Promise.all(wanted.map((item) => readRow(item.slot, item.isAutosave, item.label, names)));
    // A newer refresh has started since this one: its rows win.
    if (destroyed || token !== refreshCount) return;
    rows = read;
    paint();
  };

  /** Picks a row: reads it in load mode, or writes it in save mode after the player agrees. */
  const pick = async (row: Row): Promise<void> => {
    if (busy || destroyed) return;
    busy = true;
    try {
      if (state.mode === "load") await loadRow(row);
      else await saveRow(row);
    } catch (error) {
      if (!destroyed) report(error, row.slot);
    } finally {
      busy = false;
    }
  };

  const loadRow = async (row: Row): Promise<void> => {
    const loaded = await store.load(row.slot);
    if (destroyed) return;
    if (loaded === null) {
      setStatus("save.status.empty", "That slot is empty.");
      await refresh();
      return;
    }
    const record = decodeRecord(row.slot, loaded);
    if (record === null) throw new Error(`slot "${row.slot}" does not hold a save this list can read`);
    options.onLoad?.(record);
    setStatus("save.status.loaded", "Loaded.");
  };

  const saveRow = async (row: Row): Promise<void> => {
    if (collect === undefined) return;
    if (row.record !== null || row.damaged) {
      const agreed = await confirm("overwrite", row);
      if (!agreed || destroyed) return;
    }
    const record = await writeSlot(row.slot, collect);
    if (destroyed) return;
    options.onSaved?.(record);
    setStatus("save.status.saved", "Saved.");
    await refresh();
  };

  /** Asks the game for what to save, and writes it into a slot. */
  const writeSlot = async (
    slot: string,
    gather: (slot: string) => SaveSlotDraft | Promise<SaveSlotDraft>,
  ): Promise<SaveSlotRecord> => {
    const draft = await gather(slot);
    if (draft.state === undefined) throw new TypeError(`the state of slot "${slot}" must not be undefined`);
    const meta = metaOf(draft);
    await store.save(slot, { meta, state: draft.state });
    return { slot, savedAt: new Date().toISOString(), meta, state: draft.state };
  };

  /** Deletes the focused row, after the player agrees. */
  const deleteFocused = async (): Promise<void> => {
    const row = rows.find((candidate) => candidate.slot === focusedSlot);
    if (row === undefined || busy || destroyed) return;
    if (row.record === null && !row.damaged) return;
    busy = true;
    try {
      const agreed = await confirm("delete", row);
      if (!agreed || destroyed) return;
      await store.remove(row.slot);
      if (destroyed) return;
      options.onDeleted?.(row.slot);
      setStatus("save.status.deleted", "Deleted.");
      await refresh();
    } catch (error) {
      if (!destroyed) report(error, row.slot);
    } finally {
      busy = false;
    }
  };

  /** Asks before a write that replaces a save, or before a delete. True when the player says yes. */
  const confirm = async (kind: "overwrite" | "delete", row: Row): Promise<boolean> => {
    const args = { name: row.label };
    const title =
      kind === "overwrite"
        ? text("save.confirm.overwrite.title", args, "Overwrite this save?")
        : text("save.confirm.delete.title", args, "Delete this save?");
    const body =
      kind === "overwrite"
        ? text("save.confirm.overwrite.body", args, `${row.label} already holds a save. It will be replaced.`)
        : text("save.confirm.delete.body", args, `${row.label} will be deleted for good.`);
    modal.update({
      title,
      body,
      actions: [
        { id: "yes", label: text("save.confirm.yes", undefined, "Yes") },
        { id: "no", label: text("save.confirm.no", undefined, "No") },
      ],
    });
    return (await modal.open()) === "yes";
  };

  /** Moves the focus to the next row that can be focused, in `step` direction, wrapping at the ends. */
  const moveFocus = (step: 1 | -1): void => {
    const candidates = Array.from(list.querySelectorAll<HTMLElement>(".vu-slot")).filter(
      (node) => node.getAttribute("aria-disabled") !== "true",
    );
    if (candidates.length === 0) return;
    const current = candidates.findIndex((node) => node === document.activeElement);
    const next = current < 0 ? (step === 1 ? 0 : candidates.length - 1) : (current + step + candidates.length) % candidates.length;
    candidates[next]?.focus({ preventScroll: true });
  };

  /** The keys of the list: the arrows, Home, End and Delete. Enter and Space click the focused row by themselves. */
  const onListKeyDown = (event: KeyboardEvent): void => {
    switch (event.key) {
      case "ArrowDown":
        moveFocus(1);
        break;
      case "ArrowUp":
        moveFocus(-1);
        break;
      case "Home":
        focusTarget()?.focus({ preventScroll: true });
        break;
      case "End":
        Array.from(list.querySelectorAll<HTMLElement>(".vu-slot"))
          .filter((node) => node.getAttribute("aria-disabled") !== "true")
          .at(-1)
          ?.focus({ preventScroll: true });
        break;
      case "Delete":
        void deleteFocused();
        break;
      default:
        return;
    }
    event.preventDefault();
  };

  /** Remembers which row has the focus, so that a repaint can keep it there. */
  const onListFocusIn = (event: FocusEvent): void => {
    const target = event.target;
    if (target instanceof HTMLElement && target.dataset["slot"] !== undefined) {
      focusedSlot = target.dataset["slot"];
    }
  };

  list.addEventListener("keydown", onListKeyDown);
  list.addEventListener("focusin", onListFocusIn);
  void refresh();

  return {
    el,
    update(next: Partial<SaveSlotsProps>): void {
      state = {
        mode: next.mode ?? state.mode,
        t: next.t ?? state.t,
        locale: next.locale === undefined ? state.locale : next.locale,
      };
      void refresh();
    },
    refresh,
    async autosave(): Promise<void> {
      if (autosaveName === null || collect === undefined || destroyed) return;
      await writeSlot(autosaveName, collect);
      if (!destroyed && el.isConnected) await refresh();
    },
    focusTarget(): HTMLElement | null {
      return focusTarget();
    },
    destroy(): void {
      if (destroyed) return;
      destroyed = true;
      list.removeEventListener("keydown", onListKeyDown);
      list.removeEventListener("focusin", onListFocusIn);
      releaseFocus(el);
      deleteButton.destroy();
      modal.destroy();
      el.remove();
    },
  };
}

/**
 * Reads the record of a slot from what the store returned. A save that the list did not write, or whose
 * meta is not in the expected shape, gives null.
 * @param slot - the slot's name.
 * @param loaded - what the store returned.
 */
function decodeRecord(slot: string, loaded: { readonly savedAt: string; readonly data: unknown }): SaveSlotRecord | null {
  const data = loaded.data;
  if (typeof data !== "object" || data === null || !("meta" in data) || !("state" in data)) return null;
  const meta = readMeta(data.meta);
  if (meta === null) return null;
  return { slot, savedAt: loaded.savedAt, meta, state: data.state };
}

/**
 * Reads the meta of a stored record. Null when its shape is wrong.
 * @param value - the stored meta.
 */
function readMeta(value: unknown): SaveSlotMeta | null {
  if (typeof value !== "object" || value === null) return null;
  const title = Reflect.get(value, "title");
  const playTimeMs = Reflect.get(value, "playTimeMs");
  const thumbnail = Reflect.get(value, "thumbnail");
  if (typeof title !== "string" || typeof playTimeMs !== "number" || !Number.isFinite(playTimeMs)) return null;
  return {
    title,
    playTimeMs: Math.max(0, playTimeMs),
    thumbnail: typeof thumbnail === "string" && THUMBNAIL.test(thumbnail) ? thumbnail : undefined,
  };
}

/**
 * The meta that a draft is saved with.
 * @param draft - what the game gave.
 * @throws RangeError when the play time is not a finite number of milliseconds, or the thumbnail is not an image data URL.
 */
function metaOf(draft: SaveSlotDraft): SaveSlotMeta {
  if (!Number.isFinite(draft.playTimeMs) || draft.playTimeMs < 0) {
    throw new RangeError(`playTimeMs must be a number of milliseconds from 0, got ${draft.playTimeMs}`);
  }
  const thumbnail = draft.thumbnail;
  if (thumbnail !== undefined && thumbnail !== null && !THUMBNAIL.test(thumbnail)) {
    throw new RangeError("thumbnail must be a PNG, JPEG or WebP data URL");
  }
  return {
    title: draft.title,
    playTimeMs: Math.round(draft.playTimeMs),
    thumbnail: thumbnail === undefined || thumbnail === null ? undefined : thumbnail,
  };
}

/**
 * Writes a play time as hours, minutes and seconds: "1:05:09". Hours have no upper limit.
 * @param milliseconds - the play time.
 */
export function formatPlayTime(milliseconds: number): string {
  const total = Number.isFinite(milliseconds) && milliseconds > 0 ? Math.floor(milliseconds / 1000) : 0;
  const hours = Math.floor(total / 3600);
  const minutes = Math.floor((total % 3600) / 60);
  const seconds = total % 60;
  return `${hours}:${String(minutes).padStart(2, "0")}:${String(seconds).padStart(2, "0")}`;
}

/**
 * Writes the time of a save for a language: a short date and a short time. An unreadable time gives "".
 * @param iso - the time, as an ISO 8601 string.
 * @param locale - the language tag, or null for the page's language.
 */
export function formatSaveTime(iso: string, locale: string | null): string {
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return "";
  try {
    return new Intl.DateTimeFormat(locale ?? undefined, { dateStyle: "short", timeStyle: "short" }).format(date);
  } catch {
    return date.toISOString();
  }
}

/**
 * True when a value is a thumbnail that the list will show: a PNG, JPEG or WebP data URL.
 * @param value - the value to check.
 */
export function isSafeThumbnail(value: unknown): value is string {
  return typeof value === "string" && THUMBNAIL.test(value);
}
