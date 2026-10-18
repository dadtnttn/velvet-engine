// @vitest-environment happy-dom
// Tests for the devtools on a page: the panel and its tabs, the F3 toggle, the keys and clicks that never reach
// the game's ActionMap, the time controls on a real clock, the wrapped loop, the console and its commands, the
// watches, the State views and the event log. The frames come from a scheduler that the test drives.

import {
  ActionMap,
  EventEmitter,
  GameClock,
  GameLoop,
  type Binding,
  type FrameScheduler,
} from "@velvet/core";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { createDevtools } from "./devtools";
import { EventLog } from "./events";
import { STYLE_ID } from "./styles";
import type { Devtools, DevtoolsOptions, HudCorner } from "./types";

/** A frame scheduler that the test drives. `tick` runs the frames that are waiting. */
interface ManualScheduler extends FrameScheduler {
  tick(timeMs: number): void;
  waiting(): number;
}

/** Makes a scheduler whose frames run only when the test ticks it. */
function manualScheduler(): ManualScheduler {
  let next = 1;
  let waiting = new Map<number, (timeMs: number) => void>();
  return {
    request(callback) {
      const handle = next;
      next += 1;
      waiting.set(handle, callback);
      return handle;
    },
    cancel(handle) {
      waiting.delete(handle);
    },
    tick(timeMs) {
      const frames = [...waiting.values()];
      waiting = new Map();
      for (const frame of frames) frame(timeMs);
    },
    waiting() {
      return waiting.size;
    },
  };
}

/** A keyboard event that bubbles and can be cancelled, as a browser sends it. */
function key(name: string, options: { code?: string; repeat?: boolean } = {}): KeyboardEvent {
  return new KeyboardEvent("keydown", {
    key: name,
    code: options.code ?? name,
    repeat: options.repeat ?? false,
    bubbles: true,
    cancelable: true,
  });
}

/** A pointer press with a button, as the ActionMap reads it. */
function pointerPress(): Event {
  return Object.assign(new Event("pointerdown", { bubbles: true, cancelable: true }), {
    button: 0,
    clientX: 0,
    clientY: 0,
  });
}

/** A wheel turn downwards, as the ActionMap reads it. */
function wheelDown(): Event {
  return Object.assign(new Event("wheel", { bubbles: true, cancelable: true }), {
    deltaX: 0,
    deltaY: 120,
    deltaMode: 0,
  });
}

/** The first element that matches, or an error that names the selector. */
function query<Element extends HTMLElement>(selector: string): Element {
  const found = document.querySelector<Element>(selector);
  if (found === null) throw new Error(`no element matches ${selector}`);
  return found;
}

/** The tab button with a label. */
function tabButton(label: string): HTMLButtonElement {
  const found = [...document.querySelectorAll<HTMLButtonElement>('[role="tab"]')].find(
    (button) => button.textContent === label,
  );
  if (found === undefined) throw new Error(`no tab ${label}`);
  return found;
}

/** The page of the tab with a label. The pages are in the same order as the tabs. */
function tabPage(label: string): HTMLElement {
  const index = [...document.querySelectorAll('[role="tab"]')].findIndex((button) => button.textContent === label);
  const page = document.querySelectorAll<HTMLElement>('[role="tabpanel"]')[index];
  if (page === undefined) throw new Error(`no page for ${label}`);
  return page;
}

/** The console's prompt. */
function prompt(): HTMLInputElement {
  return query<HTMLInputElement>('input[aria-label="Console command"]');
}

/** Types a line into the prompt and presses Enter. */
function runCommand(line: string): void {
  prompt().value = line;
  prompt().dispatchEvent(key("Enter"));
}

/** The text of everything the console printed. */
function consoleText(): string {
  return query(".vdt-log").textContent ?? "";
}

/** A button in the State section of a view, by its label. */
function viewButton(label: string): HTMLButtonElement {
  const found = [...document.querySelectorAll<HTMLButtonElement>(".vdt-view button")].find(
    (button) => button.textContent === label,
  );
  if (found === undefined) throw new Error(`no view button ${label}`);
  return found;
}

let host: HTMLElement;
let scheduler: ManualScheduler;

beforeEach(() => {
  host = document.createElement("div");
  host.id = "game";
  document.body.append(host);
  scheduler = manualScheduler();
});

afterEach(() => {
  vi.restoreAllMocks();
  document.body.innerHTML = "";
  // The stylesheet lives in the head, so it is removed here to keep each test starting from a clean page.
  document.getElementById(STYLE_ID)?.remove();
});

