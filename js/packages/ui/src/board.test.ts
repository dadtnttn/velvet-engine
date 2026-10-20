// @vitest-environment happy-dom
// Tests for the board view: the world transform and its round trip, the zoom around a point, the clamp of
// the pan to the world bounds, items and their hit testing, and the drag helper and the pan. The pointer
// is driven with synthetic events, as a browser sends them. The stage has the fake geometry of
// test-support.ts: a window of 1920 by 1080 gives a scale of 1, so client and logical pixels agree.

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  createBoardView,
  type BoardDrag,
  type BoardDragCallbacks,
  type BoardView,
  type BoardViewState,
  type WorldRect,
} from "./board";
import { createStage, type Stage } from "./stage";
import { installFakeGeometry, setWindowSize } from "./test-support";

/** The world of most tests. At zoom 1 it is wider and taller than the surface. */
const WORLD: WorldRect = { x: 0, y: 0, w: 3000, h: 2000 };

let host: HTMLElement;

beforeEach(() => {
  installFakeGeometry();
  vi.stubGlobal("devicePixelRatio", 1);
  host = document.createElement("div");
  document.body.append(host);
});

afterEach(() => {
  // A gesture that a test left open is ended, so that its window listeners do not reach the next test.
  window.dispatchEvent(pointerEvent("pointercancel", 0, 0));
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
  document.body.innerHTML = "";
});

/** A stage in a window of the given size. The logical surface is 1920 by 1080 whatever the window. */
function stageIn(windowWidth = 1920, windowHeight = 1080): Stage {
  setWindowSize(windowWidth, windowHeight);
  return createStage(host, { width: 1920, height: 1080 });
}

/** A board at zoom 1 with its pan at zero, so that a logical point is the same as the world point. */
function boardAtOrigin(stage: Stage, worldBounds: WorldRect = WORLD): BoardView {
  const board = createBoardView(stage, { worldBounds });
  board.pan = { x: 0, y: 0 };
  return board;
}

/** A pointer event of the primary mouse pointer, at a client position. */
function pointerEvent(type: string, x: number, y: number): PointerEvent {
  return new PointerEvent(type, {
    bubbles: true,
    cancelable: true,
    clientX: x,
    clientY: y,
    button: 0,
    pointerId: 1,
    isPrimary: true,
  });
}

/** A press on a target. */
function press(target: EventTarget, x: number, y: number): void {
  target.dispatchEvent(pointerEvent("pointerdown", x, y));
}

/** The primary pointer moves, on the window. */
function moveTo(x: number, y: number): void {
  window.dispatchEvent(pointerEvent("pointermove", x, y));
}

/** The primary pointer is released, on the window. */
function release(x: number, y: number): void {
  window.dispatchEvent(pointerEvent("pointerup", x, y));
}

/** A wheel turn over a target. It returns the event, so that a test can read whether it was prevented. */
function wheel(target: EventTarget, x: number, y: number, deltaY: number): WheelEvent {
  const event = new WheelEvent("wheel", { bubbles: true, cancelable: true, deltaY });
  // happy-dom's WheelEvent is not a MouseEvent, so it has no client position. A browser's wheel event has one.
  Object.defineProperty(event, "clientX", { value: x });
  Object.defineProperty(event, "clientY", { value: y });
  target.dispatchEvent(event);
  return event;
}

/** A new element, for an item. */
function element(): HTMLElement {
  return document.createElement("div");
}

/** Callbacks that record each call as a pair: the callback's name, and the drag it was given. */
function recorder(): { readonly callbacks: BoardDragCallbacks; readonly calls: Array<[string, BoardDrag]> } {
  const calls: Array<[string, BoardDrag]> = [];
  return {
    calls,
    callbacks: {
      onStart: (drag) => {
        calls.push(["start", drag]);
      },
      onMove: (drag) => {
        calls.push(["move", drag]);
      },
      onDrop: (drag) => {
        calls.push(["drop", drag]);
      },
      onCancel: (drag) => {
        calls.push(["cancel", drag]);
      },
    },
  };
}

