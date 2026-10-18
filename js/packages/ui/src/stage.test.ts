// @vitest-environment happy-dom
// Tests for the stage: the letterbox maths, the device-pixel sizes at several pixel ratios, the
// exact pointer conversion, the bars that take no clicks, resizing, and clean-up.

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { computeStageLayout, createStage, type Stage } from "./stage";
import { installFakeGeometry, placeElement, setWindowSize } from "./test-support";

let host: HTMLElement;

beforeEach(() => {
  installFakeGeometry();
  vi.stubGlobal("devicePixelRatio", 1);
  host = document.createElement("div");
  document.body.append(host);
});

afterEach(() => {
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
  document.body.innerHTML = "";
});

/** Makes a stage in a window of the given size, at the given pixel ratio. */
function stageIn(width: number, height: number, devicePixelRatio = 1, native?: boolean): Stage {
  setWindowSize(width, height);
  vi.stubGlobal("devicePixelRatio", devicePixelRatio);
  return createStage(host, native === undefined ? {} : { native });
}

/** Changes the window and dispatches a resize, the way a browser does. */
function resizeWindow(width: number, height: number, devicePixelRatio = 1): void {
  setWindowSize(width, height);
  vi.stubGlobal("devicePixelRatio", devicePixelRatio);
  window.dispatchEvent(new Event("resize"));
}

describe("letterbox maths", () => {
  it("puts bars on the left and right of a wide window", () => {
    // 2560x900: the height limits the scale to 900/1080, so the surface is 1600 wide.
    const stage = stageIn(2560, 900);
    expect(stage.scale).toBeCloseTo(900 / 1080, 10);
    expect(stage.layout.cssWidth).toBeCloseTo(1600, 6);
    expect(stage.layout.cssHeight).toBeCloseTo(900, 6);
    expect(stage.layout.left).toBeCloseTo(480, 6);
    expect(stage.layout.top).toBeCloseTo(0, 6);
  });

  it("puts bars above and below a tall window", () => {
    // 800x1000: the width limits the scale to 800/1920, so the surface is 450 tall.
    const stage = stageIn(800, 1000);
    expect(stage.layout.cssWidth).toBeCloseTo(800, 6);
    expect(stage.layout.cssHeight).toBeCloseTo(450, 6);
    expect(stage.layout.left).toBeCloseTo(0, 6);
    expect(stage.layout.top).toBeCloseTo(275, 6);
  });

  it("has no bars in an exact 16:9 window", () => {
    const stage = stageIn(1920, 1080);
    expect(stage.scale).toBe(1);
    expect(stage.layout.left).toBe(0);
    expect(stage.layout.top).toBe(0);
  });

  it("keeps the logical aspect ratio in every window it is given", () => {
    for (const [width, height] of [
      [1280, 720],
      [1000, 1000],
      [3440, 1440],
      [640, 900],
    ] as const) {
      const stage = stageIn(width, height);
      const { cssWidth, cssHeight } = stage.layout;
      expect(cssWidth / cssHeight).toBeCloseTo(16 / 9, 9);
      expect(cssWidth).toBeLessThanOrEqual(width + 1e-9);
      expect(cssHeight).toBeLessThanOrEqual(height + 1e-9);
      // The presented area touches one of the window's edges: the bars take the rest.
      const touchesSide = Math.abs(stage.layout.left) < 1e-9 || Math.abs(stage.layout.top) < 1e-9;
      expect(touchesSide).toBe(true);
      destroyAndReset(stage);
    }
  });

  it("uses the logical size given in the options", () => {
    setWindowSize(1280, 800);
    const stage = createStage(host, { width: 1280, height: 800 });
    expect(stage.width).toBe(1280);
    expect(stage.height).toBe(800);
    expect(stage.scale).toBe(1);
  });

  it("defaults the logical size to 1920 by 1080", () => {
    const stage = stageIn(1920, 1080);
    expect(stage.width).toBe(1920);
    expect(stage.height).toBe(1080);
  });
});

