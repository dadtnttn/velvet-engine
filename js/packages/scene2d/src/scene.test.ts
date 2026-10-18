// Tests for the scene as a whole: the tree (adding, moving, cycles), the draw order, the composition of
// placement, opacity and visibility, the native pixel scale, layers and their parallax, the culling of
// off-screen subtrees, the stats of a render, and the background fill.

import { describe, expect, it } from "vitest";
import { createScene2D, type Scene2D, type Scene2DCanvas } from "./scene";
import { Group } from "./node";
import { Sprite } from "./sprite";
import { expectNumbers, FakeCanvas, fakeImage } from "./test-support";

/** A scene on a 1920 by 1080 fake canvas, with one layer that is fixed to the screen (its view is the identity). */
function setup(options: { now?: () => number; background?: string | null } = {}) {
  const canvas = new FakeCanvas(1920, 1080);
  const scene = createScene2D({ canvas, now: options.now, background: options.background });
  const screen = scene.createLayer({ name: "screen", parallax: 0 });
  return { canvas, scene, screen };
}

/** The image identities of the draws, in order. */
function drawnImages(canvas: FakeCanvas): unknown[] {
  return canvas.context.nodeDraws.filter((draw) => draw.op === "drawImage").map((draw) => draw.image);
}

describe("the tree", () => {
  it("adds a node to one parent, and moves it when it is added to another", () => {
    const first = new Group();
    const second = new Group();
    const child = new Group();
    first.add(child);
    expect(child.parent).toBe(first);
    second.add(child);
    expect(child.parent).toBe(second);
    expect(first.children).toEqual([]);
    expect(second.children).toEqual([child]);
  });

  it("removes and clears children", () => {
    const parent = new Group();
    const a = new Group();
    const b = new Group();
    parent.add(a, b);
    parent.remove(a);
    expect(a.parent).toBeNull();
    expect(parent.children).toEqual([b]);
    parent.clear();
    expect(b.parent).toBeNull();
    expect(parent.children).toEqual([]);
  });

  it("refuses to make a node its own ancestor", () => {
    const parent = new Group();
    const child = new Group();
    parent.add(child);
    expect(() => child.add(parent)).toThrow(RangeError);
    expect(() => parent.add(parent)).toThrow(RangeError);
  });
});

describe("draw order", () => {
  it("draws a parent before its children", () => {
    const { canvas, screen, scene } = setup();
    const back = fakeImage(4, 4);
    const front = fakeImage(4, 4);
    const parent = new Sprite({ image: back });
    parent.add(new Sprite({ image: front }));
    screen.add(parent);
    scene.render();
    expect(drawnImages(canvas)).toEqual([back, front]);
  });

  it("draws siblings by zIndex, then by the order in which they were added", () => {
    const { canvas, screen, scene } = setup();
    const a = fakeImage(1, 1);
    const b = fakeImage(1, 1);
    const c = fakeImage(1, 1);
    const spriteA = new Sprite({ image: a, zIndex: 1 });
    const spriteB = new Sprite({ image: b });
    const spriteC = new Sprite({ image: c });
    screen.add(spriteA, spriteB, spriteC);
    scene.render();
    expect(drawnImages(canvas)).toEqual([b, c, a]);
  });

  it("keeps the order of insertion when a zIndex changes", () => {
    const { canvas, screen, scene } = setup();
    const a = fakeImage(1, 1);
    const b = fakeImage(1, 1);
    const spriteA = new Sprite({ image: a, zIndex: 2 });
    const spriteB = new Sprite({ image: b, zIndex: 1 });
    screen.add(spriteA, spriteB);
    scene.render();
    expect(drawnImages(canvas)).toEqual([b, a]);
    spriteB.zIndex = 2;
    canvas.context.clearRecording();
    scene.render();
    // Equal zIndex: the one added first (a) draws first.
    expect(drawnImages(canvas)).toEqual([a, b]);
  });

  it("draws layers by zIndex, then in creation order", () => {
    // The screen layer from setup() is created first, with zIndex 0, so it comes first of the zero layers.
    const { canvas, scene, screen } = setup();
    const first = scene.createLayer({ parallax: 0, zIndex: 5 });
    const second = scene.createLayer({ parallax: 0, zIndex: 0 });
    const third = scene.createLayer({ parallax: 0, zIndex: 0 });
    const images = [fakeImage(1, 1), fakeImage(1, 1), fakeImage(1, 1)];
    first.add(new Sprite({ image: images[0] }));
    second.add(new Sprite({ image: images[1] }));
    third.add(new Sprite({ image: images[2] }));
    scene.render();
    expect(drawnImages(canvas)).toEqual([images[1], images[2], images[0]]);
    expect(scene.layers).toEqual([screen, second, third, first]);
  });

  it("removes a layer from the scene", () => {
    const { scene, screen } = setup();
    scene.removeLayer(screen);
    expect(scene.layers).toEqual([]);
  });
});