/** Creates enabled devtools on the test host, driven by the test's scheduler. */
function devtoolsWith<Events extends object = object>(options: Partial<DevtoolsOptions<Events>> = {}): Devtools {
  return createDevtools<Events>({ host, enabled: true, scheduler, ...options });
}

describe("the panel", () => {
  it("starts hidden, with the six tabs in order", () => {
    const devtools = devtoolsWith();
    expect(devtools.enabled).toBe(true);
    expect(devtools.visible).toBe(false);
    expect(query(".vdt-panel").hidden).toBe(true);
    const labels = [...document.querySelectorAll('[role="tab"]')].map((button) => button.textContent);
    expect(labels).toEqual(["Performance", "Time", "Events", "Log", "State", "Console"]);
  });

  it("is added to the host after the host's own children, and the host is otherwise untouched", () => {
    const own = document.createElement("p");
    host.append(own);
    const before = host.getAttribute("style");
    devtoolsWith();
    expect(host.firstElementChild).toBe(own);
    expect(host.lastElementChild?.classList.contains("vdt-root")).toBe(true);
    expect(host.getAttribute("style")).toBe(before);
  });

  it("F3 shows and hides the panel, ignores repeats, and cancels the browser's own F3", () => {
    const devtools = devtoolsWith();
    const first = key("F3");
    document.body.dispatchEvent(first);
    expect(devtools.visible).toBe(true);
    expect(first.defaultPrevented).toBe(true);

    document.body.dispatchEvent(key("F3", { repeat: true }));
    expect(devtools.visible).toBe(true);

    document.body.dispatchEvent(key("F3"));
    expect(devtools.visible).toBe(false);
  });

  it("uses the toggle key it was given, and ignores F3 then", () => {
    const devtools = devtoolsWith({ toggleKey: "KeyP" });
    document.body.dispatchEvent(key("F3"));
    expect(devtools.visible).toBe(false);
    document.body.dispatchEvent(key("p", { code: "KeyP" }));
    expect(devtools.visible).toBe(true);
  });

  it("shows only the page of the selected tab", () => {
    devtoolsWith().show();
    expect(tabPage("Performance").hidden).toBe(false);
    tabButton("Time").click();
    expect(tabPage("Time").hidden).toBe(false);
    expect(tabPage("Performance").hidden).toBe(true);
    expect(tabButton("Time").getAttribute("aria-selected")).toBe("true");
  });

  it("writes its stylesheet once, however many tools are made", () => {
    devtoolsWith();
    devtoolsWith();
    expect(document.querySelectorAll(`style#${STYLE_ID}`)).toHaveLength(1);
  });
});

describe("keys and clicks stay with the tools", () => {
  it("does not count typing in the console as a game key, even for a map that captures controls", () => {
    devtoolsWith().show();
    tabButton("Console").click();
    const map = new ActionMap<"move">([{ action: "move", control: { type: "key", key: "KeyW" } }], {
      captureControls: true,
    });
    map.attach(window);

    const typed = key("w", { code: "KeyW" });
    prompt().dispatchEvent(typed);
    expect(map.held("move")).toBe(false);
    expect(typed.defaultPrevented).toBe(false);

    document.body.dispatchEvent(key("w", { code: "KeyW" }));
    expect(map.held("move")).toBe(true);
    map.detach();
  });

  it("hiding the panel takes the focus out of it, so the game's keys work again", () => {
    const devtools = devtoolsWith();
    devtools.show();
    tabButton("Console").click();
    prompt().focus();
    expect(document.activeElement).toBe(prompt());

    const map = new ActionMap<"move">([{ action: "move", control: { type: "key", key: "KeyW" } }]);
    map.attach(window);
    document.body.dispatchEvent(key("F3"));
    expect(devtools.visible).toBe(false);
    expect(document.activeElement).not.toBe(prompt());

    document.body.dispatchEvent(key("w", { code: "KeyW" }));
    expect(map.held("move")).toBe(true);
    map.detach();
  });

  it("does not count a press on the panel as a game press, but counts one on the page", () => {
    devtoolsWith().show();
    const bindings: Binding<"attack">[] = [{ action: "attack", control: { type: "mouse", button: "left" } }];
    const map = new ActionMap<"attack">(bindings);
    map.attach(window);

    tabPage("Performance").dispatchEvent(pointerPress());
    expect(map.held("attack")).toBe(false);

    host.dispatchEvent(pointerPress());
    expect(map.held("attack")).toBe(true);
    map.detach();
  });

  it("does not count a wheel turn on the panel as a game turn", () => {
    devtoolsWith().show();
    const map = new ActionMap<"zoom">([{ action: "zoom", control: { type: "wheel", direction: "down" } }]);
    map.attach(window);

    tabPage("Performance").dispatchEvent(wheelDown());
    expect(map.held("zoom")).toBe(false);

    host.dispatchEvent(wheelDown());
    expect(map.held("zoom")).toBe(true);
    map.detach();
  });

  it("does not pass the toggle key to the game", () => {
    devtoolsWith();
    const map = new ActionMap<"menu">([{ action: "menu", control: { type: "key", key: "F3" } }]);
    map.attach(window);
    document.body.dispatchEvent(key("F3"));
    expect(map.held("menu")).toBe(false);
    map.detach();
  });
});

