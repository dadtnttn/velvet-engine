// The five tabs of the overlay panel: Performance, Time, Events, State and Console. Each builds its DOM
// once, and refreshes it from the devtools through OverlayApi. The overlay calls update() only while the
// tab is the one on show, so a hidden tab costs nothing.

import type { InspectView, OverlayApi } from "./api";
import { button, element } from "./dom";
import { FRAME_WINDOW, LONG_FRAME_30_MS, LONG_FRAME_60_MS } from "./stats";
import { PAGE_SIZE, buildTree, type TreeRow } from "./inspect";
import { copyJson, formatFixed, messageOf } from "./text";
import type { ConsoleKind } from "./console";

/** A tab of the panel: its page, and the refresh that the overlay calls while the tab is shown. */
export interface Tab {
  readonly element: HTMLElement;
  update(): void;
}

/** The namespace of SVG elements. */
const SVG_NS = "http://www.w3.org/2000/svg";

/** The graph's drawing size, in its own units. The SVG stretches to the panel's width. */
const GRAPH_WIDTH = 240;
const GRAPH_HEIGHT = 96;

/** The frame time at the top of the graph, in milliseconds. Taller frames are clipped to the top. */
const GRAPH_MAX_MS = 50;

/** Indent per nesting level in the State tree, in pixels. */
const INDENT_PX = 14;

/** The labels of the Performance tab, in the order they show. */
const PERF_LABELS = [
  "FPS",
  "Frame p50",
  "Frame p95",
  "Frame max",
  "Last frame",
  "Frames over 16.7 ms",
  "Frames over 33 ms",
  "Update per frame",
  "Render per frame",
  "Loop",
  "Stage",
  "DOM nodes",
  "JS heap",
] as const;

/** Milliseconds with one decimal, or a dash. */
function formatMs(value: number | null): string {
  return value === null ? "–" : `${formatFixed(value, 1)} ms`;
}

/** Seconds from milliseconds, with two decimals. */
function formatSeconds(ms: number): string {
  return `${formatFixed(ms / 1000, 2)} s`;
}

/** Megabytes from bytes, with one decimal. */
function formatHeap(bytes: number | null): string {
  return bytes === null ? "not reported" : `${formatFixed(bytes / (1024 * 1024), 1)} MB`;
}

/** The Performance tab: frame numbers, the graph of recent frames, and the memory numbers. */
export class PerformanceTab implements Tab {
  readonly element: HTMLElement;
  private readonly api: OverlayApi;
  private readonly values = new Map<string, HTMLElement>();
  private readonly graphLine: SVGPolylineElement;
  private readonly note: HTMLElement;

  constructor(doc: Document, api: OverlayApi) {
    this.api = api;
    this.element = element(doc, "div", "vdt-page");

    const table = element(doc, "dl", "vdt-kv");
    for (const label of PERF_LABELS) {
      const value = element(doc, "dd");
      table.append(element(doc, "dt", undefined, label), value);
      this.values.set(label, value);
    }

    const graph = doc.createElementNS(SVG_NS, "svg");
    graph.setAttribute("class", "vdt-graph");
    graph.setAttribute("viewBox", `0 0 ${GRAPH_WIDTH} ${GRAPH_HEIGHT}`);
    graph.setAttribute("preserveAspectRatio", "none");
    graph.setAttribute("role", "img");
    graph.setAttribute("aria-label", "Frame times, newest on the right");
    for (const [ms, warn] of [
      [LONG_FRAME_60_MS, "is-warn"],
      [LONG_FRAME_30_MS, "is-bad"],
    ] as const) {
      const y = (GRAPH_HEIGHT - (ms / GRAPH_MAX_MS) * GRAPH_HEIGHT).toFixed(1);
      const line = doc.createElementNS(SVG_NS, "line");
      line.setAttribute("class", `vdt-graph-ref ${warn}`);
      line.setAttribute("x1", "0");
      line.setAttribute("x2", String(GRAPH_WIDTH));
      line.setAttribute("y1", y);
      line.setAttribute("y2", y);
      graph.append(line);
    }
    this.graphLine = doc.createElementNS(SVG_NS, "polyline");
    this.graphLine.setAttribute("class", "vdt-graph-line");
    graph.append(this.graphLine);

    this.note = element(doc, "p", "vdt-note");
    const reset = button(doc, "Reset counters", () => {
      this.api.resetCounters();
    });
    this.element.append(table, graph, this.note, reset);
  }