describe("the world transform", () => {
  it("converts world points to logical pixels and back, at several zooms", () => {
    const board = createBoardView(stageIn(), { worldBounds: WORLD });
    for (const zoom of [0.5, 1, 2.5]) {
      board.zoom = zoom;
      for (const [x, y] of [
        [0, 0],
        [12.5, -3],
        [1500, 900],
        [2999, 1999],
      ] as const) {
        const logical = board.worldToLogical(x, y);
        const back = board.logicalToWorld(logical.x, logical.y);
        expect(back.x).toBeCloseTo(x, 9);
        expect(back.y).toBeCloseTo(y, 9);
      }
    }
  });

  it("draws a world point at its zoomed position, moved by the pan", () => {
    const board = createBoardView(stageIn(), { worldBounds: WORLD });
    board.zoom = 2;
    board.pan = { x: -100, y: -50 };
    expect(board.worldToLogical(10, 20)).toEqual({ x: 10 * 2 - 100, y: 20 * 2 - 50 });
  });

  it("writes the pan and the zoom into the layer as one transform, and starts with the world centred", () => {
    const board = createBoardView(stageIn(), { worldBounds: WORLD, initialZoom: 2 });
    // Centred: x = 960 - 1500 * 2 and y = 540 - 1000 * 2.
    expect(board.pan).toEqual({ x: -2040, y: -1460 });
    expect(board.el.style.transform).toBe("translate(-2040px, -1460px) scale(2)");
  });

  it("writes the pan at whole pixels of the screen with snapToPixels, and keeps the exact pan in the view", () => {
    const board = createBoardView(stageIn(), { worldBounds: WORLD, snapToPixels: true });
    board.pan = { x: -100.3, y: -50.6 };
    expect(board.pan).toEqual({ x: -100.3, y: -50.6 });
    expect(board.el.style.transform).toBe("translate(-100px, -51px) scale(1)");
  });

  it("keeps the world point under the pointer when it zooms around that point", () => {
    const board = createBoardView(stageIn(), { worldBounds: WORLD });
    const point = { x: 700, y: 300 };
    const under = board.logicalToWorld(point.x, point.y);
    board.zoomAt(2.5, point);
    expect(board.zoom).toBe(2.5);
    const now = board.worldToLogical(under.x, under.y);
    expect(now.x).toBeCloseTo(point.x, 9);
    expect(now.y).toBeCloseTo(point.y, 9);
  });

  it("zooms around the pointer on a wheel turn, and prevents the page from scrolling", () => {
    const stage = stageIn();
    const board = createBoardView(stage, { worldBounds: WORLD });
    const point = { x: 700, y: 300 };
    const under = board.logicalToWorld(point.x, point.y);
    const event = wheel(stage.root, point.x, point.y, -100);
    expect(event.defaultPrevented).toBe(true);
    expect(board.zoom).toBeCloseTo(Math.exp(100 * 0.0015), 9);
    const now = board.worldToLogical(under.x, under.y);
    expect(now.x).toBeCloseTo(point.x, 6);
    expect(now.y).toBeCloseTo(point.y, 6);
  });

  it("ignores a wheel turn that is not over the board", () => {
    const stage = stageIn();
    const board = createBoardView(stage, { worldBounds: WORLD });
    const hud = element();
    stage.root.append(hud);
    const event = wheel(hud, 100, 100, -100);
    expect(event.defaultPrevented).toBe(false);
    expect(board.zoom).toBe(1);
  });

  it("keeps the zoom inside its range, even for a turn too large to compute", () => {
    const stage = stageIn();
    const board = createBoardView(stage, { worldBounds: WORLD, minZoom: 0.5, maxZoom: 3 });
    board.zoom = 10;
    expect(board.zoom).toBe(3);
    board.zoom = 0.01;
    expect(board.zoom).toBe(0.5);
    // exp(1500) overflows, and the clamp still gives the largest zoom, then the smallest.
    wheel(stage.root, 960, 540, -1e6);
    expect(board.zoom).toBe(3);
    wheel(stage.root, 960, 540, 1e6);
    expect(board.zoom).toBe(0.5);
  });

  it("rejects a zoom range, an initial zoom or world bounds that are not valid, and adds nothing", () => {
    const stage = stageIn();
    expect(() => createBoardView(stage, { worldBounds: WORLD, minZoom: 2, maxZoom: 1 })).toThrow(RangeError);
    expect(() =>
      createBoardView(stage, { worldBounds: WORLD, minZoom: 0.5, maxZoom: 2, initialZoom: 3 }),
    ).toThrow(RangeError);
    expect(() => createBoardView(stage, { worldBounds: { x: 0, y: 0, w: 0, h: 10 } })).toThrow(RangeError);
    expect(stage.root.children.length).toBe(0);
  });

  it("fits a rectangle in the middle of the surface, as large as the zoom range allows", () => {
    const board = createBoardView(stageIn(), { worldBounds: WORLD });
    board.fit({ x: 100, y: 100, w: 400, h: 300 });
    // The height limits the zoom: 1080 / 300 = 3.6, which is inside the range.
    expect(board.zoom).toBeCloseTo(3.6, 9);
    const centre = board.worldToLogical(300, 250);
    expect(centre.x).toBeCloseTo(960, 6);
    expect(centre.y).toBeCloseTo(540, 6);
    const topLeft = board.worldToLogical(100, 100);
    const bottomRight = board.worldToLogical(500, 400);
    expect(topLeft.x).toBeGreaterThanOrEqual(-1e-6);
    expect(topLeft.y).toBeGreaterThanOrEqual(-1e-6);
    expect(bottomRight.x).toBeLessThanOrEqual(1920 + 1e-6);
    expect(bottomRight.y).toBeLessThanOrEqual(1080 + 1e-6);

    board.fit({ x: 0, y: 0, w: 10, h: 10 });
    expect(board.zoom).toBe(4);
  });
});

