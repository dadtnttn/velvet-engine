# @velvet/testing

Tools to test and debug any Velvet JS game. A game's loop runs by hand on a manual clock, scripted
input reaches the game's action maps the way a player's input does, a session can be recorded and
replayed to the same state hashes, DOM and state snapshots compare stable text, and soak runs play many
seeds with a bot and report the numbers.

Nothing here knows about a particular game. The game supplies its own loop, its own action maps and its
own `hash()`. The replay format is documented in `docs/formats/REPLAY.md`, with its conformance fixtures in
`docs/formats/fixtures/replay`.

## The parts

| Part | What it does | Main entry points |
|---|---|---|
| Driver | Runs a loop frame by frame on a manual clock: `step`, `advance`, `advanceGame` (game time, with the clock's speed and pause), `until`. | `createDriver`, `Driver` |
| Scripted input | Keys, pointer moves and buttons in logical stage pixels, wheel ticks, blur, and gamepads, on a timeline. | `InputScript`, `driver.play`, `driver.keyDown` |
| Recording and replay | Records a session as `velvet-replay` v1, and plays it back, checking the hash at every checkpoint. | `createRecorder`, `checkReplay`, `startReplay`, `parseReplay` |
| Snapshots | Stable DOM text, stable state text and state hashes, with a comparison that shows the lines that differ. | `serializeDom`, `compareSnapshot`, `stableStringify`, `hashState` |
| Soak runs | Many seeds and frames with a bot strategy; a report of metrics, chosen values, errors, frame times and final hashes. | `runSoak` |
| Fakes | A fake `navigator.getGamepads`, installed by the driver while it lives. | `FakeGamepads`, `readNavigatorPads` |

## The loop on a driver

The driver owns the frames. Each frame runs the loop's update once, with the fixed step, so the same
inputs give the same states every time. A frame is one update, so the driver's rate must be the loop's
rate: `driver.createLoop` builds the loop with it and checks each frame.

```ts
import { ActionMap, GameClock } from "@velvet/core";
import { InputScript, createDriver } from "@velvet/testing";
import { expect, it } from "vitest";

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
    script.at(5).tap("Space");           // one frame, five frames from now
    script.at(9).press("Space");         // held for three frames
    driver.play(script);
    driver.until(() => driver.frame >= 20);
    expect(coins).toBe(3);
  } finally {
    driver.dispose();
  }
});
```

`until` checks its predicate before each frame and after the last one, and gives up with a `timeout`
error after `maxFrames` (3600 by default). `advance(seconds)` runs `seconds * rate` frames. `advanceGame(seconds)`
runs until the game clock has moved on by that much game time, so a clock at speed 2 needs half the frames,
and a paused clock stops with a `clock-stalled` error.

`driver.dispose()` puts back the navigator and any page globals that the driver replaced. Always dispose a
driver when its test ends.

## Scripted input

Positions are logical (1920 by 1080 by default), so a script works at any window size. The stage is
letterboxed into the area that the stage map gives (see `createStageMap`).

```ts
const script = new InputScript();
script.at(0).padConnect({ pad: 0, id: "Test pad" });
script.at(3).press("Space", { holdFrames: 5 });   // down at 3, up at 8
script.at(12).click({ x: 960, y: 540 });          // a left click, two frames long
script.at(20).drag({ x: 100, y: 100 }, { x: 600, y: 400 }, 30); // frames + 3 frames in all
script.at(40).padButton("a", { holdFrames: 6 });
script.at(50).padAxis("leftX", 1, { frames: 20 }); // the stick ramps to full over 20 frames
script.at(70).wheel(120);
script.at(75).blur();
driver.play(script);
```

Frames count from the frame where the script is played. `driver.play` returns a handle with `done` and
`cancel()`. A press with a `skipped` flag stands for a press that the page had already used (a press on a
text field or a button); no action map counts it, and a replay reproduces that.

## Recording and replay

A recorder keeps a session: the inputs of each frame, the pad states, the named seeds, the starting save,
and the game's hash at every checkpoint. A replay starts the game from the same seeds and save, feeds the
same inputs to the same frames, and compares the hash at each checkpoint. The first checkpoint that differs
ends the replay.

```ts
import { createRecorder, createDriver, checkReplay, parseReplayText, serializeReplay } from "@velvet/testing";

// Record. `startGame` is the game's own code: it builds the loop with driver.createLoop and attaches
// the game's action maps to driver.target.
const driver = createDriver();
let game: GameUnderTest | null = null;
const recorder = createRecorder({
  game: "my-game",
  seeds: { board: seed },
  checkEvery: 60,
  hash: () => (game === null ? "" : game.hash()),
});
recorder.attach(driver);
game = startGame(driver, seed);
driver.play(script);
driver.step(600);
const text = serializeReplay(recorder.toReplay()); // write this to a .json file

// Replay (in a later test, or in CI).
const replay = parseReplayText(text);
const result = await checkReplay(replay, ({ driver, seeds, save }) => {
  const started = startGame(driver, Number(seeds["board"]), save);
  return { hash: () => started.hash() };
});
expect(result.ok).toBe(true);
// On a difference: result.frame is the first checkpoint that differs, and result.previousFrame the last one that matched.
```

`startGame` takes the seeds from the replay, so the game must build its random sources from them. The
recorder's `hash` must return the same text for the same state every time: `hashState` gives one, or the
game can hash its own fields.

## Replaying a bug in a page

A page records the player's real input (trusted events only) by wrapping its loop and observing the window.
The recording is saved as JSON, and the bug can then be replayed in the same page, paced at the display's rate.

Recording, in a development build:

```ts
import { GameLoop, createRecorder, serializeReplay } from "@velvet/testing";

const recorder = createRecorder({
  game: "my-game",
  seeds: { board: seed },
  hash: () => game.hash(),
  stage: stageMapOf(stageElement), // the logical stage, from the element that the game draws in
});
recorder.observe(window);
const loop = new GameLoop(recorder.wrap({ update, render }));
loop.start();
// When the bug shows up, offer serializeReplay(recorder.toReplay()) as a file to download.
```

Replaying, in the page:

```ts
import { createDriver, parseReplayText, startReplay } from "@velvet/testing";

const replay = parseReplayText(await file.text());
const driver = createDriver({ rate: replay.rate, stage: stageMapOf(stageElement) });
driver.takeOverFrames(); // before the game builds its loop: the loop's frames now come from the driver
const run = await startReplay(replay, ({ driver, seeds }) => {
  const started = startGame(driver, Number(seeds["board"]));
  return { hash: () => started.hash() };
}, { driver });
const timer = setInterval(() => {
  run.step(1); // one frame per tick, so the bug plays at the display's pace
  if (run.done) {
    clearInterval(timer);
    run.dispose(); // stops the replay's listener; the driver was passed in, so it is not disposed here
    driver.dispose(); // puts the page's frames and gamepad API back
    showResult(run.result);
  }
}, 1000 / replay.rate);
```

`stageMapOf` is the game's own function: `createStageMap({ area: () => element.getBoundingClientRect() })`.
Use `setInterval` for the pacing, not `requestAnimationFrame`, because the driver takes that over.

## Snapshots

```ts
import { compareSnapshot, serializeDom, stableStringify } from "@velvet/testing";

// Vitest stores and updates the file: `vitest -u` rewrites it.
expect(serializeDom(menuRoot)).toMatchFileSnapshot("./snapshots/menu.txt");
expect(stableStringify(game.state, { decimals: 3 })).toMatchFileSnapshot("./snapshots/state.json");

// Outside Vitest, compare with a text that you stored, and print the lines that differ.
const comparison = compareSnapshot(serializeDom(menuRoot), storedText);
if (!comparison.match) console.log(comparison.diff);
```

`serializeDom` sorts the attributes, leaves out the volatile ones (generated ids and the references to them,
unless the `drop` option names its own list) and collapses whitespace. `stableStringify` sorts the keys and
rounds the numbers, so a state built in another order gives the same text.

## Soak runs

A soak run plays many seeds for many frames with a bot strategy, and reports what happened. Each seed gets
a new driver and a new game.

```ts
import { runSoak } from "@velvet/testing";

const report = await runSoak({
  seeds: { count: 200, from: 1 },
  frames: 3600,
  create: ({ seed, driver }) => {
    const game = startGame(driver, seed);
    return { api: game, hash: () => game.hash(), dispose: () => game.dispose() };
  },
  strategy: ({ frame, random, driver, choose }) => {
    if (frame % 90 === 0) driver.keyDown("Space");
    if (frame % 90 === 3) driver.keyUp("Space");
    choose("reward", random.int(1, 20)); // every chosen value is summarised in the report
  },
  metrics: { gold: (game) => game.gold },
});
// report.chosen.reward (min, max, p50, p90, p99), report.metrics.gold, report.errors,
// report.frameTimeMs and report.finalHashes (one per seed; equal seeds give equal hashes).
```

A strategy that throws ends its seed's run, and the error is listed with its seed and frame. Set
`stopOnError: false` to go on with the next frame instead.

## Notes

- Replays need the same stage as the recording. The recording stores logical positions, and a replay turns
  them back into client positions with the stage it is given.
- A trusted key or button press that the game marks with `markHandled` is recorded as a counted press:
  the core keeps that mark private, so the recorder cannot see it. A replay of such a press diverges at the
  first checkpoint after it. Pad menu keys are not affected, since they are synthetic events that the
  recorder ignores, and the pads themselves are recorded.
- The core reads the gamepads in `endFrame`, so a pad change shows from the next update on.
- Install one driver's frame takeover at a time. Disposing drivers in any order is safe for the gamepads,
  but two takeovers of `requestAnimationFrame` must be undone in reverse order.
- A replay counts its frames from 0, so pass it a new driver, not one that has run frames.
- Hashes depend on the game's arithmetic. Two runs on the same engine give the same hashes; compare hashes
  across engines only after checking that the game's math is the same on both.

## Tests

From `js/`, `pnpm test` runs this package's tests with the rest of the workspace. The tests use happy-dom
where a page is needed (`page.test.ts`, `snapshot.test.ts`) and plain Node elsewhere. `test-support.ts` holds
a small fake game with a loop, a clock, an action map and a seeded random source, which the replay tests
record and replay.