  /** Refreshes the numbers and the graph from the devtools. */
  update(): void {
    const stats = this.api.stats();
    const loop = this.api.loopState();
    const set = (label: string, text: string): void => {
      const cell = this.values.get(label);
      if (cell !== undefined) cell.textContent = text;
    };
    set("FPS", formatFixed(stats.fps, 1));
    set("Frame p50", formatMs(stats.p50Ms));
    set("Frame p95", formatMs(stats.p95Ms));
    set("Frame max", formatMs(stats.maxMs));
    set("Last frame", formatMs(stats.lastMs));
    set("Frames over 16.7 ms", String(stats.longFrames16));
    set("Frames over 33 ms", String(stats.longFrames33));
    set("Update per frame", loop.wrapped ? formatMs(stats.updateMs) : "not measured");
    set("Render per frame", loop.wrapped ? formatMs(stats.renderMs) : "not measured");
    set("Loop", loop.running === null ? "no loop given" : loop.running ? "running" : "stopped");
    set("Stage", this.api.stageText());
    set("DOM nodes", String(stats.domNodes));
    set("JS heap", formatHeap(stats.heapBytes));
    this.note.textContent = loop.wrapped
      ? ""
      : "Update and render times need the loop options to go through devtools.wrapLoop().";

    this.graphLine.setAttribute("points", this.points(this.api.frameHistory()));
  }

  /** The SVG points of the recent frames. The newest frame is on the right edge. */
  private points(history: readonly number[]): string {
    const offset = FRAME_WINDOW - history.length;
    const step = GRAPH_WIDTH / (FRAME_WINDOW - 1);
    return history
      .map((ms, index) => {
        const x = (offset + index) * step;
        const y = GRAPH_HEIGHT - Math.min(ms / GRAPH_MAX_MS, 1) * GRAPH_HEIGHT;
        return `${x.toFixed(1)},${y.toFixed(1)}`;
      })
      .join(" ");
  }
}

/**
 * The Time tab: pause or resume, step, the speeds that the clock allows, and slow motion. Errors from an action
 * (for example a speed the clock does not allow) stay on the status line until the next action.
 */
export class TimeTab implements Tab {
  readonly element: HTMLElement;
  private readonly api: OverlayApi;
  private readonly status: HTMLElement;
  private readonly pauseButton: HTMLButtonElement;
  private readonly stepButton: HTMLButtonElement;
  private readonly slowButton: HTMLButtonElement;
  private readonly speedButtons = new Map<number, HTMLButtonElement>();
  private lastError: string | null = null;

  constructor(doc: Document, api: OverlayApi) {
    this.api = api;
    this.element = element(doc, "div", "vdt-page");
    this.status = element(doc, "p", "vdt-status");

    this.pauseButton = button(doc, "Pause", () => {
      this.attempt(() => (api.timeState().paused ? api.resume() : api.pause()));
    });
    this.stepButton = button(doc, "Step one frame", () => {
      this.attempt(() => api.step());
    });
    const controls = element(doc, "div", "vdt-buttons");
    controls.append(this.pauseButton, this.stepButton);

    const speeds = element(doc, "div", "vdt-buttons");
    for (const speed of api.timeState().speeds) {
      const speedButton = button(doc, `×${speed}`, () => {
        this.attempt(() => api.setSpeed(speed));
      });
      this.speedButtons.set(speed, speedButton);
      speeds.append(speedButton);
    }

    this.slowButton = button(doc, "Slow motion", () => {
      this.attempt(() => api.setSlowMotion(!api.timeState().slowMotion));
    });
    const slow = element(doc, "div", "vdt-buttons");
    slow.append(this.slowButton);

    const note = element(
      doc,
      "p",
      "vdt-note",
      "The speeds are the clock's own. For ×0.25 to ×4, create the clock with speeds [0.25, 0.5, 1, 2, 4].",
    );
    this.element.append(this.status, controls, speeds, slow, note);
  }

  /** Refreshes the buttons and the status line. */
  update(): void {
    const state = this.api.timeState();
    const loop = this.api.loopState();
    this.pauseButton.textContent = state.paused ? "Resume" : "Pause";
    this.stepButton.disabled = !state.paused;
    for (const [speed, speedButton] of this.speedButtons) {
      speedButton.disabled = !state.hasClock;
      speedButton.setAttribute("aria-pressed", String(!state.slowMotion && speed === state.speed));
    }
    this.slowButton.disabled = !state.hasClock || !state.slowAllowed;
    this.slowButton.setAttribute("aria-pressed", String(state.slowMotion));
    this.slowButton.textContent = `Slow motion ×${state.slowSpeed}`;

    if (this.lastError !== null) {
      this.setStatus(this.lastError, true);
      return;
    }
    if (!state.hasClock) {
      this.setStatus("No game clock was given. Pass clock to createDevtools to control game time.", false);
      return;
    }
    const parts = [
      state.paused ? "Paused" : "Running",
      `×${state.speed ?? 1}`,
      state.slowMotion ? "slow motion" : "",
      loop.wrapped ? "" : "the game update is not held: wrap the loop with devtools.wrapLoop()",
    ].filter((part) => part !== "");
    this.setStatus(parts.join(" · "), false);
  }

