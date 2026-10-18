// @vitest-environment happy-dom
// Tests for the tooltip: where it goes relative to its anchor, that it stays inside the stage,
// that it follows the stage when the window changes size, and that it describes its anchor.

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { createStage, type Stage } from "../stage";
import { installFakeGeometry, placeElement, setOffsetSize, setWindowSize } from "../test-support";
import { createTooltip } from "./tooltip";

let host: HTMLElement;
let stage: Stage;

beforeEach(() => {
  installFakeGeometry();
  vi.stubGlobal("devicePixelRatio", 1);
  setWindowSize(1920, 1080);
  host = document.createElement("div");
  document.body.append(host);
  stage = createStage(host);
});

afterEach(() => {
  stage.destroy();
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
  document.body.innerHTML = "";
});

/** An anchor button in the stage, placed at a logical box (the window is at scale 1). */
function anchorAt(left: number, top: number, width: number, height: number): HTMLElement {
  const anchor = document.createElement("button");
  stage.root.append(anchor);
  placeElement(anchor, { left, top, width, height });
  return anchor;
}

/** A tooltip in the stage, with a 200 by 50 box. */
function tooltipInStage(placement?: "above" | "below"): ReturnType<typeof createTooltip> {
  const tip = createTooltip({ stage });
  stage.root.append(tip.el);
  setOffsetSize(tip.el, 200, 50);
  if (placement !== undefined) tip.update({ placement });
  return tip;
}

describe("tooltip placement", () => {
  it("sits above its anchor, centred, when there is room", () => {
    const tip = tooltipInStage();
    const anchor = anchorAt(900, 500, 120, 40);
    tip.update({ anchor, text: "Guardar" });
    expect(tip.el.hidden).toBe(false);
    expect(tip.el.textContent).toBe("Guardar");
    expect(tip.el.style.left).toBe("860px");
    expect(tip.el.style.top).toBe("438px");
  });

  it("moves below its anchor when there is no room above", () => {
    const tip = tooltipInStage();
    tip.update({ anchor: anchorAt(900, 10, 120, 40), text: "Abajo" });
    expect(tip.el.style.top).toBe("62px");
  });

  it("moves above its anchor when it is placed below and there is no room under it", () => {
    const tip = tooltipInStage("below");
    tip.update({ anchor: anchorAt(900, 1050, 120, 20), text: "Arriba" });
    expect(tip.el.style.top).toBe("988px");
  });

  it("stays inside the stage at the right and left edges", () => {
    const tip = tooltipInStage();
    tip.update({ anchor: anchorAt(1900, 500, 20, 40), text: "Derecha" });
    expect(tip.el.style.left).toBe("1720px");
    tip.update({ anchor: anchorAt(-50, 500, 20, 40), text: "Izquierda" });
    expect(tip.el.style.left).toBe("0px");
  });

  it("is hidden without an anchor, and the anchor stops describing itself", () => {
    const tip = tooltipInStage();
    const anchor = anchorAt(900, 500, 120, 40);
    tip.update({ anchor, text: "Guardar" });
    expect(anchor.getAttribute("aria-describedby")).toBe(tip.el.id);
    expect(tip.el.getAttribute("role")).toBe("tooltip");
    tip.update({ anchor: null });
    expect(tip.el.hidden).toBe(true);
    expect(anchor.hasAttribute("aria-describedby")).toBe(false);
  });

  it("follows the stage when the window changes size", () => {
    const tip = tooltipInStage();
    const anchor = anchorAt(900, 500, 120, 40);
    tip.update({ anchor, text: "Guardar" });
    expect(tip.el.style.left).toBe("860px");

    // At half size the same logical box is at half the client position.
    setWindowSize(960, 540);
    placeElement(anchor, { left: 450, top: 250, width: 60, height: 20 });
    window.dispatchEvent(new Event("resize"));
    expect(tip.el.style.left).toBe("860px");
    expect(tip.el.style.top).toBe("438px");
  });

  it("stops following the stage after destroy", () => {
    const tip = tooltipInStage();
    tip.update({ anchor: anchorAt(900, 500, 120, 40), text: "Guardar" });
    tip.destroy();
    expect(tip.el.isConnected).toBe(false);
    expect(() => window.dispatchEvent(new Event("resize"))).not.toThrow();
  });
});