describe("device pixels and k", () => {
  it("draws at the window's pixels at a pixel ratio of 1", () => {
    // 1280x720 is two thirds of 1920x1080, so the native backing store is 1280x720.
    const stage = stageIn(1280, 720, 1);
    expect(stage.pixelWidth).toBe(1280);
    expect(stage.pixelHeight).toBe(720);
    expect(stage.k).toBeCloseTo(1280 / 1920, 10);
  });

  it("draws at 1920x1080 for a window of 1280x720 at a pixel ratio of 1.5", () => {
    const stage = stageIn(1280, 720, 1.5);
    expect(stage.pixelWidth).toBe(1920);
    expect(stage.pixelHeight).toBe(1080);
    expect(stage.k).toBeCloseTo(1, 10);
  });

  it("draws at 2560x1440 for a window of 1280x720 at a pixel ratio of 2", () => {
    const stage = stageIn(1280, 720, 2);
    expect(stage.pixelWidth).toBe(2560);
    expect(stage.pixelHeight).toBe(1440);
    expect(stage.k).toBeCloseTo(2560 / 1920, 10);
  });

  it("gives 3840x2160 for a 2560x1440 window at a pixel ratio of 1.5", () => {
    const stage = stageIn(2560, 1440, 1.5);
    expect(stage.pixelWidth).toBe(3840);
    expect(stage.pixelHeight).toBe(2160);
    expect(stage.k).toBeCloseTo(2, 10);
  });

  it("keeps the logical size as the backing store when native drawing is off", () => {
    const stage = stageIn(1280, 720, 2, false);
    expect(stage.native).toBe(false);
    expect(stage.pixelWidth).toBe(1920);
    expect(stage.pixelHeight).toBe(1080);
    expect(stage.k).toBe(1);
  });

  it("maps logical pixels exactly onto the backing store", () => {
    const stage = stageIn(1280, 720, 2);
    // The logical right and bottom edges land on the edges of the backing store.
    expect(1920 * stage.k).toBeCloseTo(stage.pixelWidth, 9);
    expect(1080 * stage.k).toBeCloseTo(stage.pixelHeight, 0);
  });

  it("is a pure function of its inputs", () => {
    const layout = computeStageLayout({ width: 1920, height: 1080 }, { width: 1920, height: 1080 }, 2, true);
    expect(layout.scale).toBe(1);
    expect(layout.pixelWidth).toBe(3840);
    expect(layout.pixelHeight).toBe(2160);
    expect(layout.k).toBe(2);
  });

  it("gives finite numbers for a host with no size", () => {
    const layout = computeStageLayout({ width: Number.NaN, height: -5 }, { width: 1920, height: 1080 }, 1, true);
    expect(layout.scale).toBe(0);
    expect(Number.isFinite(layout.pixelWidth)).toBe(true);
    expect(layout.pixelWidth).toBeGreaterThanOrEqual(1);
  });
});

describe("pointer conversion", () => {
  it("maps the centre of a wide window to the centre of the surface", () => {
    const stage = stageIn(2560, 900);
    const point = stage.toLogical(1280, 450);
    expect(point?.x).toBeCloseTo(960, 6);
    expect(point?.y).toBeCloseTo(540, 6);
  });

  it("round-trips logical points through client pixels in wide and tall windows", () => {
    const points = [
      [0, 0],
      [1, 1],
      [960, 540],
      [1919.5, 1079.5],
      [123.25, 987.75],
    ] as const;
    for (const [width, height] of [
      [2560, 900],
      [800, 1000],
      [1280, 720],
    ] as const) {
      const stage = stageIn(width, height, 1.25);
      for (const [x, y] of points) {
        const client = stage.toClient(x, y);
        const back = stage.toLogical(client.x, client.y);
        expect(back?.x).toBeCloseTo(x, 6);
        expect(back?.y).toBeCloseTo(y, 6);
      }
      destroyAndReset(stage);
    }
  });

  it("returns null on the letterbox bars", () => {
    // Wide window: the bars are the left and right strips, 480 pixels each.
    const wide = stageIn(2560, 900);
    expect(wide.toLogical(100, 450)).toBeNull();
    expect(wide.toLogical(2400, 450)).toBeNull();
    destroyAndReset(wide);

    // Tall window: the bars are the top and bottom strips, 275 pixels each.
    const tall = stageIn(800, 1000);
    expect(tall.toLogical(400, 100)).toBeNull();
    expect(tall.toLogical(400, 900)).toBeNull();
    expect(tall.toLogical(400, 500)).not.toBeNull();
  });

  it("returns null just outside the right and bottom edges of the surface", () => {
    const stage = stageIn(1920, 1080);
    expect(stage.toLogical(1920, 540)).toBeNull();
    expect(stage.toLogical(960, 1080)).toBeNull();
    expect(stage.toLogical(1919.9, 1079.9)).not.toBeNull();
  });

  it("returns null when the window has no size", () => {
    const stage = stageIn(0, 0);
    expect(stage.toLogical(0, 0)).toBeNull();
  });

  it("gives the box of an element in logical pixels", () => {
    // Wide window at a scale of 900/1080. The surface starts at client x 480.
    const stage = stageIn(2560, 900);
    const scale = 900 / 1080;
    const anchor = document.createElement("button");
    host.querySelector(".vu-root")?.append(anchor);
    placeElement(anchor, {
      left: 480 + 192 * scale,
      top: 50 * scale,
      width: 96 * scale,
      height: 48 * scale,
    });
    const box = stage.rectOf(anchor);
    expect(box.left).toBeCloseTo(192, 6);
    expect(box.top).toBeCloseTo(50, 6);
    expect(box.width).toBeCloseTo(96, 6);
    expect(box.height).toBeCloseTo(48, 6);
  });
});