  /** Runs an action. A thrown error is kept on the status line until the next action. */
  private attempt(action: () => void): void {
    try {
      action();
      this.lastError = null;
    } catch (error) {
      this.lastError = messageOf(error);
    }
    this.update();
  }

  /** Sets the status text, in the error colour when `isError` is true. */
  private setStatus(text: string, isError: boolean): void {
    this.status.textContent = text;
    this.status.classList.toggle("is-error", isError);
  }
}

/** The Events tab: the logged events, newest first, with a name filter, a pause and a clear. */
export class EventsTab implements Tab {
  readonly element: HTMLElement;
  private readonly api: OverlayApi;
  private readonly doc: Document;
  private readonly filter: HTMLInputElement;
  private readonly pauseButton: HTMLButtonElement;
  private readonly summary: HTMLElement;
  private readonly list: HTMLElement;
  private shownRevision = -1;
  private shownFilter = "";

  constructor(doc: Document, api: OverlayApi) {
    this.api = api;
    this.doc = doc;
    this.element = element(doc, "div", "vdt-page");

    this.filter = element(doc, "input", "vdt-input");
    this.filter.type = "search";
    this.filter.placeholder = "Filter by event name";
    this.filter.setAttribute("aria-label", "Filter by event name");
    this.filter.addEventListener("input", () => {
      this.shownRevision = -1;
      this.update();
    });

    this.pauseButton = button(doc, "Pause log", () => {
      const log = api.events();
      log.setPaused(!log.paused);
      this.shownRevision = -1;
      this.update();
    });
    const clear = button(doc, "Clear", () => {
      api.events().clear();
      this.shownRevision = -1;
      this.update();
    });
    const toolbar = element(doc, "div", "vdt-buttons");
    toolbar.append(this.pauseButton, clear);

    this.summary = element(doc, "p", "vdt-status");
    this.list = element(doc, "ol", "vdt-list");
    this.element.append(this.filter, toolbar, this.summary, this.list);
  }

  /** Redraws the list when the log or the filter changed. */
  update(): void {
    const log = this.api.events();
    if (log.revision === this.shownRevision && this.filter.value === this.shownFilter) return;
    this.shownRevision = log.revision;
    this.shownFilter = this.filter.value;

    this.pauseButton.textContent = log.paused ? "Resume log" : "Pause log";
    this.pauseButton.setAttribute("aria-pressed", String(log.paused));
    const entries = log.list(this.filter.value);
    const notes = [
      `${entries.length} of ${log.size} shown`,
      log.dropped > 0 ? `${log.dropped} dropped while paused` : "",
      log.paused ? "paused" : "",
    ].filter((note) => note !== "");
    this.summary.textContent = notes.join(" · ");

    const doc = this.doc;
    const items = entries.map((entry) => {
      const item = element(doc, "li", "vdt-event");
      item.append(element(doc, "span", "vdt-event-time", formatSeconds(entry.atMs)));
      const body = element(doc, "span");
      body.append(
        element(doc, "span", "vdt-event-name", entry.name),
        " ",
        element(doc, "span", "vdt-event-summary", entry.summary),
      );
      item.append(body);
      return item;
    });
    this.list.replaceChildren(...items);
  }
}

/** One State view on the page: its section, its open rows, and what it last read. */
interface ViewState {
  readonly name: string;
  read: () => unknown;
  refreshMs: number;
  readonly section: HTMLElement;
  readonly tree: HTMLElement;
  readonly status: HTMLElement;
  readonly autoButton: HTMLButtonElement;
  readonly expanded: Set<string>;
  readonly pages: Map<string, number>;
  value: unknown;
  hasValue: boolean;
  rows: TreeRow[];
  autoOn: boolean;
  lastReadMs: number;
}

/**
 * The State tab: one section per view registered with `inspect`. Each has a tree that opens row by row, and
 * buttons to refresh, to refresh every so often, and to copy the value as JSON.
 */
export class StateTab implements Tab {
  readonly element: HTMLElement;
  private readonly api: OverlayApi;
  private readonly doc: Document;
  private readonly empty: HTMLElement;
  private readonly sections = new Map<string, ViewState>();

