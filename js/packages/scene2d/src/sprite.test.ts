// Tests for the picture nodes. A sprite draws the whole image, or a frame of it, at its box; it scales a frame
// to a set width; it puts a trimmed picture back in its untrimmed box; it turns a stored frame back; it flips;
// and it draws nothing without a loaded image. A nine-slice keeps its corners and stretches the middle.

import { describe, expect, it } from "vitest";
import { createScene2D } from "./scene";
import { NineSlice } from "./nine-slice";
import { Sprite, type SpriteFrame } from "./sprite";
import { expectNumbers, FakeCanvas, fakeCanvasImage, fakeImage } from "./test-support";

/** A scene with a fixed layer, and the canvas it draws into. */
function stage() {
  const canvas = new FakeCanvas(1920, 1080);
  const scene = createScene2D({ canvas });
  const layer = scene.createLayer({ parallax: 0 });
  return { canvas, scene, layer };
}

/** Draws one node alone, and returns the single draw that it made. */
function drawOne(node: Sprite | NineSlice) {
  const { canvas, scene, layer } = stage();
  layer.add(node);
  scene.render();
  return canvas.context.nodeDraws;
}

describe("Sprite", () => {
  it("draws the whole image at its box when it has no frame", () => {
    const image = fakeImage(64, 48);
    const [draw] = drawOne(new Sprite({ image }));
    expect(draw?.op).toBe("drawImage");
    expect(draw?.image).toBe(image);
    expect(draw?.args).toEqual([0, 0, 64, 48, 0, 0, 64, 48]);
  });

  it("draws the part of the image that its frame names", () => {
    const frame: SpriteFrame = { x: 10, y: 20, width: 30, height: 40 };
    const [draw] = drawOne(new Sprite({ image: fakeImage(128, 128), frame }));
    expect(draw?.args).toEqual([10, 20, 30, 40, 0, 0, 30, 40]);
  });

  it("scales the frame to the width and height that it is given", () => {
    const frame: SpriteFrame = { x: 10, y: 20, width: 30, height: 40 };
    const [draw] = drawOne(new Sprite({ image: fakeImage(128, 128), frame, width: 60, height: 80 }));
    expect(draw?.args).toEqual([10, 20, 30, 40, 0, 0, 60, 80]);
  });

  it("puts a trimmed picture back at its place in the untrimmed box", () => {
    // The picture is 20 by 10, trimmed from a 32 by 24 box, starting 4 right and 6 down.
    const frame: SpriteFrame = { x: 0, y: 0, width: 20, height: 10, trimX: 4, trimY: 6, sourceWidth: 32, sourceHeight: 24 };
    const sprite = new Sprite({ image: fakeImage(64, 64), frame });
    expect(sprite.frame).toBe(frame);
    const [draw] = drawOne(sprite);
    expect(draw?.args).toEqual([0, 0, 20, 10, 4, 6, 20, 10]);
  });

  it("scales a trimmed picture with its untrimmed box", () => {
    const frame: SpriteFrame = { x: 0, y: 0, width: 20, height: 10, trimX: 4, trimY: 6, sourceWidth: 32, sourceHeight: 24 };
    // The box is twice the untrimmed size, so the picture doubles and moves twice as far.
    const [draw] = drawOne(new Sprite({ image: fakeImage(64, 64), frame, width: 64, height: 48 }));
    expect(draw?.args).toEqual([0, 0, 20, 10, 8, 12, 40, 20]);
  });

  it("turns a stored frame back, so that its top-left stored pixel is the bottom-left of the picture", () => {
    // The picture is 20 wide and 10 tall. Stored turned clockwise, the frame is 10 wide and 20 tall.
    const frame: SpriteFrame = { x: 0, y: 0, width: 10, height: 20, rotated: true };
    const [draw] = drawOne(new Sprite({ image: fakeImage(64, 64), frame }));
    expect(draw?.args).toEqual([0, 0, 10, 20, 0, 0, 10, 20]);
    // The transform maps a stored point (u, v) to (v, 10 - u): the stored corner (0, 0) lands at (0, 10).
    const [a, b, c, d, e, f] = draw?.transform ?? [];
    const mapped = (u: number, v: number) => ({ x: (a ?? 0) * u + (c ?? 0) * v + (e ?? 0), y: (b ?? 0) * u + (d ?? 0) * v + (f ?? 0) });
    expect(mapped(0, 0)).toEqual({ x: 0, y: 10 });
    expect(mapped(10, 20)).toEqual({ x: 20, y: 0 });
  });

  it("mirrors the picture from left to right about the middle of its box", () => {
    const sprite = new Sprite({ image: fakeImage(64, 48), flipX: true });
    const [draw] = drawOne(sprite);
    // The flip maps x to 64 - x, so the transform is (-1, 0, 0, 1, 64, 0).
    expectNumbers(draw?.transform, [-1, 0, 0, 1, 64, 0]);
  });

  it("draws nothing without an image, or before the image has loaded", () => {
    expect(drawOne(new Sprite())).toEqual([]);
    expect(drawOne(new Sprite({ image: fakeImage(0, 0) }))).toEqual([]);
  });

  it("takes its size from a canvas image as well", () => {
    const [draw] = drawOne(new Sprite({ image: fakeCanvasImage(16, 9) }));
    expect(draw?.args).toEqual([0, 0, 16, 9, 0, 0, 16, 9]);
  });

  it("switches smoothing off for pixel art, and on otherwise", () => {
    const pixel = drawOne(new Sprite({ image: fakeImage(4, 4), pixelArt: true }));
    const smooth = drawOne(new Sprite({ image: fakeImage(4, 4) }));
    expect(pixel[0]?.smoothing).toBe(false);
    expect(smooth[0]?.smoothing).toBe(true);
  });

  it("changes its frame and redraws with it", () => {
    const sprite = new Sprite({ image: fakeImage(64, 64), frame: { x: 0, y: 0, width: 8, height: 8 } });
    sprite.frame = { x: 8, y: 0, width: 8, height: 8 };
    const [draw] = drawOne(sprite);
    expect(draw?.args).toEqual([8, 0, 8, 8, 0, 0, 8, 8]);
  });
});

