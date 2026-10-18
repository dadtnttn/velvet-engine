# @velvet/core

The core runtime for Velvet JS games. It has five parts:

- `GameLoop`: a fixed-step loop. `update(dt)` runs at 60 Hz by default, and `render(alpha)` runs once per frame.
- `GameClock`: game time fed by the loop, with pause, speed multipliers (1, 2, 3 by default) and days.
- `EventEmitter`: typed events with `on`, `once`, `off` and `emit`.
- `ActionMap`: keys, mouse buttons and the wheel bound to named actions, read with `pressed`, `held` and `released`.
- `createSaveStore`: versioned save slots with migrations. Saves go to the Velvet Desktop bridge, then localStorage, then memory.

The bridge contract (`VelvetBridge` and `getBridge`) lives in `src/bridge.ts`.

## Usage

    import { ActionMap, GameClock, GameLoop, createSaveStore } from "@velvet/core";

    type Action = "confirm" | "move";

    const input = new ActionMap<Action>([
      { action: "confirm", control: { type: "key", key: "Enter" } },
      { action: "confirm", control: { type: "mouse", button: "left" } },
      { action: "move", control: { type: "key", key: "KeyW" }, axisComponent: "y" },
      { action: "move", control: { type: "key", key: "KeyS" }, axisComponent: "y", scale: -1 },
    ]);
    input.attach(window);

    const clock = new GameClock({ onDay: (day) => console.log("day", day) });
    const saves = createSaveStore({ game: "my-game", version: 1 });

    const loop = new GameLoop({
      update(dt) {
        clock.advance(dt);
        if (input.pressed("confirm")) openMenu(); // openMenu, moveHero and draw are game code
        const { x, y } = input.axis2("move");
        moveHero(x, y, dt);
        input.endFrame(); // once per frame: clears pressed and released, and the wheel and pointer deltas
      },
      render(alpha) {
        draw(alpha);
      },
    });
    loop.start();

    await saves.save("slot-1", { gold: 42 });
    const loaded = await saves.load("slot-1"); // null when the slot is empty

## Tests

From `js/`, run `pnpm test` and `pnpm typecheck`. The tests use a fake frame scheduler,
synthetic input events and in-memory storage, so they need no browser.