describe("fit with options", () => {
  it("keeps the padding free inside the visible area, and stays centred", () => {
    const board = createBoardView(stageIn(), { worldBounds: WORLD });
    board.fit({ x: 1000, y: 900, w: 400, h: 300 }, { padding: 40 });
    // The padded height is 1080 - 2 * 40 = 1000, and the height limits the zoom: 1000 / 300.
    expect(board.zoom).toBeCloseTo(1000 / 300, 9);
    const topLeft = board.worldToLogical(1000, 900);
    const bottomRight = board.worldToLogical(1400, 1200);
    expect(topLeft.y).toBeCloseTo(40, 6);
    expect(bottomRight.y).toBeCloseTo(1040, 6);
    expect(topLeft.x).toBeCloseTo((1920 - 400 * (1000 / 300)) / 2, 6);
    expect(bottomRight.x).toBeCloseTo(1920 - topLeft.x, 6);
  });

  it("fits the same as before when no options are given", () => {
    const plain = createBoardView(stageIn(), { worldBounds: WORLD });
    plain.fit({ x: 1000, y: 900, w: 400, h: 300 });
    const empty = createBoardView(stageIn(), { worldBounds: WORLD });
    empty.fit({ x: 1000, y: 900, w: 400, h: 300 }, {});
    expect(empty.zoom).toBe(plain.zoom);
    expect(empty.pan).toEqual(plain.pan);
  });

  it("caps the zoom at maxZoom for this fit, and the board's own largest zoom still holds", () => {
    const board = createBoardView(stageIn(), { worldBounds: WORLD });
    board.fit({ x: 1000, y: 900, w: 10, h: 10 }, { maxZoom: 2 });
    expect(board.zoom).toBe(2);
    board.fit({ x: 1000, y: 900, w: 10, h: 10 }, { maxZoom: 9 });
    expect(board.zoom).toBe(4);
  });

  it("keeps at least minZoom for this fit, and shows the middle of a rectangle too large for it", () => {
    const board = createBoardView(stageIn(), { worldBounds: WORLD });
    board.fit({ x: 1000, y: 900, w: 1000, h: 800 }, { minZoom: 2 });
    // The rectangle would fit at about 1.35, below the minimum, so the zoom is 2 and the view centres on it.
    expect(board.zoom).toBe(2);
    const centre = board.worldToLogical(1500, 1300);
    expect(centre.x).toBeCloseTo(960, 6);
    expect(centre.y).toBeCloseTo(540, 6);
  });

  it("does not widen the board's zoom range", () => {
    const board = createBoardView(stageIn(), { worldBounds: WORLD, maxZoom: 2 });
    board.fit({ x: 1000, y: 900, w: 10, h: 10 }, { maxZoom: 8 });
    expect(board.zoom).toBe(2);
  });

  it("refuses a bad option, and a zoom range that does not overlap the board's, and leaves the view as it was", () => {
    const board = createBoardView(stageIn(), { worldBounds: WORLD });
    board.fit({ x: 1000, y: 900, w: 400, h: 300 });
    const before = { zoom: board.zoom, pan: board.pan };
    const rect = { x: 0, y: 0, w: 10, h: 10 };
    expect(() => board.fit(rect, { padding: -1 })).toThrow(RangeError);
    expect(() => board.fit(rect, { padding: Number.NaN })).toThrow(RangeError);
    expect(() => board.fit(rect, { maxZoom: 0 })).toThrow(RangeError);
    expect(() => board.fit(rect, { minZoom: Number.POSITIVE_INFINITY })).toThrow(RangeError);
    expect(() => board.fit(rect, { minZoom: 5 })).toThrow(RangeError);
    expect(() => board.fit(rect, { maxZoom: 0.1 })).toThrow(RangeError);
    expect(() => board.fit(rect, { padding: 540 })).toThrow(RangeError);
    expect(board.zoom).toBe(before.zoom);
    expect(board.pan).toEqual(before.pan);
  });
});