describe("NineSlice", () => {
  const frame: SpriteFrame = { x: 0, y: 0, width: 100, height: 100 };

  it("draws nine pieces: the corners keep their size and the middle stretches", () => {
    const image = fakeImage(100, 100);
    const draws = drawOne(new NineSlice({ image, frame, width: 200, height: 150, insets: { left: 10, top: 10, right: 10, bottom: 10 } }));
    expect(draws).toHaveLength(9);
    // The first piece is the top-left corner, the centre is the middle, and the last is the bottom-right corner.
    expect(draws[0]?.args).toEqual([0, 0, 10, 10, 0, 0, 10, 10]);
    expect(draws[4]?.args).toEqual([10, 10, 80, 80, 10, 10, 180, 130]);
    expect(draws[8]?.args).toEqual([90, 90, 10, 10, 190, 140, 10, 10]);
    // The top edge stretches along the box, and the left edge along its height.
    expect(draws[1]?.args).toEqual([10, 0, 80, 10, 10, 0, 180, 10]);
    expect(draws[3]?.args).toEqual([0, 10, 10, 80, 0, 10, 10, 130]);
  });

  it("draws the whole picture, stretched, when it has no insets", () => {
    const draws = drawOne(new NineSlice({ image: fakeImage(100, 100), frame, width: 300, height: 200 }));
    expect(draws.map((draw) => draw.args)).toEqual([[0, 0, 100, 100, 0, 0, 300, 200]]);
  });

  it("shrinks the corners together when the box is too small for them", () => {
    // Two 30 pixel corners in a 40 wide box: each shrinks to 20, so the middle has no width and is skipped.
    const draws = drawOne(new NineSlice({ image: fakeImage(100, 100), frame, width: 40, height: 100, insets: { left: 30, top: 0, right: 30, bottom: 0 } }));
    expect(draws.map((draw) => draw.args)).toEqual([
      [0, 0, 30, 100, 0, 0, 20, 100],
      [70, 0, 30, 100, 20, 0, 20, 100],
    ]);
  });

  it("refuses a turned frame and a negative inset", () => {
    const image = fakeImage(100, 100);
    expect(() => new NineSlice({ image, frame: { x: 0, y: 0, width: 10, height: 20, rotated: true } })).toThrow(RangeError);
    expect(() => new NineSlice({ image, insets: { left: -1, top: 0, right: 0, bottom: 0 } })).toThrow(RangeError);
  });

  it("draws nothing without an image", () => {
    expect(drawOne(new NineSlice({ width: 50, height: 50 }))).toEqual([]);
  });
});