describe("time controls drive the clock", () => {
  it("pause, resume and speed go through the clock, and refuse a speed the clock does not allow", () => {
    const clock = new GameClock({ speeds: [0.25, 0.5, 1, 2, 4] });
    const devtools = devtoolsWith({ clock });
    devtools.pause();
    expect(clock.paused).toBe(true);
    devtools.resume();
    expect(clock.paused).toBe(false);
    devtools.setSpeed(2);
    expect(clock.speed).toBe(2);
    expect(() => devtools.setSpeed(3)).toThrow(RangeError);
    expect(clock.speed).toBe(2);
  });

  it("slow motion runs the clock at the slow speed, and gives back the speed it had", () => {
    const clock = new GameClock({ speeds: [0.25, 0.5, 1, 2, 4] });
    const devtools = devtoolsWith({ clock });
    devtools.setSpeed(2);
    devtools.setSlowMotion(true);
    expect(clock.speed).toBe(0.25);
    devtools.setSlowMotion(false);
    expect(clock.speed).toBe(2);
  });

  it("a speed set by hand turns slow motion off", () => {
    const clock = new GameClock({ speeds: [0.25, 0.5, 1, 2, 4] });
    const devtools = devtoolsWith({ clock });
    devtools.setSlowMotion(true);
    devtools.setSpeed(1);
    expect(clock.speed).toBe(1);
    devtools.setSlowMotion(false);
    expect(clock.speed).toBe(1);
  });

  it("a step moves game time by one loop step and leaves the clock paused", () => {
    const clock = new GameClock({ speeds: [0.5, 1, 2] });
    const devtools = devtoolsWith({ clock });
    devtools.setSpeed(1);
    const start = clock.elapsed;
    devtools.step();
    expect(clock.paused).toBe(true);
    expect(clock.elapsed).toBeCloseTo(start + 1 / 60, 9);
    devtools.step();
    expect(clock.elapsed).toBeCloseTo(start + 2 / 60, 9);
    expect(clock.paused).toBe(true);
  });

  it("a game that paused its own clock keeps it paused when the tools resume", () => {
    const clock = new GameClock({ speeds: [1, 2] });
    const devtools = devtoolsWith({ clock });
    clock.pause();
    devtools.pause();
    devtools.resume();
    expect(clock.paused).toBe(true);
  });

  it("destroy lets go of a clock that the tools had paused, and undoes slow motion", () => {
    const clock = new GameClock({ speeds: [0.25, 1, 2] });
    const devtools = devtoolsWith({ clock });
    devtools.setSpeed(2);
    devtools.setSlowMotion(true);
    devtools.pause();
    devtools.destroy();
    expect(clock.paused).toBe(false);
    expect(clock.speed).toBe(2);
  });

  it("refuses the speed controls without a clock, with an error that says how to give one", () => {
    const devtools = devtoolsWith();
    expect(() => devtools.setSpeed(1)).toThrow("pass clock to createDevtools");
  });
});