describe("the view stays on the world", () => {
  it("keeps a world larger than the surface covering it", () => {
    const board = createBoardView(stageIn(), { worldBounds: WORLD });
    board.panBy(5000, 5000);
    expect(board.pan).toEqual({ x: 0, y: 0 });
    board.panBy(-1e6, -1e6);
    expect(board.pan).toEqual({ x: 1920 - 3000, y: 1080 - 2000 });
  });

  it("clamps to the bounds of a world that starts away from the origin", () => {
    const board = createBoardView(stageIn(), { worldBounds: { x: 1000, y: 500, w: 3000, h: 2000 } });
    board.panBy(-1e6, -1e6);
    expect(board.pan).toEqual({ x: 1920 - 4000, y: 1080 - 2500 });
    board.panBy(1e6, 1e6);
    expect(board.pan).toEqual({ x: -1000, y: -500 });
  });

  it("centres a world smaller than the surface, and does not pan it", () => {
    const board = createBoardView(stageIn(), { worldBounds: { x: 0, y: 0, w: 800, h: 400 } });
    expect(board.pan).toEqual({ x: 560, y: 340 });
    board.panBy(300, -300);
    expect(board.pan).toEqual({ x: 560, y: 340 });
  });

  it("clamps the pan of a drag to the world bounds", () => {
    const stage = stageIn();
    const board = createBoardView(stage, { worldBounds: WORLD });
    board.pan = { x: 0, y: 0 };
    press(stage.root, 500, 500);
    moveTo(-5000, -5000);
    release(-5000, -5000);
    // The farthest the view can go: the world's far corner reaches the surface's far corner.
    expect(board.pan).toEqual({ x: 1920 - 3000, y: 1080 - 2000 });
  });
});

describe("items", () => {
  it("places an item by a transform in world units, and a move changes only the transform", () => {
    const board = createBoardView(stageIn(), { worldBounds: WORLD });
    const card = element();
    board.addItem(card, { x: 10, y: 20, w: 112, h: 150, z: 3 });
    expect(card.parentElement).toBe(board.el);
    expect(card.style.transform).toBe("translate(10px, 20px)");
    expect(card.style.width).toBe("112px");
    expect(card.style.height).toBe("150px");
    expect(card.style.zIndex).toBe("3");
    expect(card.style.position).toBe("absolute");

    const restOfStyle = (node: HTMLElement): string => node.style.cssText.replace(/transform:[^;]*;?\s*/, "");
    const before = restOfStyle(card);
    board.moveItem(card, { x: 300, y: 40 });
    expect(card.style.transform).toBe("translate(300px, 40px)");
    expect(restOfStyle(card)).toBe(before);
  });

  it("re-stacks an item with a move, and an item can be taken off and added again", () => {
    const board = createBoardView(stageIn(), { worldBounds: WORLD });
    const card = element();
    board.addItem(card, { x: 0, y: 0, w: 100, h: 100 });
    board.moveItem(card, { z: 9 });
    expect(card.style.zIndex).toBe("9");
    expect(card.style.transform).toBe("translate(0px, 0px)");

    board.removeItem(card);
    expect(card.parentElement).toBeNull();
    expect(board.itemAt(10, 10)).toBeNull();
    board.addItem(card, { x: 0, y: 0, w: 100, h: 100 });
    expect(board.itemAt(10, 10)).toBe(card);
  });

  it("refuses an element that is already an item, and boxes that are not valid", () => {
    const board = createBoardView(stageIn(), { worldBounds: WORLD });
    const card = element();
    board.addItem(card, { x: 0, y: 0, w: 100, h: 100 });
    expect(() => board.addItem(card, { x: 0, y: 0, w: 100, h: 100 })).toThrow(Error);
    expect(() => board.addItem(element(), { x: 0, y: 0, w: -1, h: 10 })).toThrow(RangeError);
    expect(() => board.addItem(element(), { x: 0, y: 0, w: 10, h: 10, z: 1.5 })).toThrow(RangeError);
    expect(() => board.moveItem(card, { x: Number.NaN })).toThrow(RangeError);
  });

  it("hits the topmost item: by z first, then by the later addition, and a right or bottom edge is outside", () => {
    const board = createBoardView(stageIn(), { worldBounds: WORLD });
    const a = element();
    const b = element();
    const c = element();
    board.addItem(a, { x: 0, y: 0, w: 100, h: 100 });
    board.addItem(b, { x: 50, y: 50, w: 100, h: 100 });
    board.addItem(c, { x: 0, y: 0, w: 100, h: 100, z: 1 });
    expect(board.itemAt(60, 60)).toBe(c);

    board.moveItem(c, { x: 1000 });
    expect(board.itemAt(60, 60)).toBe(b);
    expect(board.itemAt(20, 20)).toBe(a);
    expect(board.itemAt(50, 50)).toBe(b);
    expect(board.itemAt(100, 60)).toBe(b);
    expect(board.itemAt(150, 60)).toBeNull();
    expect(board.itemAt(-1, 0)).toBeNull();
  });

  it("leaves a stranger alone when it is taken off the board", () => {
    const board = createBoardView(stageIn(), { worldBounds: WORLD });
    const card = element();
    board.addItem(card, { x: 0, y: 0, w: 100, h: 100 });
    expect(() => board.removeItem(element())).not.toThrow();
    board.moveItem(element(), { x: 500 });
    board.removeItem(card);
    expect(board.itemAt(10, 10)).toBeNull();
  });
});

