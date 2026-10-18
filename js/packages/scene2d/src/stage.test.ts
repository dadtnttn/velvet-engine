// Tests for following a stage: the scene takes the stage's pixel size when it starts following, and again after
// each change of the layout, and the function it returns stops the following. A fake stage stands in for the ui
// Stage, which this package does not import.

import { describe, expect, it } from "vitest";
import { followStage } from "./stage";
import { createScene2D } from "./scene";
import { FakeCanvas, FakeStage } from "./test-support";

describe("followStage", () => {
  it("draws at the stage's pixel size from the start", () => {
    const canvas = new FakeCanvas();
    const scene = createScene2D({ canvas });
    followStage(scene, new FakeStage(1280, 720));
    expect(canvas.width).toBe(1280);
    expect(canvas.height).toBe(720);
    expect(scene.pixelScale).toBeCloseTo(1280 / 1920, 12);
  });

  it("follows each change of the stage's layout", () => {
    const canvas = new FakeCanvas();
    const scene = createScene2D({ canvas });
    const stage = new FakeStage(1920, 1080);
    followStage(scene, stage);
    stage.resize(2560, 1440);
    expect(canvas.width).toBe(2560);
    expect(canvas.height).toBe(1440);
    expect(scene.pixelScale).toBeCloseTo(2560 / 1920, 12);
  });

  it("stops following when its function is called", () => {
    const canvas = new FakeCanvas();
    const scene = createScene2D({ canvas });
    const stage = new FakeStage(1920, 1080);
    const stop = followStage(scene, stage);
    expect(stage.listenerCount).toBe(1);
    stop();
    expect(stage.listenerCount).toBe(0);
    stage.resize(800, 450);
    expect(canvas.width).toBe(1920);
  });
});
