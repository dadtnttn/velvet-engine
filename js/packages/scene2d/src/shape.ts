// Shapes: rectangles, rounded rectangles, circles, ellipses, lines and polygons, filled and stroked. A
// shape's own box starts at its origin: a rectangle spans from (0, 0) to (width, height), and a circle
// has its centre at (radius, radius). A line or a polygon uses its points as they are.

import type { Context2D, DrawPass } from "./pass";
import { clamp, isNonNegativeFinite, type MutableRect, type Point2D } from "./math";
import { Node, type NodeOptions } from "./node";

/** The smallest distance, in logical pixels, that a line can be hit from. It keeps thin lines clickable. */
const LINE_HIT_MINIMUM = 2;

/** The geometry of a shape, in the shape's own units. */
export type ShapeGeometry =
  | { readonly kind: "rect"; readonly width: number; readonly height: number }
  | { readonly kind: "roundRect"; readonly width: number; readonly height: number; readonly radius: number }
  | { readonly kind: "circle"; readonly radius: number }
  | { readonly kind: "ellipse"; readonly radiusX: number; readonly radiusY: number }
  | { readonly kind: "line"; readonly points: readonly Point2D[] }
  | { readonly kind: "polygon"; readonly points: readonly Point2D[] };

/** Options for {@link Shape}. */
export interface ShapeOptions extends NodeOptions {
  /** The shape to draw. */
  geometry: ShapeGeometry;
  /** The fill colour, or null for no fill. Default "#ffffff". A line is never filled. */
  fill?: string | null;
  /** The stroke colour, or null for no stroke. Default null. */
  stroke?: string | null;
  /** The stroke width in logical pixels. Default 1. */
  lineWidth?: number;
}

/** A filled or stroked shape. */
export class Shape extends Node {
  /** The fill colour, or null for no fill. */
  fill: string | null;
  /** The stroke colour, or null for no stroke. */
  stroke: string | null;
  /** The stroke width in logical pixels. */
  lineWidth: number;
  private geometryValue: ShapeGeometry;

  /**
   * @param options - see {@link ShapeOptions}.
   * @throws RangeError when the geometry has a negative or non-finite size, or too few points.
   */
  constructor(options: ShapeOptions) {
    super(options);
    this.geometryValue = checkGeometry(options.geometry);
    this.fill = options.fill === undefined ? "#ffffff" : options.fill;
    this.stroke = options.stroke ?? null;
    this.lineWidth = options.lineWidth ?? 1;
    if (!isNonNegativeFinite(this.lineWidth)) {
      throw new RangeError(`lineWidth must be a finite number from 0, got ${this.lineWidth}`);
    }
  }

  /** The geometry. Setting it checks the new geometry. */
  get geometry(): ShapeGeometry {
    return this.geometryValue;
  }

  set geometry(value: ShapeGeometry) {
    this.geometryValue = checkGeometry(value);
  }

  /**
   * The anchor box: it always starts at the origin, and it runs to the far corner of the geometry. So an anchor
   * of 0 is the origin, and a line or a polygon keeps its points where they are.
   */
  protected override localBox(out: MutableRect): void {
    const g = this.geometryValue;
    out.x = 0;
    out.y = 0;
    switch (g.kind) {
      case "rect":
      case "roundRect":
        out.width = g.width;
        out.height = g.height;
        return;
      case "circle":
        out.width = g.radius * 2;
        out.height = g.radius * 2;
        return;
      case "ellipse":
        out.width = g.radiusX * 2;
        out.height = g.radiusY * 2;
        return;
      case "line":
      case "polygon": {
        const bounds = pointBounds(g.points);
        out.width = Math.max(0, bounds.maxX);
        out.height = Math.max(0, bounds.maxY);
        return;
      }
    }
  }

  /** The drawn extent: the geometry itself, and the stroke that reaches past it. Culling uses it. */
  protected override contentRect(out: MutableRect): boolean {
    const g = this.geometryValue;
    if (g.kind === "line" || g.kind === "polygon") {
      const bounds = pointBounds(g.points);
      out.x = bounds.minX;
      out.y = bounds.minY;
      out.width = bounds.maxX - bounds.minX;
      out.height = bounds.maxY - bounds.minY;
    } else {
      this.localBox(out);
    }
    const pad = this.padding();
    out.x -= pad;
    out.y -= pad;
    out.width += pad * 2;
    out.height += pad * 2;
    return true;
  }

