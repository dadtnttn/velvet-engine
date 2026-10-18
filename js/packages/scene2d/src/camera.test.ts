// Tests for the camera: the screen and world conversions are exact inverses, the view zooms and turns about the
// centre, the limits hold the position inside the bounds (and centre a box that is smaller than the view), the
// follow moves toward its target with a pace that does not depend on the frame rate and stops at the edge of a
// deadzone, the shake is the same for the same seed and steps and dies away, and a paused clock stops the camera.

import { GameClock } from "@velvet/core";
import { describe, expect, it } from "vitest";
import { Camera2D } from "./camera";
import { Group } from "./node";
import { Sprite } from "./sprite";
import { createScene2D } from "./scene";
import { expectNumbers, FakeCanvas, fakeImage } from "./test-support";
import { Matrix2D } from "./math";

/** A camera for a 1920 by 1080 surface. */
function camera(options: { clock?: GameClock; seed?: number } = {}): Camera2D {
  return new Camera2D({ width: 1920, height: 1080, ...options });
}

/** The world-to-surface view of a layer with parallax 1, as six numbers. */
function viewOf(cam: Camera2D): number[] {
  const m = cam.viewFor(1, new Matrix2D());
  return [m.a, m.b, m.c, m.d, m.e, m.f];
}

describe("screen and world", () => {
  it("puts the camera position at the centre of the surface", () => {
    const cam = camera();
    cam.x = 100;
    cam.y = -40;
    expect(cam.worldToScreen({ x: 100, y: -40 })).toEqual({ x: 960, y: 540 });
  });

  it("zooms about the centre", () => {
    const cam = camera();
    cam.zoom = 2;
    expect(cam.worldToScreen({ x: 10, y: 0 })).toEqual({ x: 980, y: 540 });
  });

  it("turns about the centre, clockwise on the screen", () => {
    const cam = camera();
    cam.rotation = Math.PI / 2;
    const at = cam.worldToScreen({ x: 10, y: 0 });
    expect(at.x).toBeCloseTo(960, 9);
    expect(at.y).toBeCloseTo(550, 9);
  });

  it("converts a surface point to the world and back, exactly", () => {
    const cam = camera();
    cam.x = 321.5;
    cam.y = -77.25;
    cam.zoom = 2.5;
    cam.rotation = 0.7;
    const points = [
      { x: 0, y: 0 },
      { x: 1919, y: 1079 },
      { x: 960.5, y: 300.25 },
    ];
    for (const point of points) {
      const world = cam.screenToWorld(point);
      const back = cam.worldToScreen(world);
      expect(back.x).toBeCloseTo(point.x, 9);
      expect(back.y).toBeCloseTo(point.y, 9);
    }
  });

  it("includes the shake, so that the conversion matches what is drawn", () => {
    const cam = camera({ seed: 7 });
    cam.shake(20, 1);
    const drawn = cam.worldToScreen({ x: 50, y: 50 });
    const world = cam.screenToWorld(drawn);
    expect(world.x).toBeCloseTo(50, 9);
    expect(world.y).toBeCloseTo(50, 9);
  });

  it("refuses a zoom that is not positive", () => {
    const cam = camera();
    expect(() => {
      cam.zoom = 0;
    }).toThrow(RangeError);
  });
});

describe("bounds", () => {
  it("holds the position so that the view stays inside the bounds", () => {
    const cam = camera();
    cam.bounds = { x: 0, y: 0, width: 2000, height: 1000 };
    // At zoom 1 the view is 1920 wide, so the centre may go from 960 to 1040.
    cam.x = -500;
    expect(cam.x).toBe(960);
    cam.x = 5000;
    expect(cam.x).toBe(1040);
    cam.x = 1000;
    expect(cam.x).toBe(1000);
  });

  it("allows a bigger range when zoomed out, and a smaller one when zoomed in", () => {
    const cam = camera();
    cam.bounds = { x: 0, y: 0, width: 2000, height: 1000 };
    cam.zoom = 2;
    // At zoom 2 the view is half as wide in the world: the centre may go from 480 to 1520.
    cam.x = -5000;
    expect(cam.x).toBe(480);
  });

  it("centres the position when the bounds are smaller than the view", () => {
    const cam = camera();
    cam.bounds = { x: 100, y: 0, width: 500, height: 200 };
    cam.x = 9999;
    expect(cam.x).toBe(350);
  });

  it("holds the position again when the bounds change", () => {
    const cam = camera();
    cam.x = 2000;
    cam.bounds = { x: 0, y: 0, width: 1920, height: 1080 };
    expect(cam.x).toBe(960);
  });
});

