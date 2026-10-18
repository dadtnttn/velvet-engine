// Tests for the atlas readers, with small JSON fixtures: TexturePacker in the hash and the array form, a turned and
// trimmed frame, the image size, and the errors for a broken file. Aseprite in the array and the hash form, its
// frame durations (milliseconds in the file, seconds here), and its frame tags with their directions. A tag feeds
// an animation directly.

import { describe, expect, it } from "vitest";
import { createAnimation } from "./animation";
import { parseAseprite, parseTexturePacker } from "./atlas";
import { Sprite } from "./sprite";
import { fakeImage } from "./test-support";

/** A TexturePacker sheet in the hash form: one trimmed frame, and one turned frame. */
const hashSheet = {
  frames: {
    "hero_idle.png": {
      frame: { x: 0, y: 0, w: 32, h: 32 },
      rotated: false,
      trimmed: true,
      spriteSourceSize: { x: 4, y: 2, w: 20, h: 24 },
      sourceSize: { w: 32, h: 32 },
    },
    // A 24 by 10 picture stored turned clockwise, so the stored rectangle is 10 wide and 24 tall.
    "rock.png": {
      frame: { x: 32, y: 0, w: 10, h: 24 },
      rotated: true,
      trimmed: false,
      spriteSourceSize: { x: 0, y: 0, w: 24, h: 10 },
      sourceSize: { w: 24, h: 10 },
    },
  },
  meta: { app: "TexturePacker", size: { w: 64, h: 32 } },
};

/** The same kind of sheet in the array form. */
const arraySheet = {
  frames: [
    {
      filename: "coin_0",
      frame: { x: 0, y: 0, w: 8, h: 8 },
      rotated: false,
      trimmed: false,
      spriteSourceSize: { x: 0, y: 0, w: 8, h: 8 },
      sourceSize: { w: 8, h: 8 },
    },
    {
      filename: "coin_1",
      frame: { x: 8, y: 0, w: 8, h: 8 },
      rotated: false,
      trimmed: false,
      spriteSourceSize: { x: 0, y: 0, w: 8, h: 8 },
      sourceSize: { w: 8, h: 8 },
    },
  ],
};

/** An Aseprite export in the array form, with three tags: forward, ping-pong and reverse. */
const asepriteSheet = {
  frames: [
    { filename: "knight 0", frame: { x: 0, y: 0, w: 16, h: 16 }, rotated: false, trimmed: false, spriteSourceSize: { x: 0, y: 0, w: 16, h: 16 }, sourceSize: { w: 16, h: 16 }, duration: 100 },
    { filename: "knight 1", frame: { x: 16, y: 0, w: 16, h: 16 }, rotated: false, trimmed: false, spriteSourceSize: { x: 0, y: 0, w: 16, h: 16 }, sourceSize: { w: 16, h: 16 }, duration: 200 },
    { filename: "knight 2", frame: { x: 32, y: 0, w: 16, h: 16 }, rotated: false, trimmed: false, spriteSourceSize: { x: 0, y: 0, w: 16, h: 16 }, sourceSize: { w: 16, h: 16 }, duration: 100 },
    { filename: "knight 3", frame: { x: 48, y: 0, w: 16, h: 16 }, rotated: false, trimmed: false, spriteSourceSize: { x: 0, y: 0, w: 16, h: 16 }, sourceSize: { w: 16, h: 16 }, duration: 100 },
  ],
  meta: {
    app: "Aseprite",
    size: { w: 64, h: 16 },
    frameTags: [
      { name: "idle", from: 0, to: 1, direction: "forward" },
      { name: "walk", from: 1, to: 3, direction: "pingpong" },
      { name: "back", from: 0, to: 2, direction: "reverse" },
    ],
  },
};