describe("dragging an item", () => {
  it("reports the start, the moves and the drop, with world points and the drop target", () => {
    const stage = stageIn();
    const board = boardAtOrigin(stage);
    const forest = element();
    const villager = element();
    board.addItem(forest, { x: 100, y: 100, w: 200, h: 200 });
    board.addItem(villager, { x: 400, y: 400, w: 112, h: 150, z: 1 });
    const { callbacks, calls } = recorder();
    board.enableDrag(villager, callbacks);

    press(villager, 456, 475);
    moveTo(470, 480);
    moveTo(200, 200);
    release(200, 200);

    expect(calls.map(([name]) => name)).toEqual(["start", "move", "drop"]);
    const [startName, start] = calls[0] ?? [];
    expect(startName).toBe("start");
    expect(start?.el).toBe(villager);
    expect(start?.world).toEqual({ x: 470, y: 480 });
    expect(start?.start).toEqual({ x: 456, y: 475 });
    expect(start?.origin).toEqual({ x: 400, y: 400 });
    expect(start?.target).toBeNull();

    const [, move] = calls[1] ?? [];
    expect(move?.world).toEqual({ x: 200, y: 200 });
    expect(move?.target).toBe(forest);

    const [, drop] = calls[2] ?? [];
    expect(drop?.world).toEqual({ x: 200, y: 200 });
    expect(drop?.target).toBe(forest);
    expect(board.itemAt(200, 200)).toBe(forest);
  });

  it("reports the pointer in world units at a zoom, and the place the item would move to", () => {
    const stage = stageIn();
    const board = boardAtOrigin(stage);
    board.zoom = 2;
    board.pan = { x: 0, y: 0 };
    const card = element();
    board.addItem(card, { x: 100, y: 100, w: 112, h: 150 });
    const { callbacks, calls } = recorder();
    board.enableDrag(card, callbacks);

    // The pointer at logical (210, 210) is world (105, 105); it moves to logical (410, 410), world (205, 205).
    press(card, 210, 210);
    moveTo(410, 410);
    release(410, 410);

    const drop = calls.find(([name]) => name === "drop")?.[1];
    if (drop === undefined) throw new Error("the drag did not drop");
    expect(drop.start).toEqual({ x: 105, y: 105 });
    expect(drop.world).toEqual({ x: 205, y: 205 });
    expect(drop.origin).toEqual({ x: 100, y: 100 });
    // Where the game would put the item: its origin, moved by the pointer's world distance.
    const wouldBe = {
      x: drop.origin.x + (drop.world.x - drop.start.x),
      y: drop.origin.y + (drop.world.y - drop.start.y),
    };
    expect(wouldBe).toEqual({ x: 200, y: 200 });
  });

  it("does not report a press that stays under the threshold, which is a click", () => {
    const stage = stageIn();
    const board = boardAtOrigin(stage);
    const card = element();
    board.addItem(card, { x: 50, y: 50, w: 100, h: 100 });
    const { callbacks, calls } = recorder();
    board.enableDrag(card, callbacks);
    press(card, 60, 60);
    moveTo(61, 61);
    release(61, 61);
    expect(calls).toEqual([]);
  });

  it("uses a threshold that the game gives", () => {
    const stage = stageIn();
    const board = boardAtOrigin(stage);
    const card = element();
    board.addItem(card, { x: 50, y: 50, w: 100, h: 100 });
    const { callbacks, calls } = recorder();
    board.enableDrag(card, callbacks, { threshold: 40 });
    press(card, 60, 60);
    moveTo(80, 60);
    expect(calls).toEqual([]);
    moveTo(120, 60);
    expect(calls.map(([name]) => name)).toEqual(["start"]);
    release(120, 60);
  });

  it("reports a cancelled drag with onCancel, and never a drop", () => {
    const stage = stageIn();
    const board = boardAtOrigin(stage);
    const card = element();
    board.addItem(card, { x: 50, y: 50, w: 100, h: 100 });
    const { callbacks, calls } = recorder();
    board.enableDrag(card, callbacks);
    press(card, 60, 60);
    moveTo(90, 60);
    window.dispatchEvent(pointerEvent("pointercancel", 90, 60));
    expect(calls.map(([name]) => name)).toEqual(["start", "cancel"]);
  });

  it("keeps following the pointer off the surface, clamped to its edge, and captures the pointer", () => {
    const stage = stageIn();
    const board = boardAtOrigin(stage);
    const card = element();
    board.addItem(card, { x: 50, y: 50, w: 100, h: 100 });
    const capture = vi.spyOn(card, "setPointerCapture");
    const { callbacks, calls } = recorder();
    board.enableDrag(card, callbacks);
    press(card, 60, 60);
    moveTo(90, 60);
    moveTo(-500, 300);
    release(-500, 300);
    expect(capture).toHaveBeenCalledWith(1);
    const [, moved] = calls[calls.length - 2] ?? [];
    expect(moved?.world).toEqual({ x: 0, y: 300 });
    expect(calls[calls.length - 1]?.[0]).toBe("drop");
  });

  it("ends a drag in progress with onCancel when the helper is stopped", () => {
    const stage = stageIn();
    const board = boardAtOrigin(stage);
    const card = element();
    board.addItem(card, { x: 50, y: 50, w: 100, h: 100 });
    const { callbacks, calls } = recorder();
    const stop = board.enableDrag(card, callbacks);
    press(card, 60, 60);
    moveTo(90, 60);
    stop();
    moveTo(200, 200);
    release(200, 200);
    expect(calls.map(([name]) => name)).toEqual(["start", "cancel"]);
  });

  it("releases the drag helpers of an item that is taken off the board, so a later press does nothing", () => {
    const stage = stageIn();
    const board = boardAtOrigin(stage);
    const card = element();
    board.addItem(card, { x: 50, y: 50, w: 100, h: 100 });
    const { callbacks, calls } = recorder();
    const stop = board.enableDrag(card, callbacks);
    press(card, 60, 60);
    moveTo(90, 60);
    board.removeItem(card);
    expect(calls.map(([name]) => name)).toEqual(["start", "cancel"]);

    // Back on the board, the item needs its drag again; a stale stop is harmless.
    board.addItem(card, { x: 50, y: 50, w: 100, h: 100 });
    press(card, 60, 60);
    moveTo(120, 60);
    release(120, 60);
    expect(calls.map(([name]) => name)).toEqual(["start", "cancel"]);
    expect(() => stop()).not.toThrow();
  });

  it("does nothing for a press on an element that is not an item", () => {
    const stage = stageIn();
    const board = boardAtOrigin(stage);
    const loose = element();
    stage.root.append(loose);
    const { callbacks, calls } = recorder();
    board.enableDrag(loose, callbacks);
    press(loose, 60, 60);
    moveTo(200, 200);
    release(200, 200);
    expect(calls).toEqual([]);
  });
});

