// The overlay: the panel with its six tabs, and the watch list that stays on screen while the panel is
// hidden. The root is fixed to the window and lets pointer events through; only the panel and its
// contents take them. The overlay writes only inside its own root.

import type { OverlayApi } from "./api";
import { element, button } from "./dom";
import { LogTab } from "./log-tab";
import { ConsoleTab, EventsTab, PerformanceTab, StateTab, TimeTab, type Tab } from "./tabs";
import { formatFixed, messageOf } from "./text";
import type { HudCorner } from "./types";

/** The corners where the pinned watches can be drawn, and the class that places the HUD in each. */
export const HUD_CORNERS: readonly HudCorner[] = ["top-left", "top-right", "bottom-left", "bottom-right"];

/** The corner that the pinned watches start in. */
export const DEFAULT_HUD_CORNER: HudCorner = "top-left";

/** True when `value` is one of the four HUD corners. */
export function isHudCorner(value: unknown): value is HudCorner {
  return typeof value === "string" && (HUD_CORNERS as readonly string[]).includes(value);
}

/** The tabs in the order they show. */
const TABS = ["performance", "time", "events", "log", "state", "console"] as const;

/** One of the tab names. */
type TabName = (typeof TABS)[number];

/** The label of each tab. */
const TAB_LABELS: Readonly<Record<TabName, string>> = {
  performance: "Performance",
  time: "Time",
  events: "Events",
  log: "Log",
  state: "State",
  console: "Console",
};

/** The separator between watch names in the key that tells when the list changed. */
const KEY_SEPARATOR = "\u001f";

/** Formats a pinned number: whole numbers as they are, others with a few decimals. */
function formatWatch(value: number): string {
  if (Number.isInteger(value)) return String(value);
  return formatFixed(value, Math.abs(value) >= 10 ? 1 : 3);
}

/** Reads a watch. A getter that throws shows "error" instead of breaking the list. */
function readWatch(read: () => number): string {
  try {
    const value = read();
    return Number.isFinite(value) ? formatWatch(value) : String(value);
  } catch (error) {
    return `error: ${messageOf(error)}`;
  }
}

/** The overlay: a root with the watch list and the panel. */
export class Overlay {
  /** The fixed element that holds everything the overlay shows. */
  readonly root: HTMLElement;
  private readonly doc: Document;
  private readonly api: OverlayApi;
  private readonly hud: HTMLElement;
  private readonly panel: HTMLElement;
  private readonly tabButtons = new Map<TabName, HTMLButtonElement>();
  private readonly tabs: Record<TabName, Tab>;
  private active: TabName = "performance";
  private shown = false;
  private hudKey = "";
  private readonly hudValues = new Map<string, HTMLElement>();

  /**
   * @param doc - the document the overlay is built in.
   * @param api - what the overlay reads from the devtools.
   * @param host - the element that the root is added to.
   * @param toggleLabel - the key name shown in the panel's header.
   */
  constructor(doc: Document, api: OverlayApi, host: HTMLElement, toggleLabel: string) {
    this.doc = doc;
    this.api = api;
    this.root = element(doc, "div", "vdt-root");
    this.hud = element(doc, "div", "vdt-hud");
    this.hud.hidden = true;
    this.setHudCorner(DEFAULT_HUD_CORNER);

    this.panel = element(doc, "section", "vdt-panel");
    this.panel.hidden = true;
    this.panel.setAttribute("aria-label", "Velvet devtools");

    const head = element(doc, "div", "vdt-head");
    head.append(
      element(doc, "span", "vdt-title", "Devtools"),
      element(doc, "span", "vdt-hint", `${toggleLabel} shows or hides`),
      button(doc, "Close", () => {
        api.hide();
      }),
    );

    this.tabs = {
      performance: new PerformanceTab(doc, api),
      time: new TimeTab(doc, api),
      events: new EventsTab(doc, api),
      log: new LogTab(doc, api),
      state: new StateTab(doc, api),
      console: new ConsoleTab(doc, api),
    };
    const tabBar = element(doc, "div", "vdt-tabs");
    tabBar.setAttribute("role", "tablist");
    const pages = element(doc, "div", "vdt-pages");
    for (const name of TABS) {
      const tabButton = element(doc, "button", "vdt-tab", TAB_LABELS[name]);
      tabButton.type = "button";
      tabButton.setAttribute("role", "tab");
      tabButton.addEventListener("click", () => {
        this.select(name);
      });
      this.tabButtons.set(name, tabButton);
      tabBar.append(tabButton);

      const page = this.tabs[name].element;
      page.setAttribute("role", "tabpanel");
      pages.append(page);
    }

    this.panel.append(head, tabBar, pages);
    this.root.append(this.hud, this.panel);
    host.append(this.root);
    this.select(this.active);
  }

  /** True while the panel is shown. */
  get visible(): boolean {
    return this.shown;
  }

  /**
   * Shows or hides the panel. Hiding moves focus out of the panel, so that the keys go back to the page.
   */
  setVisible(on: boolean): void {
    if (on === this.shown) return;
    this.shown = on;
    this.panel.hidden = !on;
    if (on) {
      this.tabs[this.active].update();
      return;
    }
    const focused = this.doc.activeElement;
    if (focused instanceof HTMLElement && this.root.contains(focused)) focused.blur();
  }

  /**
   * Moves the pinned watches to a corner: the HUD keeps one corner class, and the stylesheet places it.
   * The caller checks the corner first; see {@link isHudCorner}.
   */
  setHudCorner(corner: HudCorner): void {
    for (const name of HUD_CORNERS) this.hud.classList.remove(`vdt-corner-${name}`);
    this.hud.classList.add(`vdt-corner-${corner}`);
  }

  /** Refreshes the watch list, and the visible tab. Called on the devtools' tick. */
  update(): void {
    this.updateWatches();
    if (this.shown) this.tabs[this.active].update();
  }

  /** Removes the overlay from the page. */
  destroy(): void {
    this.root.remove();
  }

  /** Makes one tab the visible one. */
  private select(name: TabName): void {
    this.active = name;
    for (const tab of TABS) {
      const selected = tab === name;
      this.tabButtons.get(tab)?.setAttribute("aria-selected", String(selected));
      this.tabs[tab].element.hidden = !selected;
    }
    if (this.shown) this.tabs[name].update();
  }

  /**
   * Builds the watch rows when the set of names changes, and refreshes their values. While the HUD is off,
   * or no watch is pinned, the HUD is hidden and no watch is read.
   */
  private updateWatches(): void {
    const watches = this.api.watches();
    const shown = this.api.hudEnabled() && watches.size > 0;
    this.hud.hidden = !shown;
    if (!shown) return;
    const key = [...watches.keys()].join(KEY_SEPARATOR);
    if (key !== this.hudKey) {
      this.hudKey = key;
      this.hudValues.clear();
      const rows = [...watches.keys()].map((name) => {
        const value = element(this.doc, "span", "vdt-hud-value");
        this.hudValues.set(name, value);
        const row = element(this.doc, "div", "vdt-hud-row");
        row.append(element(this.doc, "span", "vdt-hud-name", name), value);
        return row;
      });
      this.hud.replaceChildren(...rows);
    }
    for (const [name, read] of watches) {
      const value = this.hudValues.get(name);
      if (value !== undefined) value.textContent = readWatch(read);
    }
  }
}
