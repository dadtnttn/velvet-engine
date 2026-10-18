// @vitest-environment happy-dom
// Tests for the frame hooks of the loop: the order of the phases, the payloads and the timings (with a
// fake time source), unsubscribing, the zero cost while nothing listens, a stop or an error inside a
// frame, and the order against input's endFrame. The frames come from a scheduler that the test drives.

import { describe, expect, it, vi } from "vitest";
import { ActionMap } from "./input";
import { GameLoop, type FrameTiming, type StepEnd } from "./loop";
import { ManualFrames } from "./test-support";

describe("loop frame hooks: order and payloads", () => {
  it("runs the phases of each frame in order, with each fixed step between its own two hooks", () => {
    const frames = new ManualFrames();
    const log: string[] = [];
    const loop = new GameLoop({
      rate: 10,
      scheduler: frames,
      update: () => log.push("update"),
      render: () => log.push("render"),
    });
    loop.on("beforeFrame", (start) => log.push(`beforeFrame ${start.frame}`));
    loop.on("beforeUpdate", (step) => log.push(`beforeUpdate ${step.frame}.${step.step}`));
    loop.on("afterUpdate", (step) => log.push(`afterUpdate ${step.frame}.${step.step}`));
    loop.on("afterRender", (render) => log.push(`afterRender ${render.frame}`));
    loop.on("afterFrame", (timing) => log.push(`afterFrame ${timing.frame}`));
    loop.start();

    frames.runFrame(0);
    frames.runFrame(250);

    expect(log).toEqual([
      "beforeFrame 1",
      "render",
      "afterRender 1",
      "afterFrame 1",
      "beforeFrame 2",
      "beforeUpdate 2.0",
      "update",
      "afterUpdate 2.0",
      "beforeUpdate 2.1",
      "update",
      "afterUpdate 2.1",
      "render",
      "afterRender 2",
      "afterFrame 2",
    ]);
  });

  it("gives the frame its time and the real time since the previous frame, and the alpha of the render", () => {
    const frames = new ManualFrames();
    const starts: number[] = [];
    const alphas: number[] = [];
    const loop = new GameLoop({ rate: 10, scheduler: frames, update: () => undefined, render: () => undefined });
    loop.on("beforeFrame", (start) => starts.push(start.realDt));
    loop.on("afterRender", (render) => alphas.push(render.alpha));
    loop.start();

    frames.runFrame(1000);
    frames.runFrame(1100);
    frames.runFrame(1250);

    expect(starts[0]).toBe(0);
    expect(starts[1]).toBeCloseTo(0.1, 9);
    expect(starts[2]).toBeCloseTo(0.15, 9);
    // 0.1 s is one step exactly, so the second frame has no remainder; 0.15 s leaves half a step.
    expect(alphas[1]).toBeCloseTo(0, 9);
    expect(alphas[2]).toBeCloseTo(0.5, 9);
  });

  it("reports each step's place, length and duration, and the whole frame's timing with a fake time source", () => {
    const frames = new ManualFrames();
    let nowMs = 0;
    const loop = new GameLoop({
      rate: 10,
      scheduler: frames,
      now: () => nowMs,
      update: () => {
        nowMs += 3;
      },
      render: () => {
        nowMs += 5;
      },
    });
    const timings: FrameTiming[] = [];
    const steps: StepEnd[] = [];
    loop.on("afterFrame", (timing) => timings.push(timing));
    loop.on("afterUpdate", (step) => steps.push(step));
    loop.start();

    frames.runFrame(0);
    frames.runFrame(250);

    // The first frame steps nothing and renders for 5 ms.
    expect(timings[0]).toEqual({
      frame: 1,
      timeMs: 0,
      realDt: 0,
      scaledDt: 0,
      steps: 0,
      updateMs: 0,
      renderMs: 5,
      frameMs: 5,
    });
    // The second frame has 0.25 s of real time: two steps of 0.1 s, each update 3 ms, then a 5 ms render.
    const second = timings[1];
    expect(second).toBeDefined();
    expect(second?.frame).toBe(2);
    expect(second?.timeMs).toBe(250);
    expect(second?.realDt).toBeCloseTo(0.25, 9);
    expect(second?.scaledDt).toBeCloseTo(0.2, 9);
    expect(second?.steps).toBe(2);
    expect(second?.updateMs).toBe(6);
    expect(second?.renderMs).toBe(5);
    expect(second?.frameMs).toBe(11);
    expect(steps.map((step) => [step.frame, step.step, step.durationMs])).toEqual([
      [2, 0, 3],
      [2, 1, 3],
    ]);
    expect(steps[0]?.dt).toBeCloseTo(0.1, 9);
  });

  it("caps the steps of one frame at maxSteps, and reports the steps that ran", () => {
    const frames = new ManualFrames();
    const timings: FrameTiming[] = [];
    const loop = new GameLoop({ rate: 10, maxSteps: 2, scheduler: frames, update: () => undefined });
    loop.on("afterFrame", (timing) => timings.push(timing));
    loop.start();
    frames.runFrame(0);
    frames.runFrame(5000);
    expect(timings[1]?.steps).toBe(2);
    expect(timings[1]?.scaledDt).toBeCloseTo(0.2, 9);
  });

  it("reports a render with no render function as a phase of zero duration", () => {
    const frames = new ManualFrames();
    const renders: number[] = [];
    const loop = new GameLoop({ rate: 10, scheduler: frames, update: () => undefined });
    loop.on("afterRender", (render) => renders.push(render.durationMs));
    loop.start();
    frames.runFrame(0);
    frames.runFrame(100);
    expect(renders).toEqual([0, 0]);
  });
});

