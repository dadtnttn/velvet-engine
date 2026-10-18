// @vitest-environment happy-dom
// Tests for the screen stack: what is shown and hidden, the hooks in their order, the focus that comes back to
// each screen, the back action (Escape and pad B), the transitions, and the cleanup.

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { createScreenStack, type Screen, type ScreenKind, type ScreenStack } from "./screens";

/** A screen with a button in it, and a log of the hooks that ran on it. */
interface Probe {
  readonly screen: Screen;
  readonly log: string[];
  readonly button: HTMLButtonElement;
}

/**
 * Makes a screen whose hooks write their names into the log.
 * @param id - the screen's name.
 * @param kind - the kind of screen.
 * @param extra - hooks to add or replace.
 */
function probe(id: string, kind?: ScreenKind, extra: Partial<Screen> = {}): Probe {
  const el = document.createElement("div");
  const button = document.createElement("button");
  button.textContent = `${id}-button`;
  el.append(button);
  const log: string[] = [];
  const screen: Screen = {
    id,
    el,
    kind,
    onEnter: () => log.push(`enter:${id}`),
    onLeave: () => log.push(`leave:${id}`),
    onPause: () => log.push(`pause:${id}`),
    onResume: () => log.push(`resume:${id}`),
    ...extra,
  };
  return { screen, log, button };
}

/** Presses Escape on the focused element, the way the keyboard and a pad's B do. */
function pressEscape(): KeyboardEvent {
  const target = document.activeElement ?? document.body;
  const event = new KeyboardEvent("keydown", { key: "Escape", bubbles: true, cancelable: true });
  target.dispatchEvent(event);
  return event;
}

let host: HTMLElement;
let stack: ScreenStack;

beforeEach(() => {
  host = document.createElement("div");
  document.body.append(host);
  stack = createScreenStack(host, { transitionMs: 0 });
});

afterEach(() => {
  stack.destroy();
  document.body.innerHTML = "";
  vi.useRealTimers();
});

describe("showing and hiding", () => {
  it("shows the pushed screen, puts it in the layer and gives it the focus", () => {
    const a = probe("a");
    stack.push(a.screen);
    expect(stack.top?.id).toBe("a");
    expect(stack.depth).toBe(1);
    expect(a.screen.el.parentElement).toBe(stack.el);
    expect(a.screen.el.dataset["screen"]).toBe("a");
    expect(a.screen.el.hidden).toBe(false);
    expect(a.screen.el.hasAttribute("inert")).toBe(false);
    expect(document.activeElement).toBe(a.button);
  });

  it("hides the screens under a screen, and keeps them under an overlay", () => {
    const a = probe("a");
    const b = probe("b");
    const c = probe("c", "overlay");
    stack.push(a.screen);
    stack.push(b.screen);
    expect(a.screen.el.hidden).toBe(true);
    expect(b.screen.el.hidden).toBe(false);
    stack.push(c.screen);
    expect(b.screen.el.hidden).toBe(false);
    expect(c.screen.el.hidden).toBe(false);
    expect(c.screen.el.classList.contains("vu-screen-overlay")).toBe(true);
    expect(stack.ids()).toEqual(["a", "b", "c"]);
  });

  it("makes every screen under the top one inert, so the pointer and the focus cannot reach it", () => {
    const a = probe("a");
    const b = probe("b", "overlay");
    stack.push(a.screen);
    stack.push(b.screen);
    expect(a.screen.el.hasAttribute("inert")).toBe(true);
    expect(b.screen.el.hasAttribute("inert")).toBe(false);
    stack.pop();
    expect(a.screen.el.hasAttribute("inert")).toBe(false);
  });

  it("has, ids and depth describe the stack from the bottom up", () => {
    stack.push(probe("first").screen);
    stack.push(probe("second").screen);
    expect(stack.has("first")).toBe(true);
    expect(stack.has("third")).toBe(false);
    expect(stack.ids()).toEqual(["first", "second"]);
    expect(stack.depth).toBe(2);
  });

  it("refuses a second screen with a name that is already on the stack", () => {
    stack.push(probe("title").screen);
    expect(() => stack.push(probe("title").screen)).toThrow(/already on the stack/);
    expect(stack.depth).toBe(1);
  });

  it("reports every change to its listeners, and stops after the listener is removed", () => {
    const listener = vi.fn();
    const off = stack.onChange(listener);
    const a = probe("a");
    stack.push(a.screen);
    expect(listener).toHaveBeenLastCalledWith(a.screen);
    stack.pop();
    expect(listener).toHaveBeenLastCalledWith(null);
    off();
    stack.push(probe("b").screen);
    expect(listener).toHaveBeenCalledTimes(2);
  });

  it("uses the focus target of a screen when it has one", () => {
    const a = probe("a");
    const other = document.createElement("input");
    a.screen.el.append(other);
    stack.push({ ...a.screen, focusTarget: () => other });
    expect(document.activeElement).toBe(other);
  });
});