describe("placement, opacity and visibility", () => {
  it("composes the placement of a parent and a child", () => {
    const { canvas, screen, scene } = setup();
    const parent = new Group({ x: 100, y: 200, scaleX: 2, scaleY: 2 });
    const child = new Sprite({ image: fakeImage(5, 5), x: 10, y: 20 });
    parent.add(child);
    screen.add(parent);
    scene.render();
    expectNumbers(canvas.context.nodeDraws[0]?.transform, [2, 0, 0, 2, 120, 240]);
  });

  it("turns a child around its parent", () => {
    const { canvas, screen, scene } = setup();
    const parent = new Group({ rotation: Math.PI / 2 });
    parent.add(new Sprite({ image: fakeImage(5, 5), x: 10, y: 0 }));
    screen.add(parent);
    scene.render();
    const [a, b, c, d, e, f] = canvas.context.nodeDraws[0]?.transform ?? [];
    expect(a).toBeCloseTo(0, 9);
    expect(b).toBeCloseTo(1, 9);
    expect(c).toBeCloseTo(-1, 9);
    expect(d).toBeCloseTo(0, 9);
    expect(e).toBeCloseTo(0, 9);
    expect(f).toBeCloseTo(10, 9);
  });

  it("multiplies the opacity of every level", () => {
    const { canvas, scene } = setup();
    const layer = scene.createLayer({ parallax: 0, alpha: 0.5 });
    const group = new Group({ alpha: 0.5 });
    group.add(new Sprite({ image: fakeImage(2, 2), alpha: 0.5 }));
    layer.add(group);
    scene.render();
    expect(canvas.context.nodeDraws[0]?.alpha).toBeCloseTo(0.125, 12);
  });

  it("draws nothing under an invisible node, and nothing at zero opacity", () => {
    const { canvas, screen, scene } = setup();
    const hidden = new Group({ visible: false });
    hidden.add(new Sprite({ image: fakeImage(2, 2) }));
    const transparent = new Sprite({ image: fakeImage(2, 2), alpha: 0 });
    screen.add(hidden, transparent);
    scene.render();
    expect(canvas.context.nodeDraws).toEqual([]);
    expect(scene.stats().nodes).toBe(0);
  });

  it("hides a layer with its visibility", () => {
    const { canvas, scene } = setup();
    const layer = scene.createLayer({ parallax: 0, visible: false });
    layer.add(new Sprite({ image: fakeImage(2, 2) }));
    scene.render();
    expect(canvas.context.nodeDraws).toEqual([]);
  });

  it("takes pixel art from the nearest node that sets it", () => {
    const { canvas, scene } = setup();
    const layer = scene.createLayer({ parallax: 0, pixelArt: true });
    const inherits = new Sprite({ image: fakeImage(2, 2) });
    const smooth = new Sprite({ image: fakeImage(2, 2), pixelArt: false });
    layer.add(inherits, smooth);
    scene.render();
    expect(canvas.context.nodeDraws.map((draw) => draw.smoothing)).toEqual([false, true]);
  });
});

describe("the native pixel scale", () => {
  it("starts at the logical size, with a scale of one", () => {
    const { canvas, scene } = setup();
    expect(canvas.width).toBe(1920);
    expect(canvas.height).toBe(1080);
    expect(scene.pixelScale).toBe(1);
  });

  it("draws at the scale of a resized backing store", () => {
    const { canvas, screen, scene } = setup();
    screen.add(new Sprite({ image: fakeImage(10, 10), x: 100, y: 40 }));
    scene.resize(2880, 1620);
    expect(canvas.width).toBe(2880);
    expect(canvas.height).toBe(1620);
    expect(scene.pixelScale).toBeCloseTo(1.5, 12);
    scene.render();
    expectNumbers(canvas.context.nodeDraws[0]?.transform, [1.5, 0, 0, 1.5, 150, 60]);
  });

  it("sets the scale and the backing store together", () => {
    const { canvas, scene } = setup();
    scene.setPixelScale(0.5);
    expect(canvas.width).toBe(960);
    expect(canvas.height).toBe(540);
    expect(scene.pixelScale).toBe(0.5);
  });

  it("refuses sizes that are not positive", () => {
    const { scene } = setup();
    expect(() => scene.resize(0, 100)).toThrow(RangeError);
    expect(() => scene.setPixelScale(-1)).toThrow(RangeError);
  });

  it("refuses a surface that is not positive", () => {
    expect(() => createScene2D({ canvas: new FakeCanvas(), width: 0 })).toThrow(RangeError);
  });
});