describe("the presented root", () => {
  it("places and scales the root with inline styles, so the browser draws it in the centre", () => {
    const stage = stageIn(2560, 900);
    const frame = stage.root.getBoundingClientRect();
    expect(frame.left).toBeCloseTo(480, 6);
    expect(frame.top).toBeCloseTo(0, 6);
    expect(frame.width).toBeCloseTo(1600, 6);
    expect(frame.height).toBeCloseTo(900, 6);
    expect(stage.root.style.width).toBe("1920px");
    expect(stage.root.style.height).toBe("1080px");
  });

  it("puts the root inside a viewport that fills the host", () => {
    stageIn(1280, 720);
    const viewport = host.querySelector(".vu-viewport");
    expect(viewport).not.toBeNull();
    expect(viewport?.querySelector(".vu-root")).not.toBeNull();
  });

  it("keeps game content inside the root, in logical positions", () => {
    const stage = stageIn(1920, 1080);
    const child = document.createElement("div");
    stage.root.append(child);
    expect(stage.root.contains(child)).toBe(true);
  });
});

describe("bars do not take pointer events", () => {
  it("stops a click on the bars at the viewport", () => {
    stageIn(2560, 900);
    const viewport = host.querySelector(".vu-viewport");
    const game = vi.fn();
    window.addEventListener("pointerdown", game);
    try {
      viewport?.dispatchEvent(new PointerEvent("pointerdown", { bubbles: true, clientX: 10, clientY: 10 }));
      expect(game).not.toHaveBeenCalled();
    } finally {
      window.removeEventListener("pointerdown", game);
    }
  });

  it("lets a click on the root reach the window", () => {
    const stage = stageIn(2560, 900);
    const target = document.createElement("div");
    stage.root.append(target);
    const game = vi.fn();
    window.addEventListener("pointerdown", game);
    try {
      target.dispatchEvent(new PointerEvent("pointerdown", { bubbles: true, clientX: 900, clientY: 300 }));
      expect(game).toHaveBeenCalledTimes(1);
    } finally {
      window.removeEventListener("pointerdown", game);
    }
  });
});

