// Tests for hit testing: the topmost interactive node wins, nodes are hit through their exact inverse
// transforms (a turned and scaled sprite is hit where it is drawn, not in its box), hit areas replace the
// shape, invisible and transparent nodes are skipped, a fixed layer is hit where it is drawn, and the hit test
// sees a node that moved since the last render.

import { describe, expect, it } from "vitest";
import { createScene2D, type Scene2D } from "./scene";
import { Group } from "./node";
import { Shape } from "./shape";
import { Sprite } from "./sprite";
import { Text } from "./text";
import { FakeCanvas, fakeImage } from "./test-support";

/** A scene with a fixed layer. */
function fixedScene(): { scene: Scene2D; screen: ReturnType<Scene2D["createLayer"]> } {
  const scene = createScene2D({ canvas: new FakeCanvas(1920, 1080) });
  return { scene, screen: scene.createLayer({ parallax: 0 }) };
}

describe("which node is hit", () => {
  it("returns the topmost of two overlapping interactive nodes", () => {
    const { scene, screen } = fixedScene();
    const below = new Sprite({ image: fakeImage(100, 100), interactive: true });
    const above = new Sprite({ image: fakeImage(100, 100), x: 50, y: 50, interactive: true, zIndex: 1 });
    screen.add(below, above);
    expect(scene.hitTest({ x: 60, y: 60 })).toBe(above);
    expect(scene.nodesAt({ x: 60, y: 60 })).toEqual([above, below]);
    expect(scene.hitTest({ x: 10, y: 10 })).toBe(below);
  });

  it("returns null where nothing interactive is", () => {
    const { scene, screen } = fixedScene();
    screen.add(new Sprite({ image: fakeImage(100, 100) }));
    expect(scene.hitTest({ x: 10, y: 10 })).toBeNull();
  });

  it("hits a child of a non-interactive group, and not the group", () => {
    const { scene, screen } = fixedScene();
    const group = new Group({ x: 20 });
    const child = new Sprite({ image: fakeImage(10, 10), interactive: true });
    group.add(child);
    screen.add(group);
    expect(scene.hitTest({ x: 25, y: 5 })).toBe(child);
    expect(scene.nodesAt({ x: 25, y: 5 })).toEqual([child]);
  });

  it("uses the exact inverse of a turned and scaled node", () => {
    const { scene, screen } = fixedScene();
    // A 100 by 50 sprite, its centre on (500, 300), scaled by 2 and turned a quarter turn.
    const sprite = new Sprite({
      image: fakeImage(100, 50),
      x: 500,
      y: 300,
      anchorX: 0.5,
      anchorY: 0.5,
      scaleX: 2,
      scaleY: 2,
      rotation: Math.PI / 2,
      interactive: true,
    });
    screen.add(sprite);
    // After the turn, the sprite spans 100 across x (from -50 to 50) and 200 down y (from -100 to 100).
    expect(scene.hitTest({ x: 540, y: 390 })).toBe(sprite);
    expect(scene.hitTest({ x: 560, y: 300 })).toBeNull();
    expect(scene.hitTest({ x: 500, y: 410 })).toBeNull();
    // Its unturned box would have been wide, not tall: the point 80 across is outside the turned sprite.
    expect(scene.hitTest({ x: 580, y: 300 })).toBeNull();
  });

  it("uses the hit area in place of the shape", () => {
    const { scene, screen } = fixedScene();
    const area = new Group({ interactive: true, hitArea: { x: 0, y: 0, width: 100, height: 100 } });
    screen.add(area);
    expect(scene.hitTest({ x: 50, y: 50 })).toBe(area);
    expect(scene.hitTest({ x: 150, y: 50 })).toBeNull();
  });

  it("hits a thin line within its tolerance, and a polygon by its inside", () => {
    const { scene, screen } = fixedScene();
    const line = new Shape({
      geometry: { kind: "line", points: [{ x: 0, y: 0 }, { x: 100, y: 0 }] },
      lineWidth: 1,
      stroke: "#fff",
      interactive: true,
    });
    const triangle = new Shape({
      geometry: { kind: "polygon", points: [{ x: 200, y: 0 }, { x: 300, y: 0 }, { x: 250, y: 100 }] },
      interactive: true,
    });
    screen.add(line, triangle);
    // The line's tolerance is at least 2 pixels, so 1.5 away is a hit and 3 away is not.
    expect(scene.hitTest({ x: 50, y: 1.5 })).toBe(line);
    expect(scene.hitTest({ x: 50, y: 3 })).toBeNull();
    expect(scene.hitTest({ x: 250, y: 30 })).toBe(triangle);
    expect(scene.hitTest({ x: 205, y: 90 })).toBeNull();
  });

  it("hits a rounded rectangle inside its rounded corners only", () => {
    const { scene, screen } = fixedScene();
    screen.add(new Shape({ geometry: { kind: "roundRect", width: 100, height: 100, radius: 40 }, interactive: true }));
    expect(scene.hitTest({ x: 50, y: 50 })).not.toBeNull();
    // The very corner is cut away by the radius (40 from the corner, so the arc's centre is at (40, 40)).
    expect(scene.hitTest({ x: 2, y: 2 })).toBeNull();
    expect(scene.hitTest({ x: 30, y: 5 })).not.toBeNull();
  });

  it("hits a text by its box", () => {
    const { scene, screen } = fixedScene();
    screen.add(new Text({ text: "abc", fontSize: 20, interactive: true }));
    // The fake gives 30 pixels for "abc" and a line box of 24.
    expect(scene.hitTest({ x: 29, y: 23 })).not.toBeNull();
    expect(scene.hitTest({ x: 31, y: 10 })).toBeNull();
  });

  it("skips invisible and transparent nodes, and the subtree under an invisible group", () => {
    const { scene, screen } = fixedScene();
    const hidden = new Sprite({ image: fakeImage(100, 100), interactive: true, visible: false });
    const clear = new Sprite({ image: fakeImage(100, 100), interactive: true, alpha: 0 });
    const group = new Group({ visible: false });
    group.add(new Sprite({ image: fakeImage(100, 100), interactive: true }));
    screen.add(hidden, clear, group);
    expect(scene.hitTest({ x: 10, y: 10 })).toBeNull();
  });

  it("hits a node in a fixed layer where it is drawn, not where the world has it", () => {
    const canvas = new FakeCanvas(1920, 1080);
    const scene = createScene2D({ canvas });
    scene.camera.x = 300;
    const hud = scene.createLayer({ parallax: 0 });
    const world = scene.createLayer({ parallax: 1 });
    const button = new Sprite({ image: fakeImage(50, 50), interactive: true });
    const worldButton = new Sprite({ image: fakeImage(50, 50), interactive: true, x: 0, y: 0 });
    hud.add(button);
    world.add(worldButton);
    // The world origin is drawn at (660, 540), because the camera is 300 to the right of it. The fixed button
    // is at the top-left of the screen, where it is drawn, and the world button is where the world puts it.
    expect(scene.hitTest({ x: 10, y: 10 })).toBe(button);
    expect(scene.hitTest({ x: 665, y: 545 })).toBe(worldButton);
  });

  it("sees a node that moved since the last render", () => {
    const { scene, screen } = fixedScene();
    const sprite = new Sprite({ image: fakeImage(10, 10), interactive: true });
    screen.add(sprite);
    scene.render();
    sprite.x = 400;
    expect(scene.hitTest({ x: 405, y: 5 })).toBe(sprite);
    expect(scene.hitTest({ x: 5, y: 5 })).toBeNull();
  });
});