describe("layers and the parallax", () => {
  /** A sprite at the world origin, drawn in a layer with the given parallax, with the camera at (500, 0). */
  function drawnAt(parallax: number): readonly number[] {
    const canvas = new FakeCanvas(1920, 1080);
    const scene: Scene2D = createScene2D({ canvas });
    scene.camera.x = 500;
    const layer = scene.createLayer({ parallax });
    layer.add(new Sprite({ image: fakeImage(2, 2) }));
    scene.render();
    return canvas.context.nodeDraws[0]?.transform ?? [];
  }

  it("fixes a layer with parallax 0 to the screen", () => {
    expectNumbers(drawnAt(0), [1, 0, 0, 1, 0, 0]);
  });

  it("moves a layer with parallax 1 with the camera, the camera centred", () => {
    // The camera at x = 500 puts the world point 500 at the centre (960), so the origin is at 460.
    expectNumbers(drawnAt(1), [1, 0, 0, 1, 460, 540]);
  });

  it("moves a layer in between at its own fraction of the camera", () => {
    // Half the camera's move: the origin is at 0.5 * 960 - 0.5 * 500 = 230.
    const [, , , , e, f] = drawnAt(0.5);
    expect(e).toBeCloseTo(230, 9);
    expect(f).toBeCloseTo(270, 9);
  });

  it("keeps the zoom of a parallax 0 layer at 1", () => {
    const canvas = new FakeCanvas(1920, 1080);
    const scene = createScene2D({ canvas });
    scene.camera.zoom = 2;
    const hud = scene.createLayer({ parallax: 0 });
    hud.add(new Sprite({ image: fakeImage(2, 2), x: 30, y: 40 }));
    scene.render();
    expectNumbers(canvas.context.nodeDraws[0]?.transform, [1, 0, 0, 1, 30, 40]);
  });
});

describe("culling and stats", () => {
  it("skips a node that lies off the surface and counts it", () => {
    const { canvas, screen, scene } = setup();
    screen.add(new Sprite({ image: fakeImage(10, 10), x: 5000, y: 5000 }));
    screen.add(new Sprite({ image: fakeImage(10, 10), x: 100, y: 100 }));
    scene.render();
    expect(canvas.context.nodeDraws).toHaveLength(1);
    expect(scene.stats()).toMatchObject({ nodes: 1, culled: 1, drawCalls: 1 });
  });

  it("skips a whole subtree once, when its box is off the surface", () => {
    const { screen, scene } = setup();
    const group = new Group({ x: 5000 });
    group.add(new Sprite({ image: fakeImage(10, 10) }), new Sprite({ image: fakeImage(10, 10), x: 20 }));
    screen.add(group);
    scene.render();
    expect(scene.stats()).toMatchObject({ nodes: 0, culled: 1, drawCalls: 0 });
  });

  it("keeps a node whose box touches the edge of the surface", () => {
    const { canvas, screen, scene } = setup();
    screen.add(new Sprite({ image: fakeImage(10, 10), x: 1920 }));
    scene.render();
    expect(canvas.context.nodeDraws).toHaveLength(1);
  });

  it("reports the time of the last render from the clock it is given", () => {
    const times = [10, 25];
    const { scene } = setup({ now: () => times.shift() ?? 0 });
    scene.render();
    expect(scene.stats().renderMs).toBe(15);
  });

  it("returns a copy of its stats", () => {
    const { scene } = setup();
    scene.render();
    const first = scene.stats();
    expect(scene.stats()).toEqual(first);
    expect(first).not.toBe(scene.stats());
  });
});

describe("the background", () => {
  it("clears the canvas by default", () => {
    const { canvas, scene } = setup();
    scene.render();
    expect(canvas.context.calls.map((call) => call.name)).toContain("clearRect");
  });

  it("fills the canvas with the background colour, before the layers", () => {
    const { canvas, screen, scene } = setup({ background: "#102030" });
    screen.add(new Sprite({ image: fakeImage(2, 2) }));
    scene.render();
    const first = canvas.context.draws[0];
    expect(first?.op).toBe("fillRect");
    expect(first?.args).toEqual([0, 0, 1920, 1080]);
    expect(first?.fillStyle).toBe("#102030");
    expect(canvas.context.draws[1]?.op).toBe("drawImage");
  });
});

describe("canvas types", () => {
  it("accepts an HTML canvas and an offscreen canvas as the scene's canvas", () => {
    // The compiler checks these two lines. Nothing is created here, so no DOM is needed at run time.
    const html: Scene2DCanvas = {} as HTMLCanvasElement;
    const offscreen: Scene2DCanvas = {} as OffscreenCanvas;
    expect([html, offscreen]).toHaveLength(2);
  });
});
