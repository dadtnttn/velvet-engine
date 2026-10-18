// The scene: a retained tree of layers and nodes, drawn into a canvas in logical units.
//
// A scene has the logical size of its authoring surface (1920 by 1080 unless the game says otherwise), and
// it draws at the pixel size of its canvas. `resize`, or `followStage` for a ui Stage, gives it the screen's
// own pixels: the scale `k` is the number of device pixels per logical pixel, and every node is drawn at that
// scale. Image smoothing is bilinear unless a node is pixel art.
//
// A frame is `update` and then `render`. `update` computes the matrices of every node from the camera, and
// `render` draws the layers in order. Hit tests run the same update first, so they see the current positions.
// `tick(dt)` moves the camera, the follow and the shake, and it obeys the game clock when one is given.

import type { GameClock } from "@velvet/core";
import { Camera2D } from "./camera";
import { Layer, type LayerOptions } from "./layer";
import { isPositiveFinite, Matrix2D, type Point2D } from "./math";
import type { Node } from "./node";
import { DrawPass, type Context2D, type TextMeasure } from "./pass";

/** The part of a canvas that the scene uses: its size, which the scene sets, and its 2D context. */
export interface Scene2DCanvas {
  /** The width of the backing store in device pixels. */
  width: number;
  /** The height of the backing store in device pixels. */
  height: number;
  /** The 2D context of the canvas. */
  getContext(contextId: "2d"): Context2D | null;
}

/** Options for {@link createScene2D}. */
export interface Scene2DOptions {
  /** The canvas to draw into. An HTML canvas and an offscreen canvas both work. */
  canvas: Scene2DCanvas;
  /** The logical size of the surface. Default 1920 by 1080. */
  width?: number;
  height?: number;
  /** A colour that fills the canvas before each frame. Null, the default, clears it to transparent. */
  background?: string | null;
  /** The quality of image smoothing. "low" is bilinear, the default. */
  smoothingQuality?: ImageSmoothingQuality;
  /** A game clock. The camera's shake and follow obey its pause and speed. */
  clock?: GameClock;
  /** The seed of the camera's shake. Default 1. */
  seed?: number;
  /** A clock in milliseconds, for the render time in the stats. Default performance.now. */
  now?: () => number;
}

/** What the last render did, for the devtools overlay. */
export interface SceneStats {
  /** Nodes that drew something. */
  readonly nodes: number;
  /** Subtrees skipped because they lie off the surface. */
  readonly culled: number;
  /** Draw calls that the nodes issued. */
  readonly drawCalls: number;
  /** Milliseconds that the last render took. */
  readonly renderMs: number;
}

/** The logical size of the authoring surface when the options name none. */
const DEFAULT_WIDTH = 1920;
const DEFAULT_HEIGHT = 1080;

/** A clock in milliseconds: `performance.now` where the host has it. */
function defaultNow(): number {
  return typeof performance === "undefined" ? Date.now() : performance.now();
}

/** A retained 2D scene. Create it with {@link createScene2D}. */
export class Scene2D {
  /** The logical width of the surface. */
  readonly width: number;
  /** The logical height of the surface. */
  readonly height: number;
  /** The camera. It moves the layers according to their parallax factors. */
  readonly camera: Camera2D;
  private readonly surface: Scene2DCanvas;
  private readonly context: Context2D;
  private readonly pass: DrawPass;
  private readonly now: () => number;
  private readonly background: string | null;
  private readonly quality: ImageSmoothingQuality;
  private readonly measurer: TextMeasure;
  private readonly layerList: Layer[] = [];
  private readonly sortedLayers: Layer[] = [];
  private readonly identity = new Matrix2D();
  private scaleValue = 1;
  private latest: SceneStats = { nodes: 0, culled: 0, drawCalls: 0, renderMs: 0 };

  /**
   * @param options - see {@link Scene2DOptions}.
   * @throws RangeError when the size is not positive, and Error when the canvas has no 2D context.
   */
  constructor(options: Scene2DOptions) {
    const width = options.width ?? DEFAULT_WIDTH;
    const height = options.height ?? DEFAULT_HEIGHT;
    if (!isPositiveFinite(width)) throw new RangeError(`width must be a positive number, got ${width}`);
    if (!isPositiveFinite(height)) throw new RangeError(`height must be a positive number, got ${height}`);
    const context = options.canvas.getContext("2d");
    if (context === null) throw new Error("the canvas gave no 2D context");
    this.width = width;
    this.height = height;
    this.surface = options.canvas;
    this.context = context;
    this.pass = new DrawPass(context);
    this.background = options.background ?? null;
    this.quality = options.smoothingQuality ?? "low";
    this.now = options.now ?? defaultNow;
    this.camera = new Camera2D({ width, height, clock: options.clock, seed: options.seed });
    this.measurer = (text, font) => {
      this.context.font = font;
      const metrics = this.context.measureText(text);
      return {
        width: metrics.width,
        ascent: metrics.fontBoundingBoxAscent,
        descent: metrics.fontBoundingBoxDescent,
      };
    };
    // The backing store starts at the logical size. `resize` or `followStage` changes it.
    this.resize(width, height);
  }

  /** The canvas that the scene draws into. */
  get canvas(): Scene2DCanvas {
    return this.surface;
  }

  /** The device pixels per logical pixel: the scale of the drawing. */
  get pixelScale(): number {
    return this.scaleValue;
  }

  /** The layers in drawing order: by zIndex, then in the order they were created. */
  get layers(): readonly Layer[] {
    return [...this.orderedLayers()];
  }

