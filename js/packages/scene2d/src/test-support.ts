// Test doubles for the scene: a recording 2D context that keeps the current transform, a canvas that holds
// one, images that only have a size, and a stage that the test can resize. happy-dom has no 2D context, so
// the tests draw into the recorder and check the calls, the transform and the state that each draw had. Only
// the tests import this file. The package index does not export it.

import { expect } from "vitest";
import { Matrix2D } from "./math";
import type { Context2D } from "./pass";
import type { Scene2DCanvas } from "./scene";
import type { StageSize, StageSource } from "./stage";

/** A transform as the six numbers of a canvas matrix. */
export type TransformValues = readonly [number, number, number, number, number, number];

/** One call on the recorder, with its arguments. */
export interface RecordedCall {
  readonly name: string;
  readonly args: readonly unknown[];
}

/** A draw: a call that puts paint on the canvas, with the state that was current when it was made. */
export interface RecordedDraw {
  /** The call that drew: "drawImage", "fill", "stroke", "fillText", "strokeText", "fillRect" or "clearRect". */
  readonly op: string;
  /** The numbers of the call, in order (the coordinates and sizes of a draw). */
  readonly args: readonly number[];
  /** The text of a text draw. */
  readonly text: string | undefined;
  /** The image of a drawImage. */
  readonly image: unknown;
  /** The transform in effect. */
  readonly transform: TransformValues;
  readonly alpha: number;
  readonly smoothing: boolean;
  readonly fillStyle: string;
  readonly strokeStyle: string;
  readonly lineWidth: number;
  readonly lineJoin: string;
  readonly font: string;
  readonly shadowBlur: number;
  readonly shadowOffsetX: number;
  readonly shadowOffsetY: number;
}

/**
 * Checks that a list of numbers, such as a transform, matches the expected one to nine decimal places. It is
 * used where a value may be negative zero (which `toEqual` tells apart from zero) or a rounding error.
 */
export function expectNumbers(actual: readonly number[] | undefined, expected: readonly number[]): void {
  expect(actual?.length).toBe(expected.length);
  expected.forEach((value, index) => {
    expect(actual?.[index]).toBeCloseTo(value, 9);
  });
}

/** The size in pixels of a CSS font string, such as "bold 20px sans-serif". Zero when it has none. */
function fontPixels(font: string): number {
  const match = /(\d+(?:\.\d+)?)px/.exec(font);
  return match?.[1] === undefined ? 0 : Number(match[1]);
}

/**
 * A recording 2D context. Each text character is half the font size wide, so the width of a line is easy to
 * predict in a test: `"abc"` in a 20 pixel font is 30 wide. Set `reportsMetrics` to have it report the font's
 * ascent (80 percent of the size) and descent (20 percent).
 */
export class FakeContext2D implements Context2D {
  globalAlpha = 1;
  fillStyle: string | CanvasGradient | CanvasPattern = "#000000";
  strokeStyle: string | CanvasGradient | CanvasPattern = "#000000";
  lineWidth = 1;
  lineJoin: CanvasLineJoin = "miter";
  font = "10px sans-serif";
  textAlign: CanvasTextAlign = "start";
  textBaseline: CanvasTextBaseline = "alphabetic";
  shadowColor = "rgba(0, 0, 0, 0)";
  shadowBlur = 0;
  shadowOffsetX = 0;
  shadowOffsetY = 0;
  imageSmoothingEnabled = true;
  imageSmoothingQuality: ImageSmoothingQuality = "low";
  /** Whether measureText reports the font's ascent and descent. */
  reportsMetrics = false;
  /** Every call, in order, with its arguments. */
  readonly calls: RecordedCall[] = [];
  /** The calls that put paint on the canvas, with their state. */
  readonly draws: RecordedDraw[] = [];
  private readonly matrix = new Matrix2D();

  /** The transform in effect now, as six numbers. */
  get currentTransform(): TransformValues {
    return [this.matrix.a, this.matrix.b, this.matrix.c, this.matrix.d, this.matrix.e, this.matrix.f];
  }

  setTransform(a: number, b: number, c: number, d: number, e: number, f: number): void {
    this.log("setTransform", [a, b, c, d, e, f]);
    this.matrix.set(a, b, c, d, e, f);
  }

  transform(a: number, b: number, c: number, d: number, e: number, f: number): void {
    this.log("transform", [a, b, c, d, e, f]);
    this.matrix.product(this.matrix, new Matrix2D().set(a, b, c, d, e, f));
  }

  clearRect(x: number, y: number, width: number, height: number): void {
    this.log("clearRect", [x, y, width, height]);
    this.paint("clearRect", [x, y, width, height]);
  }

