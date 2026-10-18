// Tests for shapes: each geometry draws its own path (a rectangle, a rounded corner, a circle, an ellipse, a
// line and a polygon), fills and strokes in order, a line is never filled, and bad geometry is refused.

import { describe, expect, it } from "vitest";
import { createScene2D } from "./scene";
import { Shape, type ShapeGeometry } from "./shape";
import { Text } from "./text";
import { expectNumbers, FakeCanvas } from "./test-support";

/** Draws one shape alone, and returns the canvas it drew on. */
function drawShape(options: ConstructorParameters<typeof Shape>[0]): FakeCanvas {
  const canvas = new FakeCanvas(1920, 1080);
  const scene = createScene2D({ canvas });
  scene.createLayer({ parallax: 0 }).add(new Shape(options));
  scene.render();
  return canvas;
}

describe("drawing", () => {
  it("draws a rectangle, filled and not stroked by default", () => {
    const canvas = drawShape({ geometry: { kind: "rect", width: 40, height: 20 } });
    expect(canvas.context.names).toEqual(expect.arrayContaining(["beginPath", "rect", "fill"]));
    expect(canvas.context.calls.find((call) => call.name === "rect")?.args).toEqual([0, 0, 40, 20]);
    expect(canvas.context.nodeDraws.map((draw) => draw.op)).toEqual(["fill"]);
    expect(canvas.context.nodeDraws[0]?.fillStyle).toBe("#ffffff");
  });

  it("fills and then strokes, with the stroke colour and width", () => {
    const canvas = drawShape({ geometry: { kind: "rect", width: 10, height: 10 }, fill: "#ff0000", stroke: "#00ff00", lineWidth: 4 });
    expect(canvas.context.nodeDraws.map((draw) => draw.op)).toEqual(["fill", "stroke"]);
    expect(canvas.context.nodeDraws[1]).toMatchObject({ strokeStyle: "#00ff00", lineWidth: 4 });
  });

  it("draws nothing with no fill and no stroke", () => {
    const canvas = drawShape({ geometry: { kind: "rect", width: 10, height: 10 }, fill: null });
    expect(canvas.context.nodeDraws).toEqual([]);
  });

  it("rounds the corners of a rounded rectangle, with the radius held to half the short side", () => {
    const canvas = drawShape({ geometry: { kind: "roundRect", width: 40, height: 20, radius: 50 } });
    const arcs = canvas.context.calls.filter((call) => call.name === "arcTo");
    expect(arcs).toHaveLength(4);
    // The radius is 10: half of the 20 pixel height.
    expect(arcs[0]?.args).toEqual([40, 0, 40, 10, 10]);
    expect(canvas.context.names).toContain("closePath");
  });

  it("draws a circle centred on its box, and an ellipse on its own box", () => {
    const circle = drawShape({ geometry: { kind: "circle", radius: 5 } });
    expect(circle.context.calls.find((call) => call.name === "arc")?.args.slice(0, 3)).toEqual([5, 5, 5]);
    const ellipse = drawShape({ geometry: { kind: "ellipse", radiusX: 6, radiusY: 3 } });
    expect(ellipse.context.calls.find((call) => call.name === "ellipse")?.args.slice(0, 4)).toEqual([6, 3, 6, 3]);
  });

  it("strokes a line and never fills it", () => {
    const geometry: ShapeGeometry = { kind: "line", points: [{ x: 0, y: 0 }, { x: 10, y: 0 }, { x: 10, y: 10 }] };
    const canvas = drawShape({ geometry, fill: "#ffffff", stroke: "#000000" });
    expect(canvas.context.names).toEqual(expect.arrayContaining(["moveTo", "lineTo", "stroke"]));
    expect(canvas.context.names).not.toContain("closePath");
    expect(canvas.context.nodeDraws.map((draw) => draw.op)).toEqual(["stroke"]);
  });

  it("closes a polygon's path", () => {
    const geometry: ShapeGeometry = { kind: "polygon", points: [{ x: 0, y: 0 }, { x: 10, y: 0 }, { x: 5, y: 8 }] };
    const canvas = drawShape({ geometry });
    expect(canvas.context.names.filter((name) => name === "lineTo")).toHaveLength(2);
    expect(canvas.context.names).toContain("closePath");
    expect(canvas.context.nodeDraws.map((draw) => draw.op)).toEqual(["fill"]);
  });

  it("refuses a negative size, and too few points", () => {
    expect(() => new Shape({ geometry: { kind: "rect", width: -1, height: 2 } })).toThrow(RangeError);
    expect(() => new Shape({ geometry: { kind: "line", points: [{ x: 0, y: 0 }] } })).toThrow(RangeError);
    expect(() => new Shape({ geometry: { kind: "polygon", points: [{ x: 0, y: 0 }, { x: 1, y: 1 }] } })).toThrow(RangeError);
  });

  it("checks a new geometry when it is set", () => {
    const shape = new Shape({ geometry: { kind: "circle", radius: 3 } });
    expect(() => {
      shape.geometry = { kind: "circle", radius: Number.NaN };
    }).toThrow(RangeError);
    expect(shape.geometry).toEqual({ kind: "circle", radius: 3 });
  });
});

describe("state left by other nodes", () => {
  it("strokes with mitred joins, even after a text has set round ones for its outline", () => {
    const canvas = new FakeCanvas(1920, 1080);
    const scene = createScene2D({ canvas });
    const layer = scene.createLayer({ parallax: 0 });
    layer.add(
      new Text({ text: "a", fontSize: 20, outline: { color: "#000000", width: 2 } }),
      new Shape({ geometry: { kind: "rect", width: 5, height: 5 }, fill: null, stroke: "#ffffff" }),
    );
    scene.render();
    expect(canvas.context.nodeDraws.find((draw) => draw.op === "stroke")?.lineJoin).toBe("miter");
  });
});

describe("anchors", () => {
  const triangle: ShapeGeometry = {
    kind: "polygon",
    points: [
      { x: 200, y: 0 },
      { x: 300, y: 0 },
      { x: 250, y: 100 },
    ],
  };

  it("keeps a polygon's points where they are at the default anchor", () => {
    const canvas = drawShape({ geometry: triangle });
    expectNumbers(canvas.context.nodeDraws[0]?.transform, [1, 0, 0, 1, 0, 0]);
    expect(canvas.context.calls.find((call) => call.name === "moveTo")?.args).toEqual([200, 0]);
  });

  it("measures the anchor of a polygon from the origin, to its far corner", () => {
    // The box runs from the origin to (300, 100), so an anchor of one half is (150, 50), and that point sits at
    // the node's place: the node is moved back by it.
    const canvas = drawShape({ geometry: triangle, anchorX: 0.5, anchorY: 0.5 });
    expectNumbers(canvas.context.nodeDraws[0]?.transform, [1, 0, 0, 1, -150, -50]);
  });
});