describe("the wrapped loop", () => {
  it("a pause holds the game's update, and a step runs it once with the clock running", () => {
    const clock = new GameClock({ speeds: [1, 2] });
    const loopFrames = manualScheduler();
    const devtools = devtoolsWith({ clock, now: () => 0 });
    let updates = 0;
    const loop = new GameLoop(
      devtools.wrapLoop({
        update(dt: number) {
          updates += 1;
          clock.advance(dt);
        },
        render() {
          return undefined;
        },
        scheduler: loopFrames,
      }),
    );
    loop.start();
    loopFrames.tick(0);
    loopFrames.tick(1000 / 60);
    expect(updates).toBe(1);

    devtools.pause();
    const paused = clock.elapsed;
    loopFrames.tick((2 * 1000) / 60);
    expect(updates).toBe(1);
    expect(clock.elapsed).toBe(paused);

    devtools.step();
    loopFrames.tick((3 * 1000) / 60);
    expect(updates).toBe(2);
    expect(clock.elapsed).toBeCloseTo(paused + 1 / 60, 9);
    expect(clock.paused).toBe(true);

    loopFrames.tick((4 * 1000) / 60);
    expect(updates).toBe(2);
    expect(devtools.stats().updateMs).not.toBeNull();
    loop.stop();
  });

  it("a loop that is not wrapped keeps running, and the Performance tab says the split is not measured", () => {
    const devtools = devtoolsWith();
    devtools.show();
    scheduler.tick(0);
    expect(devtools.stats().updateMs).toBeNull();
    expect(query(".vdt-page .vdt-note").textContent).toContain("wrapLoop");
  });

  it("frame times come from the frame scheduler", () => {
    const devtools = devtoolsWith();
    for (const time of [0, 16, 33, 50, 100]) scheduler.tick(time);
    const stats = devtools.stats();
    expect(stats.frames).toBe(4);
    expect(stats.longFrames16).toBe(3);
    expect(stats.longFrames33).toBe(1);
    expect(stats.p50Ms).toBe(17);
    expect(stats.p95Ms).toBe(50);
    expect(stats.maxMs).toBe(50);
    expect(stats.fps).toBeCloseTo(40, 9);
  });

  it("reads the element count once a second, and resetCounters clears the frame counts", () => {
    const devtools = devtoolsWith();
    scheduler.tick(1000);
    expect(devtools.stats().domNodes).toBeGreaterThan(0);
    scheduler.tick(1040);
    scheduler.tick(1080);
    expect(devtools.stats().frames).toBe(2);
    devtools.resetCounters();
    expect(devtools.stats().frames).toBe(0);
  });
});

describe("the console", () => {
  it("runs the commands typed in the prompt and prints their results", () => {
    const clock = new GameClock({ speeds: [1, 2, 4] });
    devtoolsWith({ clock }).show();
    tabButton("Console").click();
    runCommand("speed 2");
    expect(clock.speed).toBe(2);
    expect(consoleText()).toContain("> speed 2");
    expect(consoleText()).toContain("speed x2");
  });

  it("help lists the commands, and help <name> shows the usage of one", () => {
    devtoolsWith().show();
    runCommand("help");
    expect(consoleText()).toContain("pause");
    expect(consoleText()).toContain("speed [value:number]");
    runCommand("help speed");
    expect(consoleText()).toContain("speed [value:number]");
  });

  it("prints an error for bad input, and carries on with the next command", () => {
    const clock = new GameClock({ speeds: [1, 2] });
    devtoolsWith({ clock }).show();
    runCommand("speed 7");
    expect(consoleText()).toContain("speed 7 is not allowed");
    runCommand("nope");
    expect(consoleText()).toContain('unknown command "nope"');
    runCommand("pause");
    expect(clock.paused).toBe(true);
  });

  it("runs a command that the game adds, checks its typed arguments, and forgets it when removed", () => {
    const devtools = devtoolsWith();
    devtools.show();
    const remove = devtools.command("tone", {
      args: [{ name: "level", type: "number" }],
      help: "sets a tone",
      run: (values) => `tone ${String(values["level"])}`,
    });
    runCommand("tone 4");
    expect(consoleText()).toContain("tone 4");
    runCommand("tone loud");
    expect(consoleText()).toContain("level must be a number");
    remove();
    runCommand("tone 4");
    expect(consoleText()).toContain('unknown command "tone"');
  });

  it("Up and Down browse the history, and Down past the newest gives back the draft", () => {
    devtoolsWith().show();
    runCommand("pause");
    runCommand("resume");
    prompt().value = "dra";
    prompt().dispatchEvent(key("ArrowUp"));
    expect(prompt().value).toBe("resume");
    prompt().dispatchEvent(key("ArrowUp"));
    expect(prompt().value).toBe("pause");
    prompt().dispatchEvent(key("ArrowDown"));
    expect(prompt().value).toBe("resume");
    prompt().dispatchEvent(key("ArrowDown"));
    expect(prompt().value).toBe("dra");
  });

  it("Tab completes a command name, and Escape leaves the prompt", () => {
    devtoolsWith().show();
    tabButton("Console").click();
    prompt().value = "spe";
    prompt().dispatchEvent(key("Tab"));
    expect(prompt().value).toBe("speed ");

    prompt().focus();
    expect(document.activeElement).toBe(prompt());
    prompt().dispatchEvent(key("Escape"));
    expect(document.activeElement).not.toBe(prompt());
  });
});

