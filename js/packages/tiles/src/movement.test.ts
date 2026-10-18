// Tests for swept box movement: free movement, contacts on each side, landing on a one-way platform from
// above and passing it from below or sideways, the ledge that is only partly solid, no tunnelling at high
// speed, the probe of a box that stands on something, and the refusals.

import { describe, expect, it } from "vitest";
import { TilesError } from "./error";
import { buildCollisionGrid, type CollisionGrid } from "./collision";
import { moveBox } from "./movement";
import { loadTileMap } from "./load";
import { map, tileLayer } from "./test-support";

/**
 * The test level, 8 by 6 cells of 16 pixels. Stone walls stand in the first and last columns from row 3 down,
 * and the floor is row 5. A one-way platform covers columns 2 to 4 of row 3. A ledge (solid in its top six
 * pixels) sits in column 4 of row 1.
 */
const level = [
  0, 0, 0, 0, 0, 0, 0, 0,
  0, 0, 0, 0, 3, 0, 0, 0,
  0, 0, 0, 0, 0, 0, 0, 0,
  2, 0, 6, 6, 6, 0, 0, 2,
  2, 0, 0, 0, 0, 0, 0, 2,
  2, 2, 2, 2, 2, 2, 2, 2,
];

/** The grid of the level: stone and the floor are solid, the platform is one-way, and the ledge is partial. */
async function levelGrid(): Promise<CollisionGrid> {
  const loaded = await loadTileMap(map({ width: 8, height: 6, layers: [tileLayer("level", 8, 6, level)] }));
  return buildCollisionGrid(loaded, { nonEmpty: false, solidProperty: "solid", oneWayProperty: "oneWay", shapes: true });
}

describe("moving through open space", () => {
  it("moves the box by the distance asked, with no contacts", async () => {
    const grid = await levelGrid();
    const result = moveBox(grid, { x: 20, y: 20, width: 8, height: 8 }, 5, 5);
    expect(result.box).toEqual({ x: 25, y: 25, width: 8, height: 8 });
    expect(result.contacts).toEqual({ left: false, right: false, top: false, bottom: false });
    expect(result.tiles).toEqual([]);
  });

  it("does not move, and reports no contacts, for a distance of zero", async () => {
    const grid = await levelGrid();
    const result = moveBox(grid, { x: 20, y: 20, width: 8, height: 8 }, 0, 0);
    expect(result.box).toEqual({ x: 20, y: 20, width: 8, height: 8 });
    expect(result.tiles).toEqual([]);
  });
});

describe("contacts with solid tiles", () => {
  it("stops flush against a wall, and names the cell that stopped it", async () => {
    const grid = await levelGrid();
    const result = moveBox(grid, { x: 90, y: 56, width: 8, height: 8 }, 100, 0);
    expect(result.box.x).toBe(104);
    expect(result.contacts.right).toBe(true);
    expect(result.tiles).toEqual([{ x: 7, y: 3, side: "right" }]);
  });

  it("stops against a wall moving left, on the left side", async () => {
    const grid = await levelGrid();
    const result = moveBox(grid, { x: 30, y: 56, width: 8, height: 8 }, -50, 0);
    expect(result.box.x).toBe(16);
    expect(result.contacts.left).toBe(true);
    expect(result.tiles).toEqual([{ x: 0, y: 3, side: "left" }]);
  });

  it("does not tunnel through a one-tile wall at a very high speed", async () => {
    const grid = await levelGrid();
    const result = moveBox(grid, { x: -50, y: 56, width: 8, height: 8 }, 1000, 0);
    expect(result.box.x).toBe(-8);
    expect(result.contacts.right).toBe(true);
  });

  it("stops a thin, partly solid ledge that is passed at a high speed", async () => {
    const grid = await levelGrid();
    const result = moveBox(grid, { x: 66, y: -40, width: 8, height: 2 }, 0, 200);
    expect(result.box.y).toBe(14);
    expect(result.contacts.bottom).toBe(true);
    expect(result.tiles).toEqual([{ x: 4, y: 1, side: "bottom" }]);
  });

  it("stops under the ledge's solid part when moving up, and the top contact names the ledge", async () => {
    const grid = await levelGrid();
    const result = moveBox(grid, { x: 66, y: 30, width: 8, height: 8 }, 0, -30);
    expect(result.box.y).toBe(22);
    expect(result.contacts.top).toBe(true);
    expect(result.tiles).toEqual([{ x: 4, y: 1, side: "top" }]);
  });

  it("lets a box that starts inside a wall move out of it without being stopped", async () => {
    const grid = await levelGrid();
    const result = moveBox(grid, { x: 2, y: 56, width: 8, height: 8 }, 20, 0);
    expect(result.box.x).toBe(22);
    expect(result.contacts.right).toBe(false);
  });
});

describe("one-way platforms", () => {
  it("catches a box that falls onto the platform from above, and lands it on the top", async () => {
    const grid = await levelGrid();
    const result = moveBox(grid, { x: 40, y: 20, width: 8, height: 8 }, 0, 100);
    expect(result.box.y).toBe(40);
    expect(result.contacts.bottom).toBe(true);
    expect(result.tiles).toEqual([{ x: 2, y: 3, side: "bottom" }]);
  });

  it("lets a box fall through the platform when ignoreOneWay is set", async () => {
    const grid = await levelGrid();
    const result = moveBox(grid, { x: 40, y: 20, width: 8, height: 8 }, 0, 100, { ignoreOneWay: true });
    expect(result.box.y).toBe(72);
    expect(result.tiles).toEqual([{ x: 2, y: 5, side: "bottom" }]);
  });

  it("lets a box pass up through the platform from below", async () => {
    const grid = await levelGrid();
    const result = moveBox(grid, { x: 40, y: 70, width: 8, height: 8 }, 0, -100);
    expect(result.box.y).toBe(-30);
    expect(result.contacts.top).toBe(false);
  });

  it("does not block a box that moves sideways through the platform's top edge", async () => {
    const grid = await levelGrid();
    const result = moveBox(grid, { x: 20, y: 44, width: 8, height: 8 }, 50, 0);
    expect(result.box.x).toBe(70);
    expect(result.contacts.right).toBe(false);
  });

  it("reports a box standing on the platform as touching it, with a probe of zero distance", async () => {
    const grid = await levelGrid();
    const standing = { x: 40, y: 40, width: 8, height: 8 };
    expect(moveBox(grid, standing, 0, 0).contacts.bottom).toBe(false);
    const probe = moveBox(grid, standing, 0, 1);
    expect(probe.contacts.bottom).toBe(true);
    expect(probe.box.y).toBe(40);
  });
});

describe("the arguments", () => {
  it("refuses a distance that is not a finite number", async () => {
    const grid = await levelGrid();
    expect(() => moveBox(grid, { x: 0, y: 0, width: 8, height: 8 }, Number.NaN, 0)).toThrow(TilesError);
    expect(() => moveBox(grid, { x: 0, y: 0, width: 8, height: 8 }, 0, Infinity)).toThrow(TilesError);
  });

  it("refuses a box with a negative size or a coordinate that is not finite", async () => {
    const grid = await levelGrid();
    expect(() => moveBox(grid, { x: 0, y: 0, width: -1, height: 8 }, 1, 0)).toThrow(/negative size/);
    expect(() => moveBox(grid, { x: Infinity, y: 0, width: 8, height: 8 }, 1, 0)).toThrow(/finite/);
  });
});
