// Tests for the 2D maths: a placement puts its pivot on the given point, a product applies its right-hand
// matrix first, an inverse undoes its matrix (and a singular one has none), and a box holds the turned
// corners of a rectangle.

import { describe, expect, it } from "vitest";
import { Aabb, clamp, isNonNegativeFinite, isPositiveFinite, Matrix2D } from "./math";

describe("Matrix2D", () => {
  it("places the pivot of an object on the given point", () => {
    // The pivot (10, 20) of a 2x3 scaled object lands on (100, 50).
    const m = new Matrix2D().placement(100, 50, 0, 2, 3, 10, 20);
    const at = m.apply(10, 20);
    expect(at.x).toBeCloseTo(100, 9);
    expect(at.y).toBeCloseTo(50, 9);
    expect(m.a).toBe(2);
    expect(m.d).toBe(3);
  });

  it("turns clockwise on the screen, where the y axis points down", () => {
    // A quarter turn takes the point to the right of the origin to the point below it.
    const m = new Matrix2D().placement(0, 0, Math.PI / 2, 1, 1, 0, 0);
    const turned = m.apply(1, 0);
    expect(turned.x).toBeCloseTo(0, 9);
    expect(turned.y).toBeCloseTo(1, 9);
  });

  it("applies the right-hand matrix first in a product", () => {
    // Scale by 2 first, then move right by 10: (1, 1) becomes (2, 2), then (12, 2).
    const move = new Matrix2D().set(1, 0, 0, 1, 10, 0);
    const scale = new Matrix2D().set(2, 0, 0, 2, 0, 0);
    const out = new Matrix2D().product(move, scale).apply(1, 1);
    expect(out).toEqual({ x: 12, y: 2 });
  });

  it("allows the product to write into one of its inputs", () => {
    const a = new Matrix2D().set(2, 0, 0, 2, 0, 0);
    const b = new Matrix2D().set(1, 0, 0, 1, 5, 5);
    a.product(a, b);
    expect(a.apply(0, 0)).toEqual({ x: 10, y: 10 });
  });

  it("inverts a matrix, so that the product with it is the identity", () => {
    const m = new Matrix2D().placement(37, -12, 0.7, 1.5, 0.8, 4, 9);
    const inverse = new Matrix2D();
    expect(inverse.invertFrom(m)).toBe(true);
    const back = new Matrix2D().product(inverse, m);
    expect(back.a).toBeCloseTo(1, 12);
    expect(back.b).toBeCloseTo(0, 12);
    expect(back.c).toBeCloseTo(0, 12);
    expect(back.d).toBeCloseTo(1, 12);
    expect(back.e).toBeCloseTo(0, 9);
    expect(back.f).toBeCloseTo(0, 9);
  });

  it("refuses to invert a singular matrix and keeps its numbers", () => {
    const flat = new Matrix2D().set(1, 0, 2, 0, 5, 5);
    const target = new Matrix2D().set(9, 9, 9, 9, 9, 9);
    expect(target.invertFrom(flat)).toBe(false);
    expect(target.a).toBe(9);
  });

  it("reports the mean scale of a matrix", () => {
    expect(new Matrix2D().set(2, 0, 0, 8, 0, 0).meanScale()).toBeCloseTo(4, 12);
    expect(new Matrix2D().set(0, 0, 0, 0, 0, 0).meanScale()).toBe(0);
  });
});

describe("Aabb", () => {
  it("starts empty and holds nothing", () => {
    const box = new Aabb();
    expect(box.isEmpty).toBe(true);
    expect(box.containsPoint(0, 0)).toBe(false);
    expect(box.overlaps(-10, -10, 10, 10)).toBe(false);
  });

  it("holds the turned corners of a rectangle", () => {
    // A 10 by 20 rectangle at the origin, turned a quarter turn: it becomes 20 wide and 10 tall.
    const turn = new Matrix2D().placement(0, 0, Math.PI / 2, 1, 1, 0, 0);
    const box = new Aabb();
    box.addRect(turn, 0, 0, 10, 20);
    expect(box.minX).toBeCloseTo(-20, 9);
    expect(box.maxX).toBeCloseTo(0, 9);
    expect(box.minY).toBeCloseTo(0, 9);
    expect(box.maxY).toBeCloseTo(10, 9);
  });

  it("includes its edges and overlaps touching boxes", () => {
    const box = new Aabb();
    box.addPoint(0, 0);
    box.addPoint(10, 10);
    expect(box.containsPoint(10, 10)).toBe(true);
    expect(box.containsPoint(10.5, 5)).toBe(false);
    expect(box.overlaps(10, 10, 20, 20)).toBe(true);
    expect(box.overlaps(11, 0, 20, 20)).toBe(false);
  });

  it("adds another box, and ignores an empty one", () => {
    const a = new Aabb();
    a.addPoint(0, 0);
    a.addPoint(1, 1);
    const b = new Aabb();
    b.addPoint(5, 5);
    a.addBox(b);
    expect(a.maxX).toBe(5);
    a.addBox(new Aabb());
    expect(a.maxX).toBe(5);
    a.clear();
    expect(a.isEmpty).toBe(true);
  });
});

describe("number helpers", () => {
  it("clamps into a range, and gives the minimum for NaN", () => {
    expect(clamp(5, 0, 3)).toBe(3);
    expect(clamp(-1, 0, 3)).toBe(0);
    expect(clamp(2, 0, 3)).toBe(2);
    expect(clamp(Number.NaN, 1, 3)).toBe(1);
  });

  it("tells positive and non-negative finite numbers apart", () => {
    expect(isPositiveFinite(0)).toBe(false);
    expect(isPositiveFinite(0.5)).toBe(true);
    expect(isPositiveFinite(Number.POSITIVE_INFINITY)).toBe(false);
    expect(isNonNegativeFinite(0)).toBe(true);
    expect(isNonNegativeFinite(-0.1)).toBe(false);
    expect(isNonNegativeFinite(Number.NaN)).toBe(false);
  });
});