describe("watches", () => {
  it("shows a pinned value on the HUD while the panel is hidden, and removes it when unwatched", () => {
    const devtools = devtoolsWith();
    const unwatch = devtools.watch("enemies", () => 7);
    scheduler.tick(0);
    const hud = query<HTMLElement>(".vdt-hud");
    expect(hud.hidden).toBe(false);
    expect(hud.textContent).toContain("enemies");
    expect(hud.textContent).toContain("7");

    unwatch();
    scheduler.tick(200);
    expect(hud.hidden).toBe(true);
  });

  it("shows an error for a watch that throws, and keeps the others", () => {
    const devtools = devtoolsWith();
    devtools.watch("broken", () => {
      throw new Error("gone");
    });
    devtools.watch("ok", () => 1);
    scheduler.tick(0);
    const hud = query<HTMLElement>(".vdt-hud");
    expect(hud.textContent).toContain("broken");
    expect(hud.textContent).toContain("error: gone");
    expect(hud.textContent).toContain("ok");
    expect(hud.textContent).toContain("1");
  });
});

/** The corners that the HUD carries, as names without the class prefix. */
function corners(hud: HTMLElement): string[] {
  return [...hud.classList].filter((name) => name.startsWith("vdt-corner-")).map((name) => name.slice(11));
}

describe("the pinned HUD", () => {
  it("is drawn in the top-left corner by default", () => {
    devtoolsWith();
    expect(corners(query(".vdt-hud"))).toEqual(["top-left"]);
  });

  it("starts in the corner that the options name, with only that corner's class", () => {
    devtoolsWith({ hudCorner: "bottom-right" });
    expect(corners(query(".vdt-hud"))).toEqual(["bottom-right"]);
  });

  it("moves to another corner with setHudCorner, and drops the class of the old one", () => {
    const devtools = devtoolsWith();
    devtools.setHudCorner("top-right");
    expect(corners(query(".vdt-hud"))).toEqual(["top-right"]);
    devtools.setHudCorner("bottom-left");
    expect(corners(query(".vdt-hud"))).toEqual(["bottom-left"]);
  });

  it("keeps the watch values on screen after it moves", () => {
    const devtools = devtoolsWith({ hudCorner: "top-right" });
    devtools.watch("gold", () => 12);
    scheduler.tick(0);
    devtools.setHudCorner("bottom-right");
    scheduler.tick(200);
    const hud = query<HTMLElement>(".vdt-hud");
    expect(hud.hidden).toBe(false);
    expect(hud.textContent).toContain("gold");
    expect(hud.textContent).toContain("12");
  });

  it("refuses a corner that is not one of the four, and leaves the HUD where it was", () => {
    const devtools = devtoolsWith({ hudCorner: "top-right" });
    expect(() => devtools.setHudCorner("middle" as HudCorner)).toThrow(RangeError);
    expect(corners(query(".vdt-hud"))).toEqual(["top-right"]);
  });

  it("refuses a bad corner in the options before it builds anything", () => {
    expect(() => devtoolsWith({ hudCorner: "centre" as HudCorner })).toThrow(RangeError);
    expect(document.querySelector(".vdt-root")).toBeNull();
    expect(document.getElementById(STYLE_ID)).toBeNull();
    expect(host.children).toHaveLength(0);
  });

  it("stays hidden with hud false, and its watches are never read", () => {
    const devtools = devtoolsWith({ hud: false });
    const read = vi.fn(() => 5);
    devtools.watch("enemies", read);
    scheduler.tick(0);
    scheduler.tick(200);
    expect(query<HTMLElement>(".vdt-hud").hidden).toBe(true);
    expect(read).not.toHaveBeenCalled();
  });

  it("still takes a corner while it is off", () => {
    const devtools = devtoolsWith({ hud: false });
    expect(() => devtools.setHudCorner("bottom-left")).not.toThrow();
    expect(corners(query(".vdt-hud"))).toEqual(["bottom-left"]);
    expect(query<HTMLElement>(".vdt-hud").hidden).toBe(true);
  });

  it("does nothing when the tools are disabled", () => {
    const devtools = createDevtools({ host, enabled: false, hudCorner: "bottom-left" });
    expect(() => devtools.setHudCorner("top-right")).not.toThrow();
    expect(document.querySelector(".vdt-root")).toBeNull();
  });
});