describe("panning", () => {
  it("pans the view when a drag starts on empty space, by the pointer's distance in logical pixels", () => {
    const stage = stageIn();
    const board = createBoardView(stage, { worldBounds: WORLD, initialZoom: 2 });
    press(stage.root, 500, 500);
    moveTo(530, 470);
    release(530, 470);
    // The centred pan was (-2040, -1460); the pointer moved (30, -30).
    expect(board.pan).toEqual({ x: -2010, y: -1490 });
  });

  it("does not pan when a drag starts on an item, or on an element inside the item", () => {
    const stage = stageIn();
    const board = createBoardView(stage, { worldBounds: WORLD, initialZoom: 2 });
    const card = element();
    const inner = element();
    card.append(inner);
    board.addItem(card, { x: 0, y: 0, w: 100, h: 100 });
    const before = board.pan;
    press(inner, 500, 500);
    moveTo(530, 470);
    release(530, 470);
    expect(board.pan).toEqual(before);
  });

  it("does not pan when a drag starts on a HUD element beside the layer", () => {
    const stage = stageIn();
    const board = createBoardView(stage, { worldBounds: WORLD, initialZoom: 2 });
    const hud = element();
    stage.root.append(hud);
    const before = board.pan;
    press(hud, 500, 500);
    moveTo(530, 470);
    release(530, 470);
    expect(board.pan).toEqual(before);
  });

  it("follows the pointer in logical pixels when the window is smaller than the surface", () => {
    const stage = stageIn(960, 540);
    const board = createBoardView(stage, { worldBounds: WORLD });
    board.pan = { x: -500, y: -400 };
    press(stage.root, 100, 100);
    // A client move of (100, 50) is a logical move of (200, 100) at a scale of 0.5.
    moveTo(200, 150);
    release(200, 150);
    expect(board.pan).toEqual({ x: -300, y: -300 });
  });

  it("pans by a distance from the keyboard, in logical pixels", () => {
    const board = createBoardView(stageIn(), { worldBounds: WORLD, initialZoom: 2 });
    const before = board.pan;
    board.panBy(-40, 25);
    expect(board.pan).toEqual({ x: before.x - 40, y: before.y + 25 });
  });
});