describe("TexturePacker", () => {
  it("reads the hash form, with the trim, the turn and the image size", () => {
    const atlas = parseTexturePacker(hashSheet);
    expect(atlas.count).toBe(2);
    expect(atlas.names).toEqual(["hero_idle.png", "rock.png"]);
    expect(atlas.size).toEqual({ width: 64, height: 32 });
    expect(atlas.frame("hero_idle.png")).toEqual({
      name: "hero_idle.png",
      x: 0,
      y: 0,
      width: 32,
      height: 32,
      rotated: false,
      trimX: 4,
      trimY: 2,
      sourceWidth: 32,
      sourceHeight: 32,
    });
  });

  it("keeps a turned frame as it is stored, with the picture's size in the source size", () => {
    const rock = parseTexturePacker(hashSheet).frame("rock.png");
    expect(rock).toMatchObject({ x: 32, y: 0, width: 10, height: 24, rotated: true, sourceWidth: 24, sourceHeight: 10 });
  });

  it("reads the array form, keyed by file name", () => {
    const atlas = parseTexturePacker(arraySheet);
    expect(atlas.names).toEqual(["coin_0", "coin_1"]);
    expect(atlas.size).toBeNull();
    expect(atlas.get("coin_1")).toMatchObject({ x: 8, y: 0, width: 8, height: 8, rotated: false });
    expect(atlas.get("missing")).toBeUndefined();
  });

  it("reads the JSON text as well as the parsed object", () => {
    expect(parseTexturePacker(JSON.stringify(arraySheet)).count).toBe(2);
  });

  it("draws a frame of the atlas with a sprite", () => {
    const frame = parseTexturePacker(arraySheet).frame("coin_1");
    const sprite = new Sprite({ image: fakeImage(64, 8), frame });
    expect(sprite.frame).toEqual(frame);
  });

  it("refuses a missing frame name, a duplicate name and a broken shape", () => {
    expect(() => parseTexturePacker(hashSheet).frame("nope")).toThrow(RangeError);
    const twice = { frames: [arraySheet.frames[0], arraySheet.frames[0]] };
    expect(() => parseTexturePacker(twice)).toThrow(RangeError);
    expect(() => parseTexturePacker({ frames: { a: { frame: { x: 0, y: 0, w: 1 } } } })).toThrow(TypeError);
    expect(() => parseTexturePacker({ frames: { a: { frame: { x: 0, y: 0, w: -1, h: 1 } } } })).toThrow(RangeError);
    expect(() => parseTexturePacker({})).toThrow(TypeError);
  });
});

describe("Aseprite", () => {
  it("reads every frame, with its duration in seconds", () => {
    const sheet = parseAseprite(asepriteSheet);
    expect(sheet.frames.map((frame) => frame.name)).toEqual(["knight 0", "knight 1", "knight 2", "knight 3"]);
    expect(sheet.durations).toEqual([0.1, 0.2, 0.1, 0.1]);
    expect(sheet.size).toEqual({ width: 64, height: 16 });
  });

  it("reads the frame tags, with the frames and durations of each", () => {
    const sheet = parseAseprite(asepriteSheet);
    const walk = sheet.tag("walk");
    expect(walk).toMatchObject({ name: "walk", from: 1, to: 3, direction: "pingpong" });
    expect(walk.frames.map((frame) => frame.x)).toEqual([16, 32, 48]);
    expect(walk.durations).toEqual([0.2, 0.1, 0.1]);
    expect(sheet.tag("back").direction).toBe("reverse");
    expect(sheet.tags.size).toBe(3);
  });

  it("plays a tag as an animation, with its durations and direction", () => {
    const sheet = parseAseprite(asepriteSheet);
    const back = sheet.tag("back");
    const animation = createAnimation(back.frames, { durations: back.durations, direction: back.direction });
    // The tag runs 0, 1, 2 in reverse: 2, 1, 0, with 0.1, 0.2 and 0.1 seconds.
    expect(animation.index).toBe(2);
    animation.tick(0.1);
    expect(animation.index).toBe(1);
    animation.tick(0.2);
    expect(animation.index).toBe(0);
  });

  it("reads the hash form, keyed by file name", () => {
    const hash = {
      frames: {
        "orb 0.ase": { frame: { x: 0, y: 0, w: 4, h: 4 }, rotated: false, trimmed: false, spriteSourceSize: { x: 0, y: 0, w: 4, h: 4 }, sourceSize: { w: 4, h: 4 }, duration: 50 },
      },
    };
    const sheet = parseAseprite(hash);
    expect(sheet.frames[0]?.name).toBe("orb 0.ase");
    expect(sheet.durations).toEqual([0.05]);
    expect(sheet.tags.size).toBe(0);
  });

  it("refuses a tag outside the frames, an unknown direction, and a duplicate tag", () => {
    const outside = { ...asepriteSheet, meta: { frameTags: [{ name: "x", from: 2, to: 9, direction: "forward" }] } };
    expect(() => parseAseprite(outside)).toThrow(RangeError);
    const direction = { ...asepriteSheet, meta: { frameTags: [{ name: "x", from: 0, to: 1, direction: "sideways" }] } };
    expect(() => parseAseprite(direction)).toThrow(TypeError);
    const twice = {
      ...asepriteSheet,
      meta: { frameTags: [{ name: "x", from: 0, to: 1, direction: "forward" }, { name: "x", from: 2, to: 3, direction: "forward" }] },
    };
    expect(() => parseAseprite(twice)).toThrow(RangeError);
    expect(() => parseAseprite(asepriteSheet).tag("nope")).toThrow(RangeError);
  });
});
