// Sprites: a picture drawn at a place in the scene. The picture is an image element, an image bitmap or
// a canvas. A sprite may show only part of it, a frame: a rectangle of the image that an atlas names. A
// frame can be stored turned 90 degrees clockwise, and can be trimmed of transparent edges. A trimmed
// picture is drawn back at its place inside the untrimmed box, so that an animation keeps its pivot.

import type { MutableRect } from "./math";
import { Node, type NodeOptions } from "./node";
import type { DrawPass } from "./pass";

/** A picture that a sprite or a nine-slice can draw: an image element, an image bitmap or a canvas. */
export type SpriteImage = HTMLImageElement | ImageBitmap | HTMLCanvasElement | OffscreenCanvas;

/**
 * A rectangle of an image, as an atlas or an Aseprite export describes one.
 *
 * `x`, `y`, `width` and `height` are the stored rectangle, in image pixels. A turned frame is stored
 * rotated 90 degrees clockwise, so its stored width is the height of the picture. `trimX` and `trimY`
 * place the trimmed picture inside the untrimmed size, `sourceWidth` by `sourceHeight`. Each of the
 * last four defaults to what the rectangle gives: no trim, and the picture's own size.
 */
export interface SpriteFrame {
  /** A name, for tools and for finding the frame again. */
  readonly name?: string;
  readonly x: number;
  readonly y: number;
  readonly width: number;
  readonly height: number;
  /** True when the image stores the picture turned 90 degrees clockwise. */
  readonly rotated?: boolean;
  /** Where the trimmed picture starts inside the untrimmed box. */
  readonly trimX?: number;
  readonly trimY?: number;
  /** The untrimmed size of the picture. */
  readonly sourceWidth?: number;
  readonly sourceHeight?: number;
}

/**
 * Where a frame sits on an image, and how its picture sits in the untrimmed box. It is mutable, so that a
 * node can keep one and rewrite it each time it is needed.
 */
export interface Placement {
  /** The stored rectangle, in image pixels. */
  x: number;
  y: number;
  width: number;
  height: number;
  /** Whether the stored picture is turned 90 degrees clockwise. */
  rotated: boolean;
  /** The trimmed picture as it reads on the screen, with any turn undone. */
  pictureWidth: number;
  pictureHeight: number;
  /** The untrimmed box. */
  sourceWidth: number;
  sourceHeight: number;
  /** Where the trimmed picture starts inside the untrimmed box. */
  trimX: number;
  trimY: number;
}

/** A placement with every number at zero. */
export function newPlacement(): Placement {
  return {
    x: 0,
    y: 0,
    width: 0,
    height: 0,
    rotated: false,
    pictureWidth: 0,
    pictureHeight: 0,
    sourceWidth: 0,
    sourceHeight: 0,
    trimX: 0,
    trimY: 0,
  };
}

/**
 * Writes the placement of a frame on an image into `target`, and returns it. Without a frame, the whole
 * image is the picture.
 */
export function fillPlacement(target: Placement, image: SpriteImage, frame: SpriteFrame | null): Placement {
  const imageWidth = naturalWidthOf(image);
  const imageHeight = naturalHeightOf(image);
  if (frame === null) {
    target.x = 0;
    target.y = 0;
    target.width = imageWidth;
    target.height = imageHeight;
    target.rotated = false;
    target.pictureWidth = imageWidth;
    target.pictureHeight = imageHeight;
    target.sourceWidth = imageWidth;
    target.sourceHeight = imageHeight;
    target.trimX = 0;
    target.trimY = 0;
    return target;
  }
  const rotated = frame.rotated === true;
  const pictureWidth = rotated ? frame.height : frame.width;
  const pictureHeight = rotated ? frame.width : frame.height;
  target.x = frame.x;
  target.y = frame.y;
  target.width = frame.width;
  target.height = frame.height;
  target.rotated = rotated;
  target.pictureWidth = pictureWidth;
  target.pictureHeight = pictureHeight;
  target.sourceWidth = frame.sourceWidth ?? pictureWidth;
  target.sourceHeight = frame.sourceHeight ?? pictureHeight;
  target.trimX = frame.trimX ?? 0;
  target.trimY = frame.trimY ?? 0;
  return target;
}

/** The width of an image in its own pixels. An image element that has not loaded has none. */
function naturalWidthOf(image: SpriteImage): number {
  return "naturalWidth" in image ? image.naturalWidth : image.width;
}

/** The height of an image in its own pixels. An image element that has not loaded has none. */
function naturalHeightOf(image: SpriteImage): number {
  return "naturalWidth" in image ? image.naturalHeight : image.height;
}

/** The options that the picture nodes share. */
export interface ImageNodeOptions extends NodeOptions {
  /** The picture. Null, the default, draws nothing. */
  image?: SpriteImage | null;
  /** The part of the picture to draw. Null, the default, draws all of it. */
  frame?: SpriteFrame | null;
  /** The size of the box, in logical pixels. Null, the default, is the untrimmed size of the frame. */
  width?: number | null;
  height?: number | null;
}