  constructor(doc: Document, api: OverlayApi) {
    this.api = api;
    this.doc = doc;
    this.element = element(doc, "div", "vdt-page");
    this.empty = element(
      doc,
      "p",
      "vdt-note",
      "No views yet. Call devtools.inspect(name, read) to show a value here.",
    );
    this.element.append(this.empty);
  }

  /** Adds, updates and removes sections to match the views, and refreshes the ones that are on auto. */
  update(): void {
    const views = this.api.views();
    const now = this.api.now();
    for (const name of [...this.sections.keys()]) {
      if (views.has(name)) continue;
      this.sections.get(name)?.section.remove();
      this.sections.delete(name);
    }
    for (const view of views.values()) {
      let state = this.sections.get(view.name);
      if (state === undefined) {
        state = this.createSection(view);
        this.sections.set(view.name, state);
        this.element.append(state.section);
        this.refresh(state);
      }
      state.read = view.read;
      state.refreshMs = view.refreshMs;
      if (state.autoOn && now - state.lastReadMs >= state.refreshMs) this.refresh(state);
    }
    this.empty.hidden = views.size > 0;
  }

  /** Builds the section of a new view. */
  private createSection(view: InspectView): ViewState {
    const doc = this.doc;
    const section = element(doc, "section", "vdt-view");
    const head = element(doc, "div", "vdt-view-head");
    const tree = element(doc, "ul", "vdt-tree");
    tree.setAttribute("aria-label", `${view.name} state`);
    const status = element(doc, "p", "vdt-status");

    const state: ViewState = {
      name: view.name,
      read: view.read,
      refreshMs: view.refreshMs,
      section,
      tree,
      status,
      autoButton: button(doc, "Auto", () => {
        state.autoOn = !state.autoOn;
        state.autoButton.setAttribute("aria-pressed", String(state.autoOn));
        if (state.autoOn) this.refresh(state);
      }),
      expanded: new Set([""]),
      pages: new Map(),
      value: undefined,
      hasValue: false,
      rows: [],
      autoOn: false,
      lastReadMs: Number.NEGATIVE_INFINITY,
    };
    state.autoButton.setAttribute("aria-pressed", "false");
    head.append(
      element(doc, "span", "vdt-view-name", view.name),
      button(doc, "Refresh", () => {
        this.refresh(state);
      }),
      state.autoButton,
      button(doc, "Copy JSON", () => {
        this.copy(state);
      }),
    );
    tree.addEventListener("click", (event) => {
      this.onTreeClick(state, event);
    });
    section.append(head, tree, status);
    return state;
  }

  /** Reads the view again. A read that throws keeps the last value and shows the error. */
  private refresh(state: ViewState): void {
    state.lastReadMs = this.api.now();
    try {
      state.value = state.read();
      state.hasValue = true;
    } catch (error) {
      this.setStatus(state, `Read failed: ${messageOf(error)}`, true);
      return;
    }
    this.setStatus(state, "", false);
    this.paint(state);
  }

  /** Builds the tree rows again from the value that was read. Closed rows read nothing more. */
  private paint(state: ViewState): void {
    if (!state.hasValue) {
      state.tree.replaceChildren();
      return;
    }
    state.rows = buildTree(state.value, { expanded: state.expanded, pages: state.pages });
    state.tree.replaceChildren(...state.rows.map((row, index) => this.rowItem(row, index)));
  }

  /** The element of one tree row. Its buttons carry the index of the row, which the click handler reads. */
  private rowItem(row: TreeRow, index: number): HTMLElement {
    const doc = this.doc;
    const item = element(doc, "li", `vdt-row vdt-kind-${row.kind}`);
    item.style.paddingLeft = `${row.depth * INDENT_PX}px`;

    if (row.kind === "more") {
      const total = row.total ?? 0;
      const more = element(
        doc,
        "button",
        "vdt-button vdt-more",
        `Show ${Math.min(PAGE_SIZE, total - (row.shown ?? 0))} more of ${total}`,
      );
      more.type = "button";
      more.dataset["action"] = "more";
      more.dataset["index"] = String(index);
      item.append(more);
      return item;
    }

    if (row.expandable) {
      const caret = element(doc, "button", "vdt-caret", row.expanded ? "▾" : "▸");
      caret.type = "button";
      caret.setAttribute("aria-expanded", String(row.expanded));
      caret.dataset["action"] = "toggle";
      caret.dataset["index"] = String(index);
      item.append(caret);
    } else {
      item.append(element(doc, "span", "vdt-caret-gap"));
    }
    item.append(element(doc, "span", "vdt-label", row.label));
    item.append(element(doc, "span", "vdt-preview", row.preview));
    return item;
  }