describe("listeners and clean-up", () => {
  it("tells the listeners about every change of the view, and not about item moves or no change", () => {
    const stage = stageIn();
    const board = createBoardView(stage, { worldBounds: WORLD });
    board.pan = { x: -500, y: -400 };
    const seen: BoardViewState[] = [];
    const stopListening = board.onChange((state) => {
      seen.push(state);
    });

    board.panBy(10, 0);
    expect(seen).toEqual([{ zoom: 1, pan: { x: -490, y: -400 } }]);
    board.panBy(0, 0);
    const card = element();
    board.addItem(card, { x: 0, y: 0, w: 100, h: 100 });
    board.moveItem(card, { x: 5, y: 5, z: 2 });
    expect(seen.length).toBe(1);

    board.zoom = 2;
    expect(seen.length).toBe(2);
    expect(seen[1]?.zoom).toBe(2);

    stopListening();
    board.panBy(10, 0);
    expect(seen.length).toBe(2);
  });

  it("removes the layer, and stops the pan, the wheel and the drags when destroyed", () => {
    const stage = stageIn();
    const board = createBoardView(stage, { worldBounds: WORLD });
    board.pan = { x: 0, y: 0 };
    const card = element();
    board.addItem(card, { x: 50, y: 50, w: 100, h: 100 });
    const { callbacks, calls } = recorder();
    board.enableDrag(card, callbacks);
    press(card, 60, 60);
    moveTo(90, 60);
    board.destroy();

    expect(calls.map(([name]) => name)).toEqual(["start", "cancel"]);
    expect(board.el.parentElement).toBeNull();
    const zoom = board.zoom;
    wheel(stage.root, 700, 300, -100);
    press(stage.root, 500, 500);
    moveTo(530, 470);
    release(530, 470);
    expect(board.zoom).toBe(zoom);
    expect(stage.root.children.length).toBe(0);
  });
});