/**
 * The base of the picture nodes. It holds the image, the frame and the box, and it works out where the
 * picture goes in the box. Use {@link Sprite} or {@link NineSlice}.
 */
export abstract class ImageNode extends Node {
  /** The picture to draw. Null draws nothing. */
  image: SpriteImage | null;
  /** The box width in logical pixels. Null means the untrimmed width of the frame. */
  width: number | null;
  /** The box height in logical pixels. Null means the untrimmed height of the frame. */
  height: number | null;
  private frameValue: SpriteFrame | null = null;
  /** The placement of the current frame, rewritten each time it is needed. */
  protected readonly placed: Placement = newPlacement();
  private readonly box: MutableRect = { x: 0, y: 0, width: 0, height: 0 };

  /** @param options - see {@link ImageNodeOptions}. */
  constructor(options: ImageNodeOptions = {}) {
    super(options);
    this.image = options.image ?? null;
    this.width = options.width ?? null;
    this.height = options.height ?? null;
    this.frame = options.frame ?? null;
  }

  /** The part of the picture to draw, or null for all of it. Setting it checks the frame. */
  get frame(): SpriteFrame | null {
    return this.frameValue;
  }

  set frame(value: SpriteFrame | null) {
    if (value !== null) this.checkFrame(value);
    this.frameValue = value;
  }

  /** Refuses a frame that the node cannot draw, by throwing a RangeError. Subclasses override it. */
  protected checkFrame(_frame: SpriteFrame): void {}

  /** The placement of the current frame, or null when there is no image. */
  protected placementNow(): Placement | null {
    const image = this.image;
    if (image === null) return null;
    return fillPlacement(this.placed, image, this.frameValue);
  }

  protected override localBox(out: MutableRect): void {
    const placement = this.placementNow();
    out.x = 0;
    out.y = 0;
    out.width = placement === null ? 0 : (this.width ?? placement.sourceWidth);
    out.height = placement === null ? 0 : (this.height ?? placement.sourceHeight);
  }

  protected override contentRect(out: MutableRect): boolean {
    if (this.image === null) return false;
    this.localBox(out);
    return true;
  }

  protected override hitGeometry(x: number, y: number): boolean {
    this.localBox(this.box);
    return x >= 0 && x <= this.box.width && y >= 0 && y <= this.box.height;
  }
}

/** Options for {@link Sprite}. */
export interface SpriteOptions extends ImageNodeOptions {
  /** Mirrors the picture from left to right, about the middle of the box. Default false. */
  flipX?: boolean;
  /** Mirrors the picture from top to bottom, about the middle of the box. Default false. */
  flipY?: boolean;
}

/** A picture, or a frame of one, at a place in the scene. */
export class Sprite extends ImageNode {
  /** Mirrors the picture from left to right. */
  flipX: boolean;
  /** Mirrors the picture from top to bottom. */
  flipY: boolean;

  /** @param options - see {@link SpriteOptions}. */
  constructor(options: SpriteOptions = {}) {
    super(options);
    this.flipX = options.flipX ?? false;
    this.flipY = options.flipY ?? false;
  }

  protected override drawSelf(pass: DrawPass, alpha: number, pixelArt: boolean): void {
    const image = this.image;
    const p = this.placementNow();
    if (image === null || p === null) return;
    if (p.width <= 0 || p.height <= 0 || p.sourceWidth <= 0 || p.sourceHeight <= 0) return;
    const boxWidth = this.width ?? p.sourceWidth;
    const boxHeight = this.height ?? p.sourceHeight;
    // The scale from the untrimmed box to the box on the screen, and the place of the trimmed picture.
    const sx = boxWidth / p.sourceWidth;
    const sy = boxHeight / p.sourceHeight;
    const dx = p.trimX * sx;
    const dy = p.trimY * sy;
    const ctx = pass.context;
    pass.place(this.surfaceMatrix);
    if (this.flipX || this.flipY) {
      ctx.transform(
        this.flipX ? -1 : 1,
        0,
        0,
        this.flipY ? -1 : 1,
        this.flipX ? boxWidth : 0,
        this.flipY ? boxHeight : 0,
      );
    }
    pass.alpha(alpha);
    pass.smooth(!pixelArt);
    if (p.rotated) {
      // The stored picture is turned clockwise, so a stored point (u, v) is the picture point (v, height - u).
      // This transform puts it back at (dx + sx * v, dy + sy * (height - u)).
      ctx.transform(0, -sy, sx, 0, dx, dy + sy * p.pictureHeight);
      ctx.drawImage(image, p.x, p.y, p.width, p.height, 0, 0, p.width, p.height);
    } else {
      ctx.drawImage(image, p.x, p.y, p.width, p.height, dx, dy, p.pictureWidth * sx, p.pictureHeight * sy);
    }
    pass.count();
  }
}