describe("resizing", () => {
  it("tells the listeners when the window changes size", () => {
    const stage = stageIn(2560, 900);
    const listener = vi.fn();
    stage.onResize(listener);
    resizeWindow(1920, 1080);
    expect(listener).toHaveBeenCalledTimes(1);
    expect(listener.mock.calls[0]?.[0]).toMatchObject({ scale: 1, left: 0, top: 0 });
  });

  it("does not tell the listeners when nothing changed", () => {
    const stage = stageIn(1280, 720);
    const listener = vi.fn();
    stage.onResize(listener);
    resizeWindow(1280, 720);
    expect(listener).not.toHaveBeenCalled();
  });

  it("tells the listeners when the device pixel ratio changes", () => {
    const stage = stageIn(1280, 720, 1);
    const listener = vi.fn();
    stage.onResize(listener);
    resizeWindow(1280, 720, 2);
    expect(listener).toHaveBeenCalledTimes(1);
    expect(stage.pixelWidth).toBe(2560);
    expect(stage.layout.devicePixelRatio).toBe(2);
  });

  it("moves the root with the new layout", () => {
    const stage = stageIn(2560, 900);
    resizeWindow(1920, 1080);
    expect(stage.root.style.left).toBe("0px");
    expect(stage.root.style.transform).toBe("scale(1)");
  });

  it("stops calling a listener once it is removed", () => {
    const stage = stageIn(1280, 720);
    const listener = vi.fn();
    const unsubscribe = stage.onResize(listener);
    unsubscribe();
    resizeWindow(1920, 1080);
    expect(listener).not.toHaveBeenCalled();
  });

  it("observes the host with ResizeObserver, and disconnects it on destroy", () => {
    const observed: Element[] = [];
    let notify: (() => void) | undefined;
    let disconnected = false;
    class FakeObserver {
      constructor(callback: () => void) {
        notify = callback;
      }
      observe(target: Element): void {
        observed.push(target);
      }
      disconnect(): void {
        disconnected = true;
      }
    }
    vi.stubGlobal("ResizeObserver", FakeObserver);
    const stage = stageIn(1280, 720);
    expect(observed).toEqual([host]);
    const listener = vi.fn();
    stage.onResize(listener);
    setWindowSize(1920, 1080);
    notify?.();
    expect(listener).toHaveBeenCalledTimes(1);
    stage.destroy();
    expect(disconnected).toBe(true);
  });

  it("re-measures when the pixel ratio media query reports a change", () => {
    let changed: (() => void) | undefined;
    vi.stubGlobal("matchMedia", (query: string) => ({
      matches: false,
      media: query,
      addEventListener: (_type: string, listener: () => void) => {
        changed = listener;
      },
      removeEventListener: () => undefined,
    }));
    const stage = stageIn(1280, 720, 1);
    expect(stage.layout.devicePixelRatio).toBe(1);
    const listener = vi.fn();
    stage.onResize(listener);
    vi.stubGlobal("devicePixelRatio", 2);
    changed?.();
    expect(listener).toHaveBeenCalledTimes(1);
    expect(stage.pixelWidth).toBe(2560);
  });

  it("switches between native and logical drawing, and tells the listeners", () => {
    const stage = stageIn(1280, 720, 2);
    const listener = vi.fn();
    stage.onResize(listener);
    stage.setNative(false);
    expect(stage.native).toBe(false);
    expect(stage.pixelWidth).toBe(1920);
    expect(stage.k).toBe(1);
    expect(listener).toHaveBeenCalledTimes(1);
  });
});

describe("the host and clean-up", () => {
  it("makes an unpositioned host a positioning context, and restores it on destroy", () => {
    const stage = stageIn(1280, 720);
    expect(host.style.position).toBe("relative");
    stage.destroy();
    expect(host.style.position).toBe("");
    expect(host.querySelector(".vu-viewport")).toBeNull();
  });

  it("leaves a host that is already positioned alone", () => {
    host.style.position = "absolute";
    const stage = stageIn(1280, 720);
    expect(host.style.position).toBe("absolute");
    stage.destroy();
    expect(host.style.position).toBe("absolute");
  });

  it("stops listening after destroy", () => {
    const stage = stageIn(1280, 720);
    const listener = vi.fn();
    stage.onResize(listener);
    stage.destroy();
    resizeWindow(1920, 1080);
    stage.refresh();
    expect(listener).not.toHaveBeenCalled();
  });

  it("rejects a logical size that is not a positive number", () => {
    expect(() => createStage(host, { width: 0 })).toThrow(RangeError);
    expect(() => createStage(host, { height: Number.NaN })).toThrow(RangeError);
    expect(() => createStage(host, { width: -1920 })).toThrow(RangeError);
  });
});

/** Destroys a stage and makes a clean host for the next one. */
function destroyAndReset(stage: Stage): void {
  stage.destroy();
  document.body.innerHTML = "";
  host = document.createElement("div");
  document.body.append(host);
}