describe("the visible area", () => {
  it("centres the world in the part of the surface that is visible, not on the whole surface", () => {
    const board = createBoardView(stageIn(), { worldBounds: WORLD, visibleArea: { left: 400, top: 100 } });
    // The visible part starts at (400, 100) and is 1520 by 980. The world centre goes to its centre, (1160, 590).
    expect(board.pan.x).toBeCloseTo(400 + 760 - 1500, 6);
    expect(board.pan.y).toBeCloseTo(100 + 490 - 1000, 6);
  });

  it("centres a world smaller than the visible part inside it, and leaves the panels' side empty", () => {
    const board = createBoardView(stageIn(), { worldBounds: { x: 0, y: 0, w: 400, h: 300 }, visibleArea: { left: 400 } });
    // The visible part is 1520 wide from 400, so the 400 wide world sits 560 in from its left edge.
    expect(board.pan.x).toBeCloseTo(400 + 560, 6);
    expect(board.pan.y).toBeCloseTo((1080 - 300) / 2, 6);
  });

  it("zooms around the centre of the visible part, which keeps the world point there in place", () => {
    const board = createBoardView(stageIn(), { worldBounds: WORLD, visibleArea: { left: 400 } });
    const centre = { x: 400 + 1520 / 2, y: 540 };
    const world = board.logicalToWorld(centre.x, centre.y);
    board.zoom = 2;
    const after = board.worldToLogical(world.x, world.y);
    expect(board.zoom).toBe(2);
    expect(after.x).toBeCloseTo(centre.x, 6);
    expect(after.y).toBeCloseTo(centre.y, 6);
  });

  it("clamps the pan to the visible part, so the world covers it and not the space under the panels", () => {
    const board = createBoardView(stageIn(), { worldBounds: WORLD, visibleArea: { left: 400 } });
    board.pan = { x: 900, y: 0 };
    // The world's left edge may not pass the visible part's left edge, which is at 400. With no area it would stop at 0.
    expect(board.pan.x).toBe(400);
    board.pan = { x: -5000, y: 0 };
    // The world's right edge may not come in past the visible part's right edge: 400 + 1520 - 3000.
    expect(board.pan.x).toBeCloseTo(-1080, 6);
  });

  it("fits a rectangle in the visible part, as large as that part allows, and centres it there", () => {
    const board = createBoardView(stageIn(), { worldBounds: WORLD, visibleArea: { left: 400 } });
    board.fit({ x: 1000, y: 900, w: 800, h: 100 });
    // The visible part is 1520 wide, so the width limits the zoom: 1520 / 800 = 1.9.
    expect(board.zoom).toBeCloseTo(1.9, 9);
    const centre = board.worldToLogical(1400, 950);
    expect(centre.x).toBeCloseTo(400 + 760, 6);
    expect(centre.y).toBeCloseTo(540, 6);
  });

  it("reports the world rectangle that the visible part shows, at the current zoom", () => {
    const board = createBoardView(stageIn(), { worldBounds: WORLD, visibleArea: { left: 400, top: 100 } });
    board.zoom = 2;
    const origin = board.logicalToWorld(400, 100);
    const rect = board.visibleWorldRect();
    expect(rect.x).toBeCloseTo(origin.x, 6);
    expect(rect.y).toBeCloseTo(origin.y, 6);
    expect(rect.w).toBeCloseTo(1520 / 2, 6);
    expect(rect.h).toBeCloseTo(980 / 2, 6);
  });

  it("reports the whole surface as the visible part when no area is given", () => {
    const board = createBoardView(stageIn(), { worldBounds: WORLD });
    board.pan = { x: 0, y: 0 };
    expect(board.visibleWorldRect()).toEqual({ x: 0, y: 0, w: 1920, h: 1080 });
  });

  it("re-clamps the view and tells the listeners when the area changes, even when the view stays put", () => {
    const board = createBoardView(stageIn(), { worldBounds: WORLD });
    const seen: BoardViewState[] = [];
    board.onChange((state) => seen.push(state));
    board.setVisibleArea({ top: 96 });
    // The view is still inside the new area, so it does not move, but the change itself is reported.
    expect(seen).toHaveLength(1);
    expect(seen[0]?.zoom).toBe(1);
    board.setVisibleArea({ top: 96 });
    expect(seen).toHaveLength(1);
    board.setVisibleArea({});
    expect(seen).toHaveLength(2);
  });

  it("moves the view when the visible part gets smaller and the world is centred in it again", () => {
    const board = createBoardView(stageIn(), { worldBounds: { x: 0, y: 0, w: 400, h: 300 } });
    const seen: BoardViewState[] = [];
    board.onChange((state) => seen.push(state));
    board.setVisibleArea({ right: 1000 });
    // The visible part is 920 wide now, so the 400 wide world is centred in it: (920 - 400) / 2.
    expect(board.pan.x).toBeCloseTo(260, 6);
    expect(seen.at(-1)?.pan.x).toBeCloseTo(260, 6);
  });

  it("rejects insets that are negative or leave no room, and keeps the area it had", () => {
    const stage = stageIn();
    expect(() => createBoardView(stage, { worldBounds: WORLD, visibleArea: { top: 600, bottom: 600 } })).toThrow(RangeError);
    const board = createBoardView(stage, { worldBounds: WORLD, visibleArea: { left: 400 } });
    expect(() => board.setVisibleArea({ left: -1 })).toThrow(RangeError);
    expect(() => board.setVisibleArea({ left: 1000, right: 1000 })).toThrow(RangeError);
    expect(() => board.setVisibleArea({ top: Number.NaN })).toThrow(RangeError);
    expect(board.visibleWorldRect().w).toBeCloseTo(1520, 6);
  });
});
