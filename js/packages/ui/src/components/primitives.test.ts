// @vitest-environment happy-dom
// Tests for the simple components: plate, button, keycap, chip and meter. Each one checks that
// update() changes its text and state, that the props it is given are shown, and that hover does
// not change the element's inline geometry.

import { afterEach, describe, expect, it, vi } from "vitest";
import { createButton } from "./button";
import { createChip } from "./chip";
import { createKeycap, keyLabel } from "./keycap";
import { createMeter } from "./meter";
import { createPlate } from "./plate";

afterEach(() => {
  document.body.innerHTML = "";
});

/** Sends the pointer events that a hover produces, and reports whether the inline style moved. */
function hoverAndCheckStyle(element: HTMLElement): void {
  const before = element.getAttribute("style");
  element.dispatchEvent(new PointerEvent("pointerenter", { bubbles: true }));
  element.dispatchEvent(new PointerEvent("pointerover", { bubbles: true }));
  element.dispatchEvent(new MouseEvent("mouseover", { bubbles: true }));
  expect(element.getAttribute("style")).toBe(before);
}

describe("plate", () => {
  it("shows its title on a tab, and hides the tab when there is no title", () => {
    const plate = createPlate();
    expect(plate.el.querySelector(".vu-plate-title")?.hasAttribute("hidden")).toBe(true);
    plate.update({ title: "Pausa" });
    const tab = plate.el.querySelector<HTMLElement>(".vu-plate-title");
    expect(tab?.textContent).toBe("Pausa");
    expect(tab?.hidden).toBe(false);
    expect(plate.el.getAttribute("aria-label")).toBe("Pausa");
    plate.update({ title: null });
    expect(tab?.hidden).toBe(true);
    expect(plate.el.hasAttribute("aria-label")).toBe(false);
  });

  it("keeps game content in its body", () => {
    const plate = createPlate({ title: "Ajustes" });
    const content = document.createElement("p");
    plate.body.append(content);
    expect(plate.el.contains(content)).toBe(true);
  });

  it("does not change its geometry on hover", () => {
    const plate = createPlate({ title: "Ajustes" });
    document.body.append(plate.el);
    hoverAndCheckStyle(plate.el);
  });

  it("removes its element on destroy", () => {
    const plate = createPlate();
    document.body.append(plate.el);
    plate.destroy();
    expect(plate.el.isConnected).toBe(false);
  });
});

describe("button", () => {
  it("shows its label as text", () => {
    const button = createButton({ label: "Guardar" });
    expect(button.el.tagName).toBe("BUTTON");
    expect(button.el.getAttribute("type")).toBe("button");
    expect(button.el.textContent).toBe("Guardar");
  });

  it("calls onPress on a click", () => {
    const onPress = vi.fn();
    const button = createButton({ label: "Guardar", onPress });
    document.body.append(button.el);
    button.el.click();
    expect(onPress).toHaveBeenCalledTimes(1);
  });

  it("does not call onPress while disabled, and reports the disabled state", () => {
    const onPress = vi.fn();
    const button = createButton({ label: "Cargar", disabled: true, onPress });
    document.body.append(button.el);
    expect((button.el as HTMLButtonElement).disabled).toBe(true);
    button.el.click();
    expect(onPress).not.toHaveBeenCalled();
    button.update({ disabled: false });
    expect((button.el as HTMLButtonElement).disabled).toBe(false);
    button.el.click();
    expect(onPress).toHaveBeenCalledTimes(1);
  });

  it("is a toggle with aria-pressed when pressed is set, and a plain button when it is null", () => {
    const button = createButton({ label: "Pausa", pressed: false });
    expect(button.el.getAttribute("aria-pressed")).toBe("false");
    button.update({ pressed: true });
    expect(button.el.getAttribute("aria-pressed")).toBe("true");
    button.update({ pressed: null });
    expect(button.el.hasAttribute("aria-pressed")).toBe(false);
  });

  it("keeps the values that update leaves out", () => {
    const button = createButton({ label: "Pausa", pressed: true });
    button.update({ label: "Seguir" });
    expect(button.el.textContent).toBe("Seguir");
    expect(button.el.getAttribute("aria-pressed")).toBe("true");
  });

  it("shows a key hint after the label, and removes it when the hint is cleared", () => {
    const button = createButton({ label: "Usar", hotkey: "E" });
    const hint = button.el.querySelector("kbd");
    expect(hint?.textContent).toBe("E");
    button.update({ hotkey: "Space" });
    expect(button.el.querySelector("kbd")?.textContent).toBe("Space");
    button.update({ hotkey: null });
    expect(button.el.querySelector("kbd")).toBeNull();
    expect(button.el.textContent).toBe("Usar");
  });

  it("does not change its geometry on hover or press", () => {
    const button = createButton({ label: "Guardar" });
    document.body.append(button.el);
    hoverAndCheckStyle(button.el);
    button.el.dispatchEvent(new PointerEvent("pointerdown", { bubbles: true }));
    expect(button.el.getAttribute("style")).toBeNull();
  });

  it("removes its element and its click listener on destroy", () => {
    const onPress = vi.fn();
    const button = createButton({ label: "Guardar", onPress });
    document.body.append(button.el);
    const element = button.el;
    button.destroy();
    expect(element.isConnected).toBe(false);
    element.click();
    expect(onPress).not.toHaveBeenCalled();
  });
});