describe("state views", () => {
  it("shows a view in the State tab, and opens a row when it is clicked", () => {
    const devtools = devtoolsWith();
    devtools.inspect("player", () => ({ hp: 3, pos: { x: 1 } }));
    devtools.show();
    tabButton("State").click();

    const labels = () => [...document.querySelectorAll(".vdt-label")].map((label) => label.textContent);
    expect(labels()).toContain("hp");
    expect(labels()).not.toContain("x");

    const position = [...document.querySelectorAll(".vdt-row")].find(
      (row) => row.querySelector(".vdt-label")?.textContent === "pos",
    );
    position?.querySelector<HTMLButtonElement>(".vdt-caret")?.click();
    expect(labels()).toContain("x");
  });

  it("a read that throws keeps the last value and shows the error", () => {
    const devtools = devtoolsWith();
    let calls = 0;
    devtools.inspect("flaky", () => {
      calls += 1;
      if (calls > 1) throw new Error("gone");
      return { ok: true };
    });
    devtools.show();
    tabButton("State").click();
    viewButton("Refresh").click();

    const status = query<HTMLElement>(".vdt-view .vdt-status.is-error");
    expect(status.textContent).toBe("Read failed: gone");
    expect(document.querySelector(".vdt-view")?.textContent).toContain("ok");
  });

  it("a payload that cannot be read does not break the game's emit", () => {
    interface Events {
      odd: object;
    }
    const bus = new EventEmitter<Events>();
    devtoolsWith({ events: bus, eventNames: ["odd"] }).show();
    const hostile = new Proxy(
      {},
      {
        ownKeys() {
          throw new Error("no keys");
        },
      },
    );
    expect(() => bus.emit("odd", hostile)).not.toThrow();
    tabButton("Events").click();
    expect(query(".vdt-event-summary").textContent).toContain("unreadable");
  });

  it("removing a view removes its section", () => {
    const devtools = devtoolsWith();
    const remove = devtools.inspect("temp", () => 1);
    devtools.show();
    tabButton("State").click();
    expect(document.querySelectorAll(".vdt-view")).toHaveLength(1);
    remove();
    scheduler.tick(0);
    expect(document.querySelectorAll(".vdt-view")).toHaveLength(0);
  });
});

describe("the event log", () => {
  it("logs the named events from the bus, and the pause and clear buttons work", () => {
    interface Events {
      day: { day: number };
      save: void;
    }
    const bus = new EventEmitter<Events>();
    const devtools = devtoolsWith({ events: bus, eventNames: ["day"] });
    bus.emit("day", { day: 2 });
    bus.emit("save");
    devtools.show();
    tabButton("Events").click();

    const names = () => [...document.querySelectorAll(".vdt-event-name")].map((name) => name.textContent);
    expect(names()).toEqual(["day"]);
    expect(query(".vdt-event-summary").textContent).toBe('{"day":2}');

    const pauseButton = [...document.querySelectorAll<HTMLButtonElement>("button")].find(
      (button) => button.textContent === "Pause log",
    );
    pauseButton?.click();
    bus.emit("day", { day: 3 });
    scheduler.tick(1000);
    expect(names()).toEqual(["day"]);
    expect(tabPage("Events").textContent).toContain("1 dropped while paused");

    const clearButton = [...document.querySelectorAll<HTMLButtonElement>("button")].find(
      (button) => button.textContent === "Clear",
    );
    clearButton?.click();
    expect(names()).toEqual([]);
  });
});

describe("the loop's frame hooks and the event tap", () => {
  it("reads the update and render times from the loop's frame hooks when the loop is given, without wrapLoop", () => {
    const loopFrames = manualScheduler();
    let nowMs = 0;
    const loop = new GameLoop({
      rate: 10,
      scheduler: loopFrames,
      now: () => nowMs,
      update: () => {
        nowMs += 4;
      },
      render: () => {
        nowMs += 6;
      },
    });
    const devtools = devtoolsWith({ loop });
    loop.start();
    loopFrames.tick(0);
    loopFrames.tick(250);
    // The first frame renders for 6 ms with no step. The second runs two steps of 4 ms, then renders for 6 ms.
    expect(devtools.stats().updateMs).toBe(4);
    expect(devtools.stats().renderMs).toBe(6);
    loop.stop();
  });

  it("logs every event of the bus through a tap, newest first, when no names are given", () => {
    interface Events {
      day: { day: number };
      save: void;
    }
    const bus = new EventEmitter<Events>();
    devtoolsWith({ events: bus }).show();
    bus.emit("day", { day: 1 });
    bus.emit("save");
    tabButton("Events").click();
    const names = [...document.querySelectorAll(".vdt-event-name")].map((name) => name.textContent);
    expect(names).toEqual(["save", "day"]);
  });

  it("logs only the names it is given, when names are given to narrow the log", () => {
    interface Events {
      day: { day: number };
      save: void;
    }
    const bus = new EventEmitter<Events>();
    devtoolsWith({ events: bus, eventNames: ["save"] }).show();
    bus.emit("day", { day: 1 });
    bus.emit("save");
    tabButton("Events").click();
    const names = [...document.querySelectorAll(".vdt-event-name")].map((name) => name.textContent);
    expect(names).toEqual(["save"]);
  });
});