  protected override hitGeometry(x: number, y: number): boolean {
    const g = this.geometryValue;
    switch (g.kind) {
      case "rect":
        return x >= 0 && x <= g.width && y >= 0 && y <= g.height;
      case "roundRect":
        return insideRoundRect(x, y, g.width, g.height, g.radius);
      case "circle":
        return (x - g.radius) ** 2 + (y - g.radius) ** 2 <= g.radius ** 2;
      case "ellipse":
        return insideEllipse(x, y, g.radiusX, g.radiusY);
      case "line":
        return distanceToPolyline(x, y, g.points, false) <= this.lineHitTolerance();
      case "polygon":
        if (insidePolygon(x, y, g.points)) return true;
        return this.stroke !== null && distanceToPolyline(x, y, g.points, true) <= this.lineWidth / 2;
    }
  }

  protected override drawSelf(pass: DrawPass, alpha: number, _pixelArt: boolean): void {
    const ctx = pass.context;
    const g = this.geometryValue;
    pass.place(this.surfaceMatrix);
    pass.alpha(alpha);
    ctx.beginPath();
    appendPath(ctx, g);
    if (g.kind !== "line" && this.fill !== null) {
      ctx.fillStyle = this.fill;
      ctx.fill();
      pass.count();
    }
    if (this.stroke !== null && this.lineWidth > 0) {
      // Set on every stroke: a text leaves round joins in the context for its outline.
      ctx.strokeStyle = this.stroke;
      ctx.lineWidth = this.lineWidth;
      ctx.lineJoin = "miter";
      ctx.stroke();
      pass.count();
    }
  }

  /** How far the stroke reaches outside the geometry, in logical pixels. */
  private padding(): number {
    if (this.geometryValue.kind === "line") return this.lineHitTolerance();
    return this.stroke !== null ? this.lineWidth / 2 : 0;
  }

  /** The distance from a line that counts as a hit: half the stroke, and never less than the minimum. */
  private lineHitTolerance(): number {
    return Math.max(this.lineWidth / 2, LINE_HIT_MINIMUM);
  }
}

/** Checks a geometry and returns it. @throws RangeError when a size or a point is wrong. */
function checkGeometry(g: ShapeGeometry): ShapeGeometry {
  switch (g.kind) {
    case "rect":
      requireSize(g.width, "width");
      requireSize(g.height, "height");
      return g;
    case "roundRect":
      requireSize(g.width, "width");
      requireSize(g.height, "height");
      requireSize(g.radius, "radius");
      return g;
    case "circle":
      requireSize(g.radius, "radius");
      return g;
    case "ellipse":
      requireSize(g.radiusX, "radiusX");
      requireSize(g.radiusY, "radiusY");
      return g;
    case "line":
    case "polygon": {
      const minimum = g.kind === "line" ? 2 : 3;
      if (g.points.length < minimum) {
        throw new RangeError(`a ${g.kind} needs at least ${minimum} points, got ${g.points.length}`);
      }
      for (const point of g.points) {
        if (!Number.isFinite(point.x) || !Number.isFinite(point.y)) {
          throw new RangeError(`a ${g.kind} point must have finite coordinates`);
        }
      }
      return g;
    }
  }
}

/** Throws unless a size is a finite number from zero. */
function requireSize(value: number, name: string): void {
  if (!isNonNegativeFinite(value)) {
    throw new RangeError(`${name} must be a finite number from 0, got ${value}`);
  }
}

/** The path of a geometry, added to the context's current path. The caller has begun the path. */
function appendPath(ctx: Context2D, g: ShapeGeometry): void {
  switch (g.kind) {
    case "rect":
      ctx.rect(0, 0, g.width, g.height);
      return;
    case "roundRect":
      roundRectPath(ctx, g.width, g.height, g.radius);
      return;
    case "circle":
      ctx.arc(g.radius, g.radius, g.radius, 0, Math.PI * 2);
      return;
    case "ellipse":
      ctx.ellipse(g.radiusX, g.radiusY, g.radiusX, g.radiusY, 0, 0, Math.PI * 2);
      return;
    case "line":
    case "polygon": {
      const first = g.points[0];
      if (first === undefined) return;
      ctx.moveTo(first.x, first.y);
      for (let index = 1; index < g.points.length; index += 1) {
        const point = g.points[index];
        if (point !== undefined) ctx.lineTo(point.x, point.y);
      }
      if (g.kind === "polygon") ctx.closePath();
      return;
    }
  }
}