  /**
   * Adds a layer on top of the others, in creation order.
   * @returns the new layer.
   */
  createLayer(options: LayerOptions = {}): Layer {
    const layer = new Layer(options);
    this.layerList.push(layer);
    return layer;
  }

  /** Takes a layer out of the scene. Its nodes stay in the layer. */
  removeLayer(layer: Layer): void {
    const index = this.layerList.indexOf(layer);
    if (index >= 0) this.layerList.splice(index, 1);
  }

  /**
   * Sets the backing store to a pixel size, and the scale to match: a logical pixel is `pixelWidth` over
   * `width` device pixels. A stage gives the size of its presented area.
   * @throws RangeError when a size is not positive.
   */
  resize(pixelWidth: number, pixelHeight: number): void {
    if (!isPositiveFinite(pixelWidth) || !isPositiveFinite(pixelHeight)) {
      throw new RangeError(`the pixel size must be positive, got ${pixelWidth} by ${pixelHeight}`);
    }
    const widthPixels = Math.max(1, Math.round(pixelWidth));
    const heightPixels = Math.max(1, Math.round(pixelHeight));
    if (this.surface.width !== widthPixels) this.surface.width = widthPixels;
    if (this.surface.height !== heightPixels) this.surface.height = heightPixels;
    this.scaleValue = widthPixels / this.width;
  }

  /**
   * Sets the scale directly: the backing store becomes the logical size times `scale`, rounded to whole pixels.
   * The scale that results is the one that `pixelScale` then gives.
   * @throws RangeError when `scale` is not positive.
   */
  setPixelScale(scale: number): void {
    if (!isPositiveFinite(scale)) throw new RangeError(`the pixel scale must be positive, got ${scale}`);
    this.resize(this.width * scale, this.height * scale);
  }

  /**
   * Moves the camera by a real step of `dt` seconds, scaled by the clock. A camera that follows a target
   * first takes the target's position from the last update.
   * @throws RangeError when `dt` is negative or not finite.
   */
  tick(dt: number): void {
    if (this.camera.followTarget !== null) this.update();
    this.camera.tick(dt);
  }

  /**
   * Computes the matrices and culling boxes of every node, from the camera as it is now. `render` and the hit
   * tests call it; a game calls it only to read positions before a render.
   */
  update(): void {
    for (const layer of this.orderedLayers()) {
      this.camera.viewFor(layer.parallax, layer.viewMatrix);
      layer.updateTree(layer.viewMatrix, this.identity, this.measurer);
    }
  }

  /** Draws one frame: the update, then the layers in order. It sets the stats of the frame. */
  render(): void {
    const started = this.now();
    this.update();
    const ctx = this.context;
    const pass = this.pass;
    pass.begin(this.scaleValue, this.quality, this.width, this.height);
    ctx.setTransform(1, 0, 0, 1, 0, 0);
    const pixelWidth = this.surface.width;
    const pixelHeight = this.surface.height;
    if (this.background !== null) {
      ctx.globalAlpha = 1;
      ctx.fillStyle = this.background;
      ctx.fillRect(0, 0, pixelWidth, pixelHeight);
    } else {
      ctx.clearRect(0, 0, pixelWidth, pixelHeight);
    }
    for (const layer of this.orderedLayers()) layer.drawTree(pass, 1, false);
    this.latest = {
      nodes: pass.nodes,
      culled: pass.culled,
      drawCalls: pass.calls,
      renderMs: this.now() - started,
    };
  }

  /**
   * The topmost visible, interactive node under a point of the surface, or null. The point is in logical
   * pixels: a pointer position from `Stage.toLogical`. A node is hit only when it and its parents are
   * visible and their opacity is above zero.
   */
  hitTest(point: Point2D): Node | null {
    return this.nodesAt(point)[0] ?? null;
  }

  /** Every visible, interactive node under a point of the surface, topmost first. */
  nodesAt(point: Point2D): Node[] {
    this.update();
    const hits: Node[] = [];
    for (const layer of this.orderedLayers()) layer.collectAt(point.x, point.y, 1, hits);
    return hits.reverse();
  }

  /** What the last render did. A copy: reading it again gives the same numbers. */
  stats(): SceneStats {
    return { ...this.latest };
  }

  /** Sorts the layers into drawing order, in a buffer that the scene reuses. */
  private orderedLayers(): Layer[] {
    this.sortedLayers.length = 0;
    for (const layer of this.layerList) this.sortedLayers.push(layer);
    this.sortedLayers.sort(compareLayers);
    return this.sortedLayers;
  }
}

/** Orders two layers by zIndex. The sort is stable, so layers with equal zIndex keep their creation order. */
function compareLayers(a: Layer, b: Layer): number {
  return a.zIndex - b.zIndex;
}

/**
 * Makes a scene. It draws into the canvas at its logical size until `resize` or `followStage` gives it the
 * native pixel size.
 *
 * @example
 * const scene = createScene2D({ canvas, width: 1920, height: 1080, clock });
 * const world = scene.createLayer({ name: "world", parallax: 1 });
 * const hud = scene.createLayer({ name: "hud", parallax: 0 });
 * world.add(new Sprite({ image, x: 100, y: 200 }));
 * followStage(scene, stage);
 * scene.camera.follow(player, { lerp: 0.2, deadzone: { width: 200, height: 120 } });
 * // In the game loop:
 * scene.tick(dt);
 * scene.render();
 * @throws RangeError when the size is not positive, and Error when the canvas has no 2D context.
 */
export function createScene2D(options: Scene2DOptions): Scene2D {
  return new Scene2D(options);
}