  /** Opens or closes a row, or shows the next page of a long list. The tree is then built again. */
  private onTreeClick(state: ViewState, event: Event): void {
    const target = event.target;
    if (!(target instanceof Element)) return;
    const control = target.closest("button[data-action]");
    if (!(control instanceof HTMLElement)) return;
    const row = state.rows[Number(control.dataset["index"])];
    if (row === undefined) return;
    if (control.dataset["action"] === "toggle") {
      if (state.expanded.has(row.id)) state.expanded.delete(row.id);
      else state.expanded.add(row.id);
    } else if (control.dataset["action"] === "more") {
      const parent = row.parentId ?? "";
      state.pages.set(parent, (state.pages.get(parent) ?? PAGE_SIZE) + PAGE_SIZE);
    }
    this.paint(state);
  }

  /** Copies the last value as JSON to the clipboard, and says whether that worked. */
  private copy(state: ViewState): void {
    if (!state.hasValue) {
      this.setStatus(state, "Nothing to copy yet. Refresh first.", true);
      return;
    }
    const text = copyJson(state.value);
    const clipboard = typeof navigator === "undefined" ? undefined : navigator.clipboard;
    if (clipboard === undefined || typeof clipboard.writeText !== "function") {
      this.setStatus(state, "The clipboard is not available here.", true);
      return;
    }
    clipboard.writeText(text).then(
      () => {
        this.setStatus(state, `Copied ${text.length} characters of JSON.`, false);
      },
      (error: unknown) => {
        this.setStatus(state, `Copy failed: ${messageOf(error)}`, true);
      },
    );
  }

  /** Sets the status line of a section. */
  private setStatus(state: ViewState, text: string, isError: boolean): void {
    state.status.textContent = text;
    state.status.classList.toggle("is-error", isError);
  }
}

/** The Console tab: the output, and a prompt with history, completion and Escape to leave it. */
export class ConsoleTab implements Tab {
  readonly element: HTMLElement;
  private readonly api: OverlayApi;
  private readonly log: HTMLElement;
  private readonly input: HTMLInputElement;
  private shownRevision = -1;
  private readonly doc: Document;

  constructor(doc: Document, api: OverlayApi) {
    this.api = api;
    this.doc = doc;
    this.element = element(doc, "div", "vdt-page");

    this.log = element(doc, "div", "vdt-log");
    this.log.setAttribute("role", "log");
    this.log.setAttribute("aria-live", "polite");

    this.input = element(doc, "input", "vdt-input");
    this.input.type = "text";
    this.input.autocomplete = "off";
    this.input.spellcheck = false;
    this.input.placeholder = "Type help and press Enter";
    this.input.setAttribute("aria-label", "Console command");
    this.input.addEventListener("keydown", (event) => {
      this.onKey(event);
    });
    this.input.addEventListener("input", () => {
      this.api.historyReset();
    });

    const hint = element(
      doc,
      "p",
      "vdt-prompt-hint",
      "Enter runs · Tab completes · Up and Down browse history · Escape leaves the prompt",
    );
    this.element.append(this.log, this.input, hint);
  }

  /** Writes the output lines again when they changed. */
  update(): void {
    const output = this.api.output();
    if (output.revision === this.shownRevision) return;
    this.shownRevision = output.revision;
    const lines = output.lines.map((line) => element(this.doc, "div", lineClass(line.kind), line.text));
    this.log.replaceChildren(...lines);
    this.log.scrollTop = this.log.scrollHeight;
  }

  /** Handles the keys of the prompt. None of them reaches the page, and no key reaches the game. */
  private onKey(event: KeyboardEvent): void {
    if (event.isComposing) return;
    switch (event.key) {
      case "Enter": {
        event.preventDefault();
        const line = this.input.value;
        this.input.value = "";
        this.api.runLine(line);
        this.update();
        return;
      }
      case "ArrowUp": {
        event.preventDefault();
        const previous = this.api.historyPrevious(this.input.value);
        if (previous !== null) this.input.value = previous;
        return;
      }
      case "ArrowDown": {
        event.preventDefault();
        const next = this.api.historyNext();
        if (next !== null) this.input.value = next;
        return;
      }
      case "Tab": {
        event.preventDefault();
        const completion = this.api.complete(this.input.value);
        this.input.value = completion.text;
        if (completion.options.length > 1) this.api.print("output", completion.options.join("   "));
        return;
      }
      case "Escape":
        this.input.blur();
        return;
      default:
        return;
    }
  }
}

/** The class name of an output line of a kind. */
function lineClass(kind: ConsoleKind): string {
  return `vdt-line-${kind}`;
}