describe("the hooks", () => {
  it("pauses the screen under a new one and resumes it when that one leaves", () => {
    const a = probe("a");
    const b = probe("b");
    stack.push(a.screen);
    stack.push(b.screen);
    stack.pop();
    expect(a.log).toEqual(["enter:a", "pause:a", "resume:a"]);
    expect(b.log).toEqual(["enter:b", "leave:b"]);
  });

  it("does not pause a screen twice when two screens are stacked over it", () => {
    const a = probe("a");
    stack.push(a.screen);
    stack.push(probe("b", "overlay").screen);
    stack.push(probe("c", "overlay").screen);
    expect(a.log).toEqual(["enter:a", "pause:a"]);
  });

  it("replace swaps the top screen and leaves the screens under it alone", () => {
    const a = probe("a");
    const b = probe("b");
    const c = probe("c");
    stack.push(a.screen);
    stack.push(b.screen);
    stack.replace(c.screen);
    expect(stack.ids()).toEqual(["a", "c"]);
    expect(a.log).toEqual(["enter:a", "pause:a"]);
    expect(b.log).toEqual(["enter:b", "leave:b"]);
    expect(c.log).toEqual(["enter:c"]);
    expect(document.activeElement).toBe(c.button);
  });

  it("replace pushes when the stack is empty", () => {
    stack.replace(probe("only").screen);
    expect(stack.ids()).toEqual(["only"]);
  });

  it("pop returns the screen it took off and resumes the one below", () => {
    const a = probe("a");
    const b = probe("b");
    stack.push(a.screen);
    stack.push(b.screen);
    expect(stack.pop()).toBe(b.screen);
    expect(stack.top?.id).toBe("a");
    expect(a.log.at(-1)).toBe("resume:a");
  });

  it("pop on an empty stack gives null", () => {
    expect(stack.pop()).toBeNull();
  });

  it("popTo takes the screens above a name off, top first, and resumes that screen once", () => {
    const a = probe("a");
    const b = probe("b");
    const c = probe("c");
    stack.push(a.screen);
    stack.push(b.screen);
    stack.push(c.screen);
    expect(stack.popTo("a")).toBe(true);
    expect(stack.ids()).toEqual(["a"]);
    expect(c.log).toEqual(["enter:c", "leave:c"]);
    expect(b.log).toEqual(["enter:b", "pause:b", "leave:b"]);
    expect(a.log).toEqual(["enter:a", "pause:a", "resume:a"]);
  });

  it("popTo gives false for a name that is not on the stack, and true for the top one without changing it", () => {
    const a = probe("a");
    stack.push(a.screen);
    expect(stack.popTo("missing")).toBe(false);
    expect(stack.popTo("a")).toBe(true);
    expect(a.log).toEqual(["enter:a"]);
  });
});

describe("back and Escape", () => {
  it("pops a screen without onBack when Escape is pressed, and marks the key as taken", () => {
    stack.push(probe("a").screen);
    stack.push(probe("b").screen);
    const event = pressEscape();
    expect(stack.ids()).toEqual(["a"]);
    expect(event.defaultPrevented).toBe(true);
  });

  it("does nothing on the only screen, and leaves the key free", () => {
    stack.push(probe("only").screen);
    const event = pressEscape();
    expect(stack.depth).toBe(1);
    expect(event.defaultPrevented).toBe(false);
  });

  it("keeps a screen whose onBack returns false", () => {
    const a = probe("a", undefined, { onBack: () => false });
    stack.push(a.screen);
    stack.push(probe("b").screen);
    stack.pop();
    expect(stack.back()).toBe(true);
    expect(stack.ids()).toEqual(["a"]);
  });

  it("keeps the screen that pushed another one in onBack, and does not pop the new one", () => {
    const pause = probe("pause", "overlay");
    const game = probe("game", undefined, {
      onBack: () => {
        stack.push(pause.screen);
        return false;
      },
    });
    stack.push(game.screen);
    expect(stack.back()).toBe(true);
    expect(stack.ids()).toEqual(["game", "pause"]);
    expect(game.log).toEqual(["enter:game", "pause:game"]);
  });

  it("does not pop after onBack changed the stack without returning false", () => {
    const pause = probe("pause", "overlay");
    const game = probe("game", undefined, {
      onBack: () => {
        stack.push(pause.screen);
      },
    });
    stack.push(game.screen);
    stack.back();
    expect(stack.ids()).toEqual(["game", "pause"]);
  });

  it("pops a screen whose onBack returns nothing, when it is not the only one", () => {
    stack.push(probe("under").screen);
    stack.push(probe("top", undefined, { onBack: () => undefined }).screen);
    stack.back();
    expect(stack.ids()).toEqual(["under"]);
  });

  it("leaves Escape alone when an earlier listener already took it", () => {
    const blocker = (event: KeyboardEvent): void => {
      event.preventDefault();
    };
    document.addEventListener("keydown", blocker);
    stack.push(probe("a").screen);
    stack.push(probe("b").screen);
    pressEscape();
    document.removeEventListener("keydown", blocker);
    expect(stack.ids()).toEqual(["a", "b"]);
  });

  it("ignores a held Escape after its first press", () => {
    stack.push(probe("a").screen);
    stack.push(probe("b").screen);
    const repeat = new KeyboardEvent("keydown", { key: "Escape", repeat: true, bubbles: true, cancelable: true });
    document.body.dispatchEvent(repeat);
    expect(stack.depth).toBe(2);
  });
});