describe("loop frame hooks: subscriptions", () => {
  it("stops calling a hook once its unsubscribe function has run", () => {
    const frames = new ManualFrames();
    const loop = new GameLoop({ rate: 10, scheduler: frames, update: () => undefined });
    let calls = 0;
    const unsubscribe = loop.on("afterFrame", () => {
      calls += 1;
    });
    loop.start();
    frames.runFrame(0);
    frames.runFrame(100);
    expect(calls).toBe(2);
    unsubscribe();
    unsubscribe();
    frames.runFrame(200);
    expect(calls).toBe(2);
  });

  it("calls a hook added during a frame from the next phase that the frame reaches", () => {
    const frames = new ManualFrames();
    const seen: string[] = [];
    let addOnce = true;
    const loop = new GameLoop({
      rate: 10,
      scheduler: frames,
      update: () => {
        if (!addOnce) return;
        addOnce = false;
        loop.on("afterFrame", (timing) => seen.push(`late hook, frame ${timing.frame}`));
      },
    });
    // A hook that is already listening makes every frame timed.
    loop.on("beforeFrame", () => undefined);
    loop.start();
    frames.runFrame(0);
    frames.runFrame(100);
    frames.runFrame(200);
    expect(seen).toEqual(["late hook, frame 2", "late hook, frame 3"]);
  });

  it("does not call any hook in a frame that began with no hook listening, even one it added itself", () => {
    const frames = new ManualFrames();
    const seen: string[] = [];
    let addOnce = true;
    const loop = new GameLoop({
      rate: 10,
      scheduler: frames,
      update: () => {
        if (!addOnce) return;
        addOnce = false;
        loop.on("afterFrame", (timing) => seen.push(`frame ${timing.frame}`));
      },
    });
    loop.start();
    frames.runFrame(0);
    frames.runFrame(100);
    expect(seen).toEqual([]);
    frames.runFrame(200);
    expect(seen).toEqual(["frame 3"]);
  });
});

describe("loop frame hooks: cost and clock", () => {
  it("reads no clock and builds no timing while no hook listens", () => {
    const frames = new ManualFrames();
    const now = vi.fn(() => 0);
    const loop = new GameLoop({ rate: 10, scheduler: frames, now, update: () => undefined, render: () => undefined });
    loop.start();
    frames.runFrame(0);
    frames.runFrame(250);
    expect(now).not.toHaveBeenCalled();

    const stop = loop.on("afterFrame", () => undefined);
    frames.runFrame(500);
    expect(now).toHaveBeenCalled();

    now.mockClear();
    stop();
    frames.runFrame(750);
    expect(now).not.toHaveBeenCalled();
  });

  it("uses performance.now when no clock is given", () => {
    const spy = vi.spyOn(performance, "now").mockReturnValue(42);
    try {
      const frames = new ManualFrames();
      const timings: FrameTiming[] = [];
      const loop = new GameLoop({ rate: 10, scheduler: frames, update: () => undefined });
      loop.on("afterFrame", (timing) => timings.push(timing));
      loop.start();
      frames.runFrame(0);
      expect(spy).toHaveBeenCalled();
      expect(timings[0]?.frameMs).toBe(0);
    } finally {
      spy.mockRestore();
    }
  });
});

describe("loop frame hooks: stops and errors", () => {
  it("when update stops the loop, skips the rest of the frame, and still reports the frame once", () => {
    const frames = new ManualFrames();
    const log: string[] = [];
    const loop = new GameLoop({
      rate: 10,
      scheduler: frames,
      update: () => {
        log.push("update");
        loop.stop();
      },
      render: () => log.push("render"),
    });
    loop.on("afterRender", () => log.push("afterRender"));
    loop.on("afterFrame", (timing) => log.push(`afterFrame steps=${timing.steps} renderMs=${timing.renderMs}`));
    loop.start();
    frames.runFrame(0);
    log.length = 0;

    frames.runFrame(250);

    expect(log).toEqual(["update", "afterFrame steps=1 renderMs=0"]);
    expect(loop.running).toBe(false);
  });

  it("lets an error from a hook stop that frame, and the next frame still runs", () => {
    const frames = new ManualFrames();
    const renders: number[] = [];
    let fail = true;
    const loop = new GameLoop({
      rate: 10,
      scheduler: frames,
      update: () => undefined,
      render: () => {
        renders.push(1);
      },
    });
    loop.on("afterUpdate", () => {
      if (!fail) return;
      fail = false;
      throw new Error("hook failed");
    });
    loop.start();
    frames.runFrame(0);
    expect(() => frames.runFrame(100)).toThrow("hook failed");
    expect(renders).toHaveLength(1);

    frames.runFrame(200);
    expect(renders).toHaveLength(2);
    expect(loop.running).toBe(true);
  });
});

describe("loop frame hooks and input's endFrame", () => {
  it("shows the edges to beforeUpdate and to update, and clears them for afterUpdate once update has called endFrame", () => {
    const frames = new ManualFrames();
    const map = new ActionMap<"fire">([{ action: "fire", control: { type: "key", key: "KeyF" } }]);
    map.attach(window);
    const seen: string[] = [];
    const loop = new GameLoop({
      rate: 10,
      scheduler: frames,
      update: () => {
        seen.push(`update pressed=${map.pressed("fire")}`);
        map.endFrame();
      },
    });
    loop.on("beforeUpdate", () => seen.push(`beforeUpdate pressed=${map.pressed("fire")}`));
    loop.on("afterUpdate", () => seen.push(`afterUpdate pressed=${map.pressed("fire")}`));
    loop.start();
    frames.runFrame(0);

    window.dispatchEvent(new KeyboardEvent("keydown", { code: "KeyF" }));
    frames.runFrame(100);
    map.detach();

    expect(seen).toEqual(["beforeUpdate pressed=true", "update pressed=true", "afterUpdate pressed=false"]);
  });
});
