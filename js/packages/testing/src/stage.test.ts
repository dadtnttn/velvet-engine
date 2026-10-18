import { describe, expect, it } from "vitest";
import { TestingError } from "./errors";
import {
  DEFAULT_STAGE_HEIGHT,
  DEFAULT_STAGE_WIDTH,
  clientToLogical,
  createStageMap,
  logicalToClient,
  placementOf,
} from "./stage";

describe("createStageMap", () => {
  it("is 1920 by 1080 at the client origin by default", () => {
    const stage = createStageMap();
    expect([stage.width, stage.height]).toEqual([DEFAULT_STAGE_WIDTH, DEFAULT_STAGE_HEIGHT]);
    expect(logicalToClient(stage, 100, 50)).toEqual({ x: 100, y: 50 });
  });

  it("refuses a size that is not a positive number", () => {
    expect(() => createStageMap({ width: 0 })).toThrow(TestingError);
    expect(() => createStageMap({ height: Number.NaN })).toThrow(/positive size/);
  });
});

describe("the letterbox placement", () => {
  it("fits the largest 16:9 area in a taller window, and centres it", () => {
    // A 1600 by 1200 window: the width limits the scale to 1600 / 1920, so the picture is 900 pixels tall.
    const stage = createStageMap({ area: () => ({ left: 0, top: 0, width: 1600, height: 1200 }) });
    const placement = placementOf(stage);
    expect(placement.scale).toBeCloseTo(1600 / 1920, 12);
    expect(placement.offsetX).toBeCloseTo(0, 9);
    expect(placement.offsetY).toBeCloseTo(150, 9);
    expect(logicalToClient(stage, 0, 0)).toEqual({ x: 0, y: placement.offsetY });
  });

  it("adds the area's own position to every point", () => {
    const stage = createStageMap({ area: () => ({ left: 100, top: 50, width: 1920, height: 1080 }) });
    expect(logicalToClient(stage, 960, 540)).toEqual({ x: 1060, y: 590 });
  });

  it("inverts exactly: clientToLogical undoes logicalToClient at many points", () => {
    const stage = createStageMap({ area: () => ({ left: 37, top: 11, width: 1333, height: 999 }) });
    for (let step = 0; step <= 20; step += 1) {
      const x = (step / 20) * DEFAULT_STAGE_WIDTH;
      const y = (1 - step / 20) * DEFAULT_STAGE_HEIGHT;
      const client = logicalToClient(stage, x, y);
      const back = clientToLogical(stage, client.x, client.y);
      expect(back.x).toBeCloseTo(x, 9);
      expect(back.y).toBeCloseTo(y, 9);
    }
  });

  it("reads the area at each use, so a resize is seen at once", () => {
    let width = 1920;
    const stage = createStageMap({ area: () => ({ left: 0, top: 0, width, height: 1080 }) });
    expect(logicalToClient(stage, 960, 0).x).toBeCloseTo(960, 9);
    width = 960;
    expect(logicalToClient(stage, 960, 0).x).toBeCloseTo(480, 9);
  });

  it("refuses to map a client point when the stage has no size on screen", () => {
    const stage = createStageMap({ area: () => ({ left: 0, top: 0, width: 0, height: 0 }) });
    expect(() => clientToLogical(stage, 10, 10)).toThrow(/no size on screen/);
  });
});