describe("disabled and destroyed tools", () => {
  it("when disabled, adds no element, no style and no listener, and every method does nothing", () => {
    const clock = new GameClock({ speeds: [1, 2] });
    const addListener = vi.spyOn(window, "addEventListener");
    const before = host.innerHTML;
    const devtools = createDevtools({ host, enabled: false, scheduler, clock });

    expect(devtools.enabled).toBe(false);
    expect(devtools.visible).toBe(false);
    expect(host.innerHTML).toBe(before);
    expect(document.getElementById(STYLE_ID)).toBeNull();
    expect(addListener.mock.calls.some((call) => call[0] === "keydown")).toBe(false);

    document.body.dispatchEvent(key("F3"));
    expect(devtools.visible).toBe(false);
    devtools.pause();
    expect(clock.paused).toBe(false);
    expect(devtools.inspect("x", () => 1)).toBeTypeOf("function");
    expect(devtools.watch("x", () => 1)).toBeTypeOf("function");
    expect(devtools.stats().frames).toBe(0);
    devtools.destroy();
    expect(host.innerHTML).toBe(before);
  });

  it("wrapLoop returns the same options when disabled, so the loop runs as it would without the tools", () => {
    const devtools = createDevtools({ host, enabled: false, scheduler });
    const options = {
      update(): void {
        return undefined;
      },
    };
    expect(devtools.wrapLoop(options)).toBe(options);
  });

  it("destroy removes the panel and the frame loop, and the toggle key stops working", () => {
    const devtools = devtoolsWith();
    devtools.show();
    expect(scheduler.waiting()).toBe(1);

    devtools.destroy();
    expect(document.querySelector(".vdt-root")).toBeNull();
    expect(scheduler.waiting()).toBe(0);

    document.body.dispatchEvent(key("F3"));
    expect(devtools.visible).toBe(true);
    devtools.destroy();
  });
});

/** The buttons of the Time tab that give a speed. Their text starts with the sign, unlike the slow-motion button. */
function speedButtons(): HTMLButtonElement[] {
  return [...tabPage("Time").querySelectorAll<HTMLButtonElement>("button")].filter(
    (button) => button.textContent?.startsWith("×") === true,
  );
}

/** The names of the events that the Events tab lists, newest first. */
function listedEvents(): Array<string | null> {
  return [...document.querySelectorAll(".vdt-event-name")].map((name) => name.textContent);
}

/** The notice of the Events tab, which is shown while no event bus is connected. */
function eventsNotice(): HTMLElement {
  const notice = tabPage("Events").querySelector<HTMLElement>(".vdt-note");
  if (notice === null) throw new Error("no notice in the Events tab");
  return notice;
}