describe("following", () => {
  it("snaps to a target with a lerp of 1, once it has a step", () => {
    const cam = camera();
    cam.follow({ x: 300, y: -200 });
    cam.tick(0);
    expect(cam.x).toBe(0);
    cam.tick(1 / 60);
    expect(cam.x).toBeCloseTo(300, 9);
    expect(cam.y).toBeCloseTo(-200, 9);
  });

  it("covers half the remaining distance in one frame at a lerp of 0.5, and three quarters in two", () => {
    const cam = camera();
    cam.follow({ x: 400, y: 0 }, { lerp: 0.5 });
    cam.tick(1 / 60);
    expect(cam.x).toBeCloseTo(200, 9);
    cam.tick(1 / 60);
    expect(cam.x).toBeCloseTo(300, 9);
  });

  it("keeps the same pace at any frame rate", () => {
    const fast = camera();
    const slow = camera();
    fast.follow({ x: 400, y: 0 }, { lerp: 0.5 });
    slow.follow({ x: 400, y: 0 }, { lerp: 0.5 });
    for (let index = 0; index < 120; index += 1) fast.tick(1 / 120);
    for (let index = 0; index < 60; index += 1) slow.tick(1 / 60);
    expect(fast.x).toBeCloseTo(slow.x, 9);
  });

  it("moves only as far as the edge of the deadzone", () => {
    const cam = camera();
    cam.follow({ x: 150, y: 0 }, { deadzone: { width: 200, height: 0 } });
    cam.tick(1 / 60);
    // The box reaches 100 either way, so the target at 150 leaves the camera at 50.
    expect(cam.x).toBeCloseTo(50, 9);
    cam.unfollow();
    expect(cam.followTarget).toBeNull();
  });

  it("does not move while the target stays inside the deadzone", () => {
    const cam = camera();
    cam.follow({ x: 100, y: 0 }, { deadzone: { width: 200, height: 200 } });
    cam.tick(1 / 60);
    expect(cam.x).toBe(0);
  });

  it("follows the pivot of a node in its layer", () => {
    const canvas = new FakeCanvas(1920, 1080);
    const scene = createScene2D({ canvas });
    const layer = scene.createLayer({ parallax: 1 });
    const holder = new Group({ x: 300, y: 200 });
    const player = new Sprite({ image: fakeImage(10, 10) });
    holder.add(player);
    layer.add(holder);
    scene.camera.follow(player);
    scene.tick(1 / 60);
    expect(scene.camera.x).toBeCloseTo(300, 9);
    expect(scene.camera.y).toBeCloseTo(200, 9);
  });

  it("refuses a lerp outside 0 to 1", () => {
    expect(() => camera().follow({ x: 0, y: 0 }, { lerp: 1.5 })).toThrow(RangeError);
  });
});

describe("shake", () => {
  it("gives the same offsets for the same seed and the same steps", () => {
    const a = camera({ seed: 42 });
    const b = camera({ seed: 42 });
    a.shake(10, 0.5);
    b.shake(10, 0.5);
    const frames: number[][] = [];
    for (let index = 0; index < 10; index += 1) {
      a.tick(1 / 60);
      b.tick(1 / 60);
      expect(viewOf(a)).toEqual(viewOf(b));
      frames.push(viewOf(a));
    }
    // And the shake moves the view at all.
    expect(frames.some((view) => view[4] !== 960)).toBe(true);
  });

  it("gives different offsets for a different seed", () => {
    const a = camera({ seed: 1 });
    const b = camera({ seed: 2 });
    a.shake(10, 1);
    b.shake(10, 1);
    expect(viewOf(a)).not.toEqual(viewOf(b));
  });

  it("dies away over its length, and then leaves the view where it was", () => {
    const cam = camera({ seed: 3 });
    const before = viewOf(cam);
    cam.shake(30, 0.25);
    cam.tick(0.1);
    cam.tick(0.2);
    expect(viewOf(cam)).toEqual(before);
  });

  it("keeps the running shake when a weaker one arrives", () => {
    const cam = camera({ seed: 5 });
    cam.shake(10, 1);
    const running = viewOf(cam);
    cam.shake(1, 1);
    expect(viewOf(cam)).toEqual(running);
  });

  it("does nothing for a shake of zero strength", () => {
    const cam = camera();
    cam.shake(0, 1);
    expect(viewOf(cam)).toEqual(viewOf(camera()));
  });
});

describe("the clock", () => {
  it("stops the shake and the follow while the clock is paused", () => {
    const clock = new GameClock();
    const cam = camera({ clock, seed: 9 });
    cam.shake(10, 1);
    const shaken = viewOf(cam);
    clock.pause();
    cam.tick(0.5);
    expect(viewOf(cam)).toEqual(shaken);
    cam.follow({ x: 500, y: 0 });
    cam.tick(0.5);
    expect(cam.x).toBe(0);
    clock.resume();
    cam.tick(1 / 60);
    expect(cam.x).toBeCloseTo(500, 9);
  });

  it("runs the shake at the clock's speed", () => {
    const clock = new GameClock({ speeds: [1, 2] });
    const cam = camera({ clock, seed: 4 });
    const before = viewOf(cam);
    cam.shake(10, 1);
    clock.setSpeed(2);
    // Half a real second is one game second at speed 2, which ends a shake of one second.
    cam.tick(0.5);
    expect(viewOf(cam)).toEqual(before);
  });

  it("refuses a negative step", () => {
    expect(() => camera().tick(-1)).toThrow(RangeError);
  });
});

describe("parallax views", () => {
  it("is the identity for a layer fixed to the screen", () => {
    const cam = camera();
    cam.x = 500;
    cam.zoom = 3;
    const view = cam.viewFor(0, new Matrix2D());
    expectNumbers([view.a, view.b, view.c, view.d, view.e, view.f], [1, 0, 0, 1, 0, 0]);
  });

  it("moves a layer by its parallax factor of the camera's move", () => {
    const cam = camera();
    cam.x = 500;
    const view = cam.viewFor(0.25, new Matrix2D());
    // 0.25 * 960 - 0.25 * 500 = 115 on x, and 0.25 * 540 = 135 on y.
    expect(view.e).toBeCloseTo(115, 9);
    expect(view.f).toBeCloseTo(135, 9);
  });
});
