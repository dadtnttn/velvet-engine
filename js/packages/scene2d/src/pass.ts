// The render pass: the context that nodes draw into, and the counts that the stats report.
//
// A node asks the pass to put the context on its surface matrix, in device pixels, to set its alpha
// and its smoothing, and then to draw. The pass knows the device-pixel scale, so no node needs the
// canvas size. Smoothing is written only when it changes, so a run of pictures sets it once.

import type { Matrix2D } from "./math";

/** What a text measurement gives: the advance width, and the font's ascent and descent when the context reports them. */
export interface MeasuredText {
  readonly width: number;
  readonly ascent?: number;
  readonly descent?: number;
}

/** Measures a line of text in a font. The font is a CSS font string, such as `"bold 20px sans-serif"`. */
export type TextMeasure = (text: string, font: string) => MeasuredText;

/**
 * The part of a 2D canvas context that the scene draws with. A `CanvasRenderingContext2D` and an
 * `OffscreenCanvasRenderingContext2D` both have all of it, and so does the recording fake in the tests.
 */
export interface Context2D {
  globalAlpha: number;
  fillStyle: string | CanvasGradient | CanvasPattern;
  strokeStyle: string | CanvasGradient | CanvasPattern;
  lineWidth: number;
  lineJoin: CanvasLineJoin;
  font: string;
  textAlign: CanvasTextAlign;
  textBaseline: CanvasTextBaseline;
  shadowColor: string;
  shadowBlur: number;
  shadowOffsetX: number;
  shadowOffsetY: number;
  imageSmoothingEnabled: boolean;
  imageSmoothingQuality: ImageSmoothingQuality;
  setTransform(a: number, b: number, c: number, d: number, e: number, f: number): void;
  transform(a: number, b: number, c: number, d: number, e: number, f: number): void;
  clearRect(x: number, y: number, width: number, height: number): void;
  fillRect(x: number, y: number, width: number, height: number): void;
  drawImage(
    image: CanvasImageSource,
    sx: number,
    sy: number,
    sw: number,
    sh: number,
    dx: number,
    dy: number,
    dw: number,
    dh: number,
  ): void;
  beginPath(): void;
  closePath(): void;
  moveTo(x: number, y: number): void;
  lineTo(x: number, y: number): void;
  rect(x: number, y: number, width: number, height: number): void;
  arc(x: number, y: number, radius: number, startAngle: number, endAngle: number, counterclockwise?: boolean): void;
  arcTo(x1: number, y1: number, x2: number, y2: number, radius: number): void;
  ellipse(
    x: number,
    y: number,
    radiusX: number,
    radiusY: number,
    rotation: number,
    startAngle: number,
    endAngle: number,
    counterclockwise?: boolean,
  ): void;
  fill(): void;
  stroke(): void;
  fillText(text: string, x: number, y: number): void;
  strokeText(text: string, x: number, y: number): void;
  measureText(text: string): {
    readonly width: number;
    readonly fontBoundingBoxAscent?: number;
    readonly fontBoundingBoxDescent?: number;
  };
}

/**
 * One render: the context it draws into, the device-pixel scale, the surface size, and the counts that
 * the stats report. The scene keeps one pass and reuses it for every frame.
 */
export class DrawPass {
  /** The context that the nodes draw into. */
  readonly context: Context2D;
  /** The surface width in logical pixels. Subtrees outside it are culled. */
  surfaceWidth = 0;
  /** The surface height in logical pixels. */
  surfaceHeight = 0;
  /** Draw calls that the nodes issued in this render. */
  calls = 0;
  /** Nodes that drew something in this render. */
  nodes = 0;
  /** Subtrees skipped in this render because they lie off the surface. */
  culled = 0;
  private pixelScale = 1;
  private smoothing: boolean | null = null;

  /** @param context - the context to draw into. */
  constructor(context: Context2D) {
    this.context = context;
  }

  /** The device pixels per logical pixel. */
  get scale(): number {
    return this.pixelScale;
  }

  /**
   * Starts a render. It stores the scale and the surface size, sets the smoothing quality, and zeroes
   * the counts.
   */
  begin(scale: number, quality: ImageSmoothingQuality, width: number, height: number): void {
    this.pixelScale = scale;
    this.surfaceWidth = width;
    this.surfaceHeight = height;
    this.calls = 0;
    this.nodes = 0;
    this.culled = 0;
    this.smoothing = null;
    this.context.imageSmoothingQuality = quality;
  }

  /** Puts the context on a node's surface matrix, scaled from logical pixels to device pixels. */
  place(surface: Readonly<Matrix2D>): void {
    const k = this.pixelScale;
    this.context.setTransform(
      k * surface.a,
      k * surface.b,
      k * surface.c,
      k * surface.d,
      k * surface.e,
      k * surface.f,
    );
  }

  /** Sets the alpha that the draws after it use. */
  alpha(value: number): void {
    this.context.globalAlpha = value;
  }

  /** Turns image smoothing on (bilinear, at the chosen quality) or off for pixel art. It writes only on a change. */
  smooth(on: boolean): void {
    if (this.smoothing === on) return;
    this.smoothing = on;
    this.context.imageSmoothingEnabled = on;
  }

  /** Counts one draw call. */
  count(): void {
    this.calls += 1;
  }
}