describe("keycap", () => {
  it("gives keys their usual labels", () => {
    expect(keyLabel("KeyE")).toBe("E");
    expect(keyLabel("Digit7")).toBe("7");
    expect(keyLabel("Space")).toBe("Space");
    expect(keyLabel("ArrowUp")).toBe("↑");
    expect(keyLabel("Escape")).toBe("Esc");
    expect(keyLabel("E")).toBe("E");
    expect(keyLabel("F5")).toBe("F5");
  });

  it("does not turn prototype names into labels", () => {
    expect(keyLabel("constructor")).toBe("constructor");
    expect(keyLabel("toString")).toBe("toString");
  });

  it("shows the label of a key as a kbd element", () => {
    const keycap = createKeycap({ key: "KeyE" });
    expect(keycap.el.tagName).toBe("KBD");
    expect(keycap.el.textContent).toBe("E");
  });

  it("draws the mouse with the lit part for a mouse button, and names it", () => {
    const keycap = createKeycap({ key: "MouseLeft" });
    expect(keycap.el.getAttribute("aria-label")).toBe("Left mouse button");
    const lit = keycap.el.querySelectorAll(".is-lit");
    expect(lit).toHaveLength(1);
    expect(lit[0]?.getAttribute("x")).toBe("3");
    keycap.update({ key: "MouseRight" });
    expect(keycap.el.getAttribute("aria-label")).toBe("Right mouse button");
    expect(keycap.el.querySelector(".is-lit")?.getAttribute("x")).toBe("12.5");
  });

  it("changes back to text when the key is text again", () => {
    const keycap = createKeycap({ key: "MouseWheel" });
    keycap.update({ key: "Space" });
    expect(keycap.el.querySelector("svg")).toBeNull();
    expect(keycap.el.textContent).toBe("Space");
    expect(keycap.el.hasAttribute("aria-label")).toBe(false);
  });
});

describe("chip", () => {
  it("shows its label and an optional count badge", () => {
    const chip = createChip({ label: "Pociones", count: 3 });
    expect(chip.el.querySelector(".vu-chip-label")?.textContent).toBe("Pociones");
    const badge = chip.el.querySelector<HTMLElement>(".vu-chip-count");
    expect(badge?.textContent).toBe("3");
    expect(badge?.hidden).toBe(false);
    chip.update({ count: null });
    expect(badge?.hidden).toBe(true);
    chip.update({ count: 0, label: "Llaves" });
    expect(badge?.textContent).toBe("0");
    expect(chip.el.querySelector(".vu-chip-label")?.textContent).toBe("Llaves");
  });

  it("does not change its geometry on hover", () => {
    const chip = createChip({ label: "Llaves" });
    document.body.append(chip.el);
    hoverAndCheckStyle(chip.el);
  });
});

describe("meter", () => {
  it("fills to the value as a percentage, with role meter and its values", () => {
    const meter = createMeter({ label: "Salud", value: 70, max: 100 });
    expect(meter.el.getAttribute("role")).toBe("meter");
    expect(meter.el.getAttribute("aria-label")).toBe("Salud");
    expect(meter.el.getAttribute("aria-valuenow")).toBe("70");
    expect(meter.el.getAttribute("aria-valuemax")).toBe("100");
    expect(meter.el.querySelector<HTMLElement>(".vu-meter-fill")?.style.width).toBe("70%");
  });

  it("follows update and clamps values outside the range", () => {
    const meter = createMeter({ label: "Salud", value: 70 });
    meter.update({ value: 30 });
    expect(meter.el.querySelector<HTMLElement>(".vu-meter-fill")?.style.width).toBe("30%");
    meter.update({ value: 150 });
    expect(meter.el.getAttribute("aria-valuenow")).toBe("100");
    expect(meter.el.querySelector<HTMLElement>(".vu-meter-fill")?.style.width).toBe("100%");
    meter.update({ value: -4 });
    expect(meter.el.getAttribute("aria-valuenow")).toBe("0");
  });

  it("lights up the cells up to the value when it has segments", () => {
    const meter = createMeter({ label: "Carga", value: 5, max: 10, segments: 10 });
    expect(meter.el.querySelectorAll(".vu-meter-cell")).toHaveLength(10);
    expect(meter.el.querySelectorAll(".vu-meter-cell.is-lit")).toHaveLength(5);
    meter.update({ value: 10 });
    expect(meter.el.querySelectorAll(".vu-meter-cell.is-lit")).toHaveLength(10);
    meter.update({ segments: null });
    expect(meter.el.querySelector(".vu-meter-cell")).toBeNull();
    expect(meter.el.querySelector(".vu-meter-fill")).not.toBeNull();
  });

  it("sets the colour through data-tone", () => {
    const meter = createMeter({ label: "Oro", value: 1, tone: "gold" });
    expect(meter.el.getAttribute("data-tone")).toBe("gold");
  });

  it("rejects values it cannot show, and keeps its state when an update is refused", () => {
    expect(() => createMeter({ label: "x", value: Number.NaN })).toThrow(RangeError);
    expect(() => createMeter({ label: "x", value: 1, max: 0 })).toThrow(RangeError);
    expect(() => createMeter({ label: "x", value: 1, segments: 2.5 })).toThrow(RangeError);
    const meter = createMeter({ label: "Salud", value: 40, max: 80 });
    expect(() => meter.update({ max: -1 })).toThrow(RangeError);
    expect(meter.el.getAttribute("aria-valuemax")).toBe("80");
    expect(meter.el.getAttribute("aria-valuenow")).toBe("40");
  });

  it("does not change its geometry on hover", () => {
    const meter = createMeter({ label: "Salud", value: 70 });
    document.body.append(meter.el);
    hoverAndCheckStyle(meter.el);
  });
});