describe("a clock and an event bus that are connected after the tools were made", () => {
  it("connects a clock made later: the Time tab shows its speeds, and the controls act on it", () => {
    const devtools = devtoolsWith();
    devtools.show();
    tabButton("Time").click();
    scheduler.tick(0);
    expect(tabPage("Time").textContent).toContain("No game clock is connected");
    expect(speedButtons()).toHaveLength(0);

    const clock = new GameClock({ speeds: [0.25, 1, 2] });
    devtools.attachClock(clock);
    scheduler.tick(200);
    expect(speedButtons().map((button) => button.textContent)).toEqual(["×0.25", "×1", "×2"]);
    expect(tabPage("Time").textContent).not.toContain("No game clock");

    speedButtons()[2]?.click();
    expect(clock.speed).toBe(2);
    devtools.pause();
    expect(clock.paused).toBe(true);
  });

  it("gives the clock back when it is detached: it runs again, slow motion is undone, and the Time tab says so", () => {
    const clock = new GameClock({ speeds: [0.25, 1, 2] });
    const devtools = devtoolsWith({ clock });
    devtools.setSpeed(2);
    devtools.setSlowMotion(true);
    devtools.pause();
    expect(clock.paused).toBe(true);
    expect(clock.speed).toBe(0.25);

    devtools.attachClock(null);
    expect(clock.paused).toBe(false);
    expect(clock.speed).toBe(2);
    expect(() => devtools.setSpeed(1)).toThrow("pass clock to createDevtools");

    devtools.show();
    tabButton("Time").click();
    scheduler.tick(0);
    expect(tabPage("Time").textContent).toContain("No game clock is connected");
    expect(speedButtons()).toHaveLength(0);
  });

  it("gives back the clock it replaces, and the controls then act on the new one only", () => {
    const first = new GameClock({ speeds: [0.25, 1, 2] });
    const second = new GameClock({ speeds: [0.25, 1, 2] });
    const devtools = devtoolsWith({ clock: first });
    devtools.setSpeed(2);
    devtools.setSlowMotion(true);
    const secondSpeed = second.speed;
    devtools.attachClock(second);
    expect(first.speed).toBe(2);
    expect(second.speed).toBe(secondSpeed);
    devtools.setSpeed(1);
    expect(second.speed).toBe(1);
    expect(first.speed).toBe(2);
  });

  it("pauses a clock connected while the tools are paused, and resumes it when the tools resume", () => {
    const first = new GameClock({ speeds: [1, 2] });
    const second = new GameClock({ speeds: [1, 2] });
    const devtools = devtoolsWith({ clock: first });
    devtools.pause();
    devtools.attachClock(second);
    expect(first.paused).toBe(false);
    expect(second.paused).toBe(true);
    devtools.resume();
    expect(second.paused).toBe(false);
  });

  it("does not resume a clock that the game paused itself when the tools let go of it", () => {
    const game = new GameClock({ speeds: [1, 2] });
    const devtools = devtoolsWith({ clock: game });
    game.pause();
    devtools.pause();
    devtools.attachClock(null);
    expect(game.paused).toBe(true);
  });

  it("logs the events of a bus connected later, and clears the log when another bus is connected", () => {
    interface Events {
      day: { day: number };
      save: void;
    }
    const first = new EventEmitter<Events>();
    const second = new EventEmitter<Events>();
    const devtools = devtoolsWith();
    devtools.show();
    tabButton("Events").click();
    first.emit("day", { day: 1 });
    scheduler.tick(0);
    expect(listedEvents()).toEqual([]);
    expect(eventsNotice().hidden).toBe(false);

    devtools.attachEvents(first);
    first.emit("day", { day: 2 });
    scheduler.tick(200);
    expect(listedEvents()).toEqual(["day"]);
    expect(eventsNotice().hidden).toBe(true);

    devtools.attachEvents(second);
    scheduler.tick(400);
    expect(listedEvents()).toEqual([]);
    first.emit("save");
    second.emit("save");
    scheduler.tick(600);
    expect(listedEvents()).toEqual(["save"]);
  });

  it("stops logging when the bus is detached, and the tab then says that no bus is connected", () => {
    interface Events {
      save: void;
    }
    const bus = new EventEmitter<Events>();
    const devtools = devtoolsWith({ events: bus });
    devtools.show();
    tabButton("Events").click();
    bus.emit("save");
    scheduler.tick(0);
    expect(listedEvents()).toEqual(["save"]);

    const record = vi.spyOn(EventLog.prototype, "record");
    devtools.attachEvents(null);
    bus.emit("save");
    expect(record).not.toHaveBeenCalled();
    scheduler.tick(200);
    expect(listedEvents()).toEqual([]);
    expect(eventsNotice().hidden).toBe(false);
  });

  it("keeps only the names it is given when a bus is connected with names", () => {
    interface Events {
      day: { day: number };
      save: void;
    }
    const bus = new EventEmitter<Events>();
    const devtools = devtoolsWith();
    devtools.show();
    tabButton("Events").click();
    devtools.attachEvents(bus, ["save"]);
    bus.emit("day", { day: 1 });
    bus.emit("save");
    scheduler.tick(0);
    expect(listedEvents()).toEqual(["save"]);
  });

  it("stops the tap of the connected bus when the tools are destroyed, and ignores a bus connected after that", () => {
    interface Events {
      save: void;
    }
    const bus = new EventEmitter<Events>();
    const devtools = devtoolsWith({ events: bus });
    const record = vi.spyOn(EventLog.prototype, "record");
    devtools.destroy();
    bus.emit("save");
    expect(record).not.toHaveBeenCalled();

    const later = new EventEmitter<Events>();
    devtools.attachEvents(later);
    later.emit("save");
    expect(record).not.toHaveBeenCalled();
  });

  it("ignores the clock and the bus that are connected later when the tools are disabled", () => {
    const devtools = createDevtools({ host, enabled: false, scheduler });
    expect(() => devtools.attachClock(new GameClock({ speeds: [1, 2] }))).not.toThrow();
    expect(() => devtools.attachEvents(new EventEmitter<{ save: void }>())).not.toThrow();
  });
});
