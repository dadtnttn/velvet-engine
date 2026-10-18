// Small 2D maths for the scene: affine matrices, mutable rectangles and axis-aligned boxes.
//
// A matrix holds the six numbers of a canvas transform. It maps a point (x, y) to
// (a·x + c·y + e, b·x + d·y + f), so `set(a, b, c, d, e, f)` takes the numbers that
// `setTransform` takes. Every node keeps its own matrices, and the scene rewrites them each frame,
// so the render path builds no objects of its own.

/** A point in some coordinate space. */
export interface Point2D {
  readonly x: number;
  readonly y: number;
}

/** A rectangle: its top-left corner, and its size. */
export interface Rect2D {
  readonly x: number;
  readonly y: number;
  readonly width: number;
  readonly height: number;
}

/** A rectangle that a call writes into, so that the call does not need to allocate one. */
export interface MutableRect {
  x: number;
  y: number;
  width: number;
  height: number;
}

/** A point that a call writes into. */
export interface MutablePoint {
  x: number;
  y: number;
}

/** An affine transform: the six numbers of a canvas matrix. It is reused and rewritten in place. */
export class Matrix2D {
  a = 1;
  b = 0;
  c = 0;
  d = 1;
  e = 0;
  f = 0;

  /** Sets the six numbers. @returns this matrix. */
  set(a: number, b: number, c: number, d: number, e: number, f: number): this {
    this.a = a;
    this.b = b;
    this.c = c;
    this.d = d;
    this.e = e;
    this.f = f;
    return this;
  }

  /** Makes this matrix the identity. @returns this matrix. */
  reset(): this {
    return this.set(1, 0, 0, 1, 0, 0);
  }

  /**
   * Makes this matrix the placement of an object. The point `(pivotX, pivotY)` of the object, in its
   * own units, lands on `(x, y)`, and the object turns by `rotation` radians and scales by `scaleX` and
   * `scaleY` around that point. A positive rotation is clockwise on the screen, because the y axis
   * points down. @returns this matrix.
   */
  placement(
    x: number,
    y: number,
    rotation: number,
    scaleX: number,
    scaleY: number,
    pivotX: number,
    pivotY: number,
  ): this {
    const cos = rotation === 0 ? 1 : Math.cos(rotation);
    const sin = rotation === 0 ? 0 : Math.sin(rotation);
    const a = cos * scaleX;
    const b = sin * scaleX;
    const c = -sin * scaleY;
    const d = cos * scaleY;
    return this.set(a, b, c, d, x - (a * pivotX + c * pivotY), y - (b * pivotX + d * pivotY));
  }

  /**
   * Makes this matrix `left` after `right`: a point is moved by `right` first, then by `left`. Either
   * input may be this matrix. @returns this matrix.
   */
  product(left: Readonly<Matrix2D>, right: Readonly<Matrix2D>): this {
    const { a: la, b: lb, c: lc, d: ld, e: le, f: lf } = left;
    const { a: ra, b: rb, c: rc, d: rd, e: re, f: rf } = right;
    return this.set(
      la * ra + lc * rb,
      lb * ra + ld * rb,
      la * rc + lc * rd,
      lb * rc + ld * rd,
      la * re + lc * rf + le,
      lb * re + ld * rf + lf,
    );
  }

  /**
   * Makes this matrix the inverse of `source`. When `source` has no inverse (its determinant is zero or
   * not finite), this matrix keeps its numbers. @returns false in that case.
   */
  invertFrom(source: Readonly<Matrix2D>): boolean {
    const { a, b, c, d, e, f } = source;
    const det = a * d - b * c;
    if (!Number.isFinite(det) || det === 0) return false;
    this.set(d / det, -b / det, -c / det, a / det, (c * f - d * e) / det, (b * e - a * f) / det);
    return true;
  }

  /** The point `(x, y)` moved by this matrix. It writes into `out` when given, or else into a new point. */
  apply(x: number, y: number, out: MutablePoint = { x: 0, y: 0 }): MutablePoint {
    out.x = this.a * x + this.c * y + this.e;
    out.y = this.b * x + this.d * y + this.f;
    return out;
  }

  /** The average scale: the square root of the size of the determinant. A matrix that squashes to a line gives 0. */
  meanScale(): number {
    return Math.sqrt(Math.abs(this.a * this.d - this.b * this.c));
  }
}

/** An axis-aligned box. It starts empty, and each point or box that is added makes it larger. */
export class Aabb {
  minX = Number.POSITIVE_INFINITY;
  minY = Number.POSITIVE_INFINITY;
  maxX = Number.NEGATIVE_INFINITY;
  maxY = Number.NEGATIVE_INFINITY;

  /** True when nothing has been added. */
  get isEmpty(): boolean {
    return this.minX > this.maxX || this.minY > this.maxY;
  }

  /** Empties the box again. */
  clear(): void {
    this.minX = Number.POSITIVE_INFINITY;
    this.minY = Number.POSITIVE_INFINITY;
    this.maxX = Number.NEGATIVE_INFINITY;
    this.maxY = Number.NEGATIVE_INFINITY;
  }

  /** Makes the box hold a point. */
  addPoint(x: number, y: number): void {
    if (x < this.minX) this.minX = x;
    if (x > this.maxX) this.maxX = x;
    if (y < this.minY) this.minY = y;
    if (y > this.maxY) this.maxY = y;
  }

  /** Makes the box hold another box. An empty box adds nothing. */
  addBox(other: Aabb): void {
    if (other.isEmpty) return;
    this.addPoint(other.minX, other.minY);
    this.addPoint(other.maxX, other.maxY);
  }

  /**
   * Makes the box hold a rectangle after a matrix has moved its four corners. The matrix maps the
   * rectangle's own coordinates, so a rotated rectangle gets the box that holds its turned corners.
   */
  addRect(matrix: Readonly<Matrix2D>, x: number, y: number, width: number, height: number): void {
    const right = x + width;
    const bottom = y + height;
    this.addMapped(matrix, x, y);
    this.addMapped(matrix, right, y);
    this.addMapped(matrix, x, bottom);
    this.addMapped(matrix, right, bottom);
  }

  /** True when a point lies inside the box, edges included. An empty box contains nothing. */
  containsPoint(x: number, y: number): boolean {
    return !this.isEmpty && x >= this.minX && x <= this.maxX && y >= this.minY && y <= this.maxY;
  }

  /** True when the box touches the rectangle with the given corners, edges included. */
  overlaps(minX: number, minY: number, maxX: number, maxY: number): boolean {
    return (
      !this.isEmpty && this.minX <= maxX && this.maxX >= minX && this.minY <= maxY && this.maxY >= minY
    );
  }

  /** Makes the box hold a point after a matrix has moved it. */
  private addMapped(matrix: Readonly<Matrix2D>, x: number, y: number): void {
    this.addPoint(matrix.a * x + matrix.c * y + matrix.e, matrix.b * x + matrix.d * y + matrix.f);
  }
}

/** Keeps a number inside [min, max], which must be in that order. A NaN gives `min`. */
export function clamp(value: number, min: number, max: number): number {
  if (Number.isNaN(value)) return min;
  return Math.min(max, Math.max(min, value));
}

/** True for a finite number above zero. */
export function isPositiveFinite(value: number): boolean {
  return Number.isFinite(value) && value > 0;
}

/** True for a finite number that is zero or more. */
export function isNonNegativeFinite(value: number): boolean {
  return Number.isFinite(value) && value >= 0;
}