/** The path of a rounded rectangle, with the radius held to half of the smaller side. */
function roundRectPath(ctx: Context2D, width: number, height: number, radius: number): void {
  const r = Math.min(radius, width / 2, height / 2);
  ctx.moveTo(r, 0);
  ctx.lineTo(width - r, 0);
  ctx.arcTo(width, 0, width, r, r);
  ctx.lineTo(width, height - r);
  ctx.arcTo(width, height, width - r, height, r);
  ctx.lineTo(r, height);
  ctx.arcTo(0, height, 0, height - r, r);
  ctx.lineTo(0, r);
  ctx.arcTo(0, 0, r, 0, r);
  ctx.closePath();
}

/** Whether a point lies inside a rounded rectangle. */
function insideRoundRect(x: number, y: number, width: number, height: number, radius: number): boolean {
  if (x < 0 || y < 0 || x > width || y > height) return false;
  const r = Math.min(radius, width / 2, height / 2);
  // The nearest point of the inner rectangle; inside the corner regions it is the corner's centre.
  const nearestX = clamp(x, r, width - r);
  const nearestY = clamp(y, r, height - r);
  return (x - nearestX) ** 2 + (y - nearestY) ** 2 <= r ** 2;
}

/** Whether a point lies inside an ellipse centred on (radiusX, radiusY). */
function insideEllipse(x: number, y: number, radiusX: number, radiusY: number): boolean {
  if (radiusX === 0 || radiusY === 0) return false;
  return ((x - radiusX) / radiusX) ** 2 + ((y - radiusY) / radiusY) ** 2 <= 1;
}

/**
 * Whether a point lies inside a polygon, by the even-odd rule. The polygon closes from its last point to
 * its first.
 */
function insidePolygon(x: number, y: number, points: readonly Point2D[]): boolean {
  let inside = false;
  let previous = points.length - 1;
  for (let index = 0; index < points.length; index += 1) {
    const a = points[index];
    const b = points[previous];
    if (a !== undefined && b !== undefined && (a.y > y) !== (b.y > y)) {
      const crossingX = ((b.x - a.x) * (y - a.y)) / (b.y - a.y) + a.x;
      if (x < crossingX) inside = !inside;
    }
    previous = index;
  }
  return inside;
}

/**
 * The shortest distance from a point to a polyline. With `closed`, the polyline also runs from its last
 * point back to its first.
 */
function distanceToPolyline(x: number, y: number, points: readonly Point2D[], closed: boolean): number {
  let best = Number.POSITIVE_INFINITY;
  for (let index = 0; index + 1 < points.length; index += 1) {
    const a = points[index];
    const b = points[index + 1];
    if (a !== undefined && b !== undefined) best = Math.min(best, distanceToSegment(x, y, a, b));
  }
  const first = points[0];
  const last = points[points.length - 1];
  if (closed && first !== undefined && last !== undefined) {
    best = Math.min(best, distanceToSegment(x, y, last, first));
  }
  return best;
}

/** The shortest distance from a point to a segment. */
function distanceToSegment(x: number, y: number, a: Point2D, b: Point2D): number {
  const dx = b.x - a.x;
  const dy = b.y - a.y;
  const lengthSquared = dx * dx + dy * dy;
  const t = lengthSquared === 0 ? 0 : clamp(((x - a.x) * dx + (y - a.y) * dy) / lengthSquared, 0, 1);
  return Math.hypot(a.x + t * dx - x, a.y + t * dy - y);
}

/** The bounds that `pointBounds` writes into. One object, reused, so that a frame allocates nothing. */
const boundsScratch = { minX: 0, minY: 0, maxX: 0, maxY: 0 };

/**
 * The smallest and largest coordinates of a list of points. The result is a shared object: read its values
 * before the next call.
 */
function pointBounds(points: readonly Point2D[]): { minX: number; minY: number; maxX: number; maxY: number } {
  const out = boundsScratch;
  out.minX = Number.POSITIVE_INFINITY;
  out.minY = Number.POSITIVE_INFINITY;
  out.maxX = Number.NEGATIVE_INFINITY;
  out.maxY = Number.NEGATIVE_INFINITY;
  for (const point of points) {
    out.minX = Math.min(out.minX, point.x);
    out.minY = Math.min(out.minY, point.y);
    out.maxX = Math.max(out.maxX, point.x);
    out.maxY = Math.max(out.maxY, point.y);
  }
  return out;
}
