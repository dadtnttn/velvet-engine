// The Vitest example of the package README, as a test, so that the README cannot drift from the code.

import { ActionMap, GameClock } from "@velvet/core";
import { describe, expect, it } from "vitest";
import { InputScript, createDriver } from "./index";

describe("the README example", () => {
  it("gives a coin for each jump", () => {
    const driver = createDriver();
    const clock = new GameClock();
    const keys = new ActionMap<"jump">([{ action: "jump", control: { type: "key", key: "Space" } }]);
    keys.attach(driver.target); // scripted keys, pointer and wheel reach the map here
    let coins = 0;
    const loop = driver.createLoop({
      update(dt) {
        clock.advance(dt);
        if (keys.pressed("jump")) coins += 1;
        keys.endFrame();
      },
    });
    loop.start();
    try {
      driver.keyDown("Space");
      driver.step(1);
      driver.keyUp("Space");
      expect(coins).toBe(1);

      const script = new InputScript();
      script.at(5).tap("Space"); // one frame, five frames from now
      script.at(9).press("Space"); // held for three frames
      driver.play(script);
      driver.until(() => driver.frame >= 20);
      expect(coins).toBe(3);
    } finally {
      driver.dispose();
    }
  });
});