  fillRect(x: number, y: number, width: number, height: number): void {
    this.log("fillRect", [x, y, width, height]);
    this.paint("fillRect", [x, y, width, height]);
  }

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
  ): void {
    this.log("drawImage", [image, sx, sy, sw, sh, dx, dy, dw, dh]);
    this.paint("drawImage", [sx, sy, sw, sh, dx, dy, dw, dh], { image });
  }

  beginPath(): void {
    this.log("beginPath", []);
  }

  closePath(): void {
    this.log("closePath", []);
  }

  moveTo(x: number, y: number): void {
    this.log("moveTo", [x, y]);
  }

  lineTo(x: number, y: number): void {
    this.log("lineTo", [x, y]);
  }

  rect(x: number, y: number, width: number, height: number): void {
    this.log("rect", [x, y, width, height]);
  }

  arc(x: number, y: number, radius: number, startAngle: number, endAngle: number, counterclockwise?: boolean): void {
    this.log("arc", [x, y, radius, startAngle, endAngle, counterclockwise ?? false]);
  }

  arcTo(x1: number, y1: number, x2: number, y2: number, radius: number): void {
    this.log("arcTo", [x1, y1, x2, y2, radius]);
  }

  ellipse(
    x: number,
    y: number,
    radiusX: number,
    radiusY: number,
    rotation: number,
    startAngle: number,
    endAngle: number,
    counterclockwise?: boolean,
  ): void {
    this.log("ellipse", [x, y, radiusX, radiusY, rotation, startAngle, endAngle, counterclockwise ?? false]);
  }

  fill(): void {
    this.log("fill", []);
    this.paint("fill", []);
  }

  stroke(): void {
    this.log("stroke", []);
    this.paint("stroke", []);
  }

  fillText(text: string, x: number, y: number): void {
    this.log("fillText", [text, x, y]);
    this.paint("fillText", [x, y], { text });
  }

  strokeText(text: string, x: number, y: number): void {
    this.log("strokeText", [text, x, y]);
    this.paint("strokeText", [x, y], { text });
  }

  measureText(text: string): {
    readonly width: number;
    readonly fontBoundingBoxAscent?: number;
    readonly fontBoundingBoxDescent?: number;
  } {
    const size = fontPixels(this.font);
    const width = text.length * size * 0.5;
    if (!this.reportsMetrics) return { width };
    return { width, fontBoundingBoxAscent: size * 0.8, fontBoundingBoxDescent: size * 0.2 };
  }

  /** The names of the calls, in order. A short way to check a path or a sequence. */
  get names(): string[] {
    return this.calls.map((call) => call.name);
  }

  /** The draws of the nodes: every draw except the clear and the background fill that the scene makes. */
  get nodeDraws(): RecordedDraw[] {
    return this.draws.filter((draw) => draw.op !== "clearRect" && draw.op !== "fillRect");
  }

  /** Clears the recording, but not the state of the context. */
  clearRecording(): void {
    this.calls.length = 0;
    this.draws.length = 0;
  }

  /** Records a call. */
  private log(name: string, args: readonly unknown[]): void {
    this.calls.push({ name, args });
  }

  /** Records a draw with the state that is current now. */
  private paint(op: string, args: readonly number[], extra: { text?: string; image?: unknown } = {}): void {
    this.draws.push({
      op,
      args,
      text: extra.text,
      image: extra.image,
      transform: this.currentTransform,
      alpha: this.globalAlpha,
      smoothing: this.imageSmoothingEnabled,
      fillStyle: typeof this.fillStyle === "string" ? this.fillStyle : "",
      strokeStyle: typeof this.strokeStyle === "string" ? this.strokeStyle : "",
      lineWidth: this.lineWidth,
      lineJoin: this.lineJoin,
      font: this.font,
      shadowBlur: this.shadowBlur,
      shadowOffsetX: this.shadowOffsetX,
      shadowOffsetY: this.shadowOffsetY,
    });
  }
}

/** A canvas that holds a recording context. It is a scene canvas. */
export class FakeCanvas implements Scene2DCanvas {
  width: number;
  height: number;
  /** The context that every call of getContext gives. */
  readonly context = new FakeContext2D();

  /** @param width - the starting backing-store width. @param height - the starting backing-store height. */
  constructor(width = 1920, height = 1080) {
    this.width = width;
    this.height = height;
  }

  getContext(_contextId: "2d"): FakeContext2D {
    return this.context;
  }
}

/** An image element of the given natural size, which has loaded. */
export function fakeImage(width: number, height: number): HTMLImageElement {
  return { naturalWidth: width, naturalHeight: height, width, height } as unknown as HTMLImageElement;
}

/** A canvas-like image of the given size. */
export function fakeCanvasImage(width: number, height: number): HTMLCanvasElement {
  return { width, height } as unknown as HTMLCanvasElement;
}

/** A stage that the test resizes. It has the members that `followStage` uses. */
export class FakeStage implements StageSource {
  pixelWidth: number;
  pixelHeight: number;
  private readonly listeners = new Set<(layout: StageSize) => void>();

  constructor(pixelWidth: number, pixelHeight: number) {
    this.pixelWidth = pixelWidth;
    this.pixelHeight = pixelHeight;
  }

  /** How many listeners are attached now. */
  get listenerCount(): number {
    return this.listeners.size;
  }

  onResize(listener: (layout: StageSize) => void): () => void {
    this.listeners.add(listener);
    return () => {
      this.listeners.delete(listener);
    };
  }

  /** Changes the pixel size and tells the listeners, the way the real stage does. */
  resize(pixelWidth: number, pixelHeight: number): void {
    this.pixelWidth = pixelWidth;
    this.pixelHeight = pixelHeight;
    for (const listener of [...this.listeners]) listener({ pixelWidth, pixelHeight });
  }
}
