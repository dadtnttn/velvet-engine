// @vitest-environment happy-dom
// Tests of the action map on a page: the controls keep their own keys, a map can be scoped to an
// element, and a UI handler can use a key before the map sees it. The events are real DOM events on
// real elements, so the bubbling and the focus events come from the page, as they do in a browser.

import { afterEach, describe, expect, it } from "vitest";
import { ActionMap, markHandled, type Binding } from "./input";

type Action = "confirm" | "pause";

const CONFIRM: Binding<Action> = { action: "confirm", control: { type: "key", key: "Enter" } };
const PAUSE: Binding<Action> = { action: "pause", control: { type: "key", key: "Space" } };

afterEach(() => {
  document.body.replaceChildren();
});

/** The key that a keyboard sends for a physical key: its character, or its name. */
function keyOf(code: string): string {
  return code === "Space" ? " " : code;
}

/** A keydown as the browser sends it: it bubbles, and it can be cancelled. */
function keyDown(code: string): KeyboardEvent {
  return new KeyboardEvent("keydown", { code, key: keyOf(code), bubbles: true, cancelable: true });
}

/** A keyup as the browser sends it. */
function keyUp(code: string): KeyboardEvent {
  return new KeyboardEvent("keyup", { code, key: keyOf(code), bubbles: true, cancelable: true });
}

/** A new element with the given attributes. It is not in the page until it is appended. */
function element(tag: string, attributes: Record<string, string> = {}): HTMLElement {
  const node = document.createElement(tag);
  for (const [name, value] of Object.entries(attributes)) node.setAttribute(name, value);
  return node;
}

describe("controls keep their own keys", () => {
  const controls: Array<[string, string, Record<string, string>]> = [
    ["an input", "input", {}],
    ["a textarea", "textarea", {}],
    ["a select", "select", {}],
    ["a button", "button", {}],
    ["an editable element", "div", { contenteditable: "true" }],
    ["a menu item", "div", { role: "menuitem" }],
  ];

  it.each(controls)("does not count the keys pressed on %s", (_label, tag, attributes) => {
    const control = element(tag, attributes);
    document.body.append(control);
    const map = new ActionMap<Action>([CONFIRM, PAUSE]);
    map.attach(window);
    control.dispatchEvent(keyDown("Enter"));
    control.dispatchEvent(keyDown("Space"));
    expect(map.pressed("confirm")).toBe(false);
    expect(map.held("pause")).toBe(false);
    map.detach();
  });

  it("does not count the keys pressed inside a control, such as a span in a button", () => {
    const item = element("div", { role: "menuitem" });
    const label = element("span");
    item.append(label);
    document.body.append(item);
    const map = new ActionMap<Action>([CONFIRM]);
    map.attach(window);
    label.dispatchEvent(keyDown("Enter"));
    expect(map.held("confirm")).toBe(false);
    map.detach();
  });

  it("counts the keys pressed on other elements, on the page, and on an element that is not editable", () => {
    const plain = element("div");
    const fixed = element("div", { contenteditable: "false" });
    document.body.append(plain, fixed);
    const map = new ActionMap<Action>([CONFIRM, PAUSE]);
    map.attach(window);

    plain.dispatchEvent(keyDown("Space"));
    expect(map.held("pause")).toBe(true);
    document.body.dispatchEvent(keyUp("Space"));
    expect(map.released("pause")).toBe(true);

    fixed.dispatchEvent(keyDown("Enter"));
    expect(map.pressed("confirm")).toBe(true);
    map.detach();
  });

  it("counts the keys pressed on a control when the map captures controls", () => {
    const button = element("button");
    document.body.append(button);
    const map = new ActionMap<Action>([CONFIRM], { captureControls: true });
    map.attach(window);
    button.dispatchEvent(keyDown("Enter"));
    expect(map.pressed("confirm")).toBe(true);
    map.detach();
  });

  it("releases a key that counted, when it comes up on a control that took the focus", () => {
    const plain = element("div");
    const button = element("button");
    document.body.append(plain, button);
    const map = new ActionMap<Action>([PAUSE]);
    map.attach(window);
    plain.dispatchEvent(keyDown("Space"));
    expect(map.held("pause")).toBe(true);
    button.dispatchEvent(keyUp("Space"));
    expect(map.released("pause")).toBe(true);
    expect(map.held("pause")).toBe(false);
    map.detach();
  });
});

describe("a map scoped to an element", () => {
  it("sees only the keys pressed inside its element", () => {
    const scope = element("section");
    const inside = element("div");
    const outside = element("div");
    scope.append(inside);
    document.body.append(scope, outside);
    const map = new ActionMap<Action>([CONFIRM, PAUSE]);
    map.attach(scope);

    outside.dispatchEvent(keyDown("Space"));
    expect(map.held("pause")).toBe(false);

    inside.dispatchEvent(keyDown("Enter"));
    expect(map.held("confirm")).toBe(true);
    map.detach();
  });

  it("releases its held keys when focus leaves it, and keeps them while focus stays inside", () => {
    const scope = element("section");
    const pad = element("div", { tabindex: "0" });
    const field = element("input");
    scope.append(pad, field);
    const outside = element("button");
    document.body.append(scope, outside);
    const map = new ActionMap<Action>([CONFIRM]);
    map.attach(scope);

    pad.focus();
    pad.dispatchEvent(keyDown("Enter"));
    expect(map.held("confirm")).toBe(true);

    field.focus();
    expect(map.held("confirm")).toBe(true);

    outside.focus();
    expect(map.held("confirm")).toBe(false);
    expect(map.released("confirm")).toBe(true);
    map.detach();
  });
});

describe("keys that a UI handler has used", () => {
  it("does not count a key whose handler prevented its default, on the element it went to", () => {
    const widget = element("div", { tabindex: "0" });
    document.body.append(widget);
    widget.addEventListener("keydown", (event) => {
      event.preventDefault();
    });
    const map = new ActionMap<Action>([CONFIRM]);
    map.attach(window);
    const event = keyDown("Enter");
    widget.dispatchEvent(event);
    expect(event.defaultPrevented).toBe(true);
    expect(map.held("confirm")).toBe(false);
    map.detach();
  });

  it("does not count a key that a handler marked, and leaves its default alone", () => {
    const widget = element("div", { tabindex: "0" });
    document.body.append(widget);
    widget.addEventListener("keydown", (event) => {
      markHandled(event);
    });
    const map = new ActionMap<Action>([CONFIRM]);
    map.attach(window);
    const event = keyDown("Enter");
    widget.dispatchEvent(event);
    expect(event.defaultPrevented).toBe(false);
    expect(map.held("confirm")).toBe(false);
    map.detach();
  });
});