describe("focus", () => {
  it("gives the focus back to the element a screen had when another one covered it", () => {
    const a = probe("a");
    stack.push(a.screen);
    const second = document.createElement("button");
    a.screen.el.append(second);
    second.focus();
    stack.push(probe("b").screen);
    expect(document.activeElement).not.toBe(second);
    stack.pop();
    expect(document.activeElement).toBe(second);
  });

  it("falls back to the first control of a screen when the saved element is gone", () => {
    const a = probe("a");
    stack.push(a.screen);
    const gone = document.createElement("button");
    a.screen.el.append(gone);
    gone.focus();
    stack.push(probe("b").screen);
    gone.remove();
    stack.pop();
    expect(document.activeElement).toBe(a.button);
  });

  it("brings the focus back into the top screen when a click on empty space takes it away", async () => {
    const a = probe("a");
    stack.push(a.screen);
    a.button.blur();
    a.button.dispatchEvent(new FocusEvent("focusout", { bubbles: true, relatedTarget: null }));
    await Promise.resolve();
    expect(document.activeElement).toBe(a.button);
  });
});

describe("transitions", () => {
  it("keeps a leaving screen in the layer until the transition has run, and then removes it", () => {
    vi.useFakeTimers();
    stack.destroy();
    stack = createScreenStack(host, { transitionMs: 100 });
    const a = probe("a");
    const b = probe("b");
    stack.push(a.screen);
    stack.push(b.screen);
    stack.pop();
    expect(b.screen.el.parentElement).toBe(stack.el);
    expect(b.screen.el.dataset["phase"]).toBe("leave");
    expect(b.screen.el.hasAttribute("inert")).toBe(true);
    vi.advanceTimersByTime(100);
    expect(b.screen.el.parentElement).toBeNull();
  });

  it("starts a screen from its enter phase, and the phase is gone once the screen is shown", () => {
    stack.destroy();
    stack = createScreenStack(host, { transitionMs: 100 });
    const a = probe("a");
    stack.push(a.screen);
    expect(a.screen.el.dataset["phase"]).toBeUndefined();
  });

  it("gives the layer the transition length, which the stylesheet reads", () => {
    stack.destroy();
    stack = createScreenStack(host, { transitionMs: 250 });
    expect(stack.el.style.getPropertyValue("--vu-screen-ms")).toBe("250ms");
  });

  it("reuses a screen that was popped and is pushed again", () => {
    const a = probe("a");
    stack.push(a.screen);
    stack.pop();
    stack.push(a.screen);
    expect(a.screen.el.parentElement).toBe(stack.el);
    expect(a.screen.el.hidden).toBe(false);
    expect(a.log).toEqual(["enter:a", "leave:a", "enter:a"]);
  });

  it("refuses a negative transition length", () => {
    expect(() => createScreenStack(host, { transitionMs: -1 })).toThrow(RangeError);
  });
});

describe("a pad", () => {
  it("B goes back: it arrives as an Escape keydown on the focused element, as startPadMenuKeys sends it", () => {
    stack.push(probe("a").screen);
    stack.push(probe("b").screen);
    const padB = new KeyboardEvent("keydown", { key: "Escape", code: "Escape", bubbles: true, cancelable: true });
    (document.activeElement ?? document.body).dispatchEvent(padB);
    expect(stack.ids()).toEqual(["a"]);
    expect(padB.defaultPrevented).toBe(true);
  });
});

describe("cleanup", () => {
  it("removes the layer, and gives a static host back its position", () => {
    const fresh = document.createElement("div");
    document.body.append(fresh);
    const own = createScreenStack(fresh, { transitionMs: 0 });
    expect(fresh.style.position).toBe("relative");
    own.destroy();
    expect(fresh.querySelector(".vu-screens")).toBeNull();
    expect(fresh.style.position).toBe("");
  });

  it("stops answering Escape once it is destroyed, so no hook runs", () => {
    const onBack = vi.fn();
    const onLeave = vi.fn();
    stack.push(probe("a", undefined, { onBack, onLeave }).screen);
    stack.destroy();
    pressEscape();
    expect(onBack).not.toHaveBeenCalled();
    expect(onLeave).not.toHaveBeenCalled();
  });

  it("does nothing after it is destroyed", () => {
    const a = probe("a");
    stack.destroy();
    stack.push(a.screen);
    expect(stack.depth).toBe(0);
    expect(stack.back()).toBe(false);
  });

  it("makes a static host positioned, so the layer covers it", () => {
    stack.destroy();
    const plain = document.createElement("div");
    plain.style.position = "static";
    document.body.append(plain);
    stack = createScreenStack(plain, { transitionMs: 0 });
    expect(plain.style.position).toBe("relative");
    stack.destroy();
    expect(plain.style.position).toBe("static");
    stack = createScreenStack(host, { transitionMs: 0 });
  });
});
