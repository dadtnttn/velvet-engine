# @velvet/tween

Motion for any game or tool: easings, tweens, timelines and springs, run by a ticker that follows
the core's game clock. A ticker with no clock runs in real time, which is what UI needs while the
game is paused. Helpers animate the `transform` and `opacity` of DOM elements, once per frame.

- **Easings** (`ease`, `cubicBezier`, `steps` and the named families such as `quadOut`): functions from
  progress to eased progress. `cubicBezier` and `steps` follow the CSS definitions, and `ease(name)`
  looks a built-in easing up by name, for data files.
- **Tweens** (`tween`): one value from `from` to `to`, for a number, an array, an object of numbers or
  a colour text (`#rgb`, `#rrggbb`, `rgb()`, `rgba()`). Each time the value is drawn it goes to `set`.
  Colours are mixed in OKLab, so they pass through the colours a person expects in between.
- **Timelines** (`timeline`): tweens, timelines and calls on one time axis, with labels and relative
  positions. A timeline plays, reverses, changes time scale, and scrubs to any time.
- **Springs** (`spring`, `criticalDamping`): a value that follows a goal under stiffness, damping and
  mass. It settles exactly on the goal.
- **Tickers** (`Ticker`, `sharedTicker`, `createRealtimeDriver`): they run the playables.
- **DOM helpers** (`animateElement`, `placeElement`, `isReducedMotion`): transform and opacity only,
  written once per frame, and a jump to the end state for users who ask for reduced motion.

## Usage

Make the tickers in the game's loop, next to the clock:

    import { GameClock, GameLoop } from "@velvet/core";
    import { Ticker, tween, timeline, spring, animateElement, quadOut } from "@velvet/tween";

    const clock = new GameClock();
    const motion = new Ticker({ clock }); // game time: stops when the clock is paused, runs at its speed
    const ui = new Ticker();              // real time: keeps moving while the game is paused

    const loop = new GameLoop({
      update(dt) {
        clock.advance(dt);
        motion.advance(dt);
        ui.advance(dt);
      },
      render(alpha) {
        // draw here
      },
    });
    loop.start();

    // A number, written to a setter on every step.
    const door = tween({
      from: 0,
      to: 1,
      duration: 0.6,
      ease: quadOut,
      ticker: motion,
      set: (value) => {
        doorEl.style.opacity = String(value);
      },
    });
    await door.finished; // "complete", or "cancelled" when it is cancelled

    // A timeline: a label, a child that starts at the label, and a child that runs with it.
    const intro = timeline({ ticker: motion, onComplete: () => console.log("intro done") })
      .add(fadeIn)
      .label("title", "+=0.2")
      .add(slideTitle, "title")
      .add(glow, "title");
    intro.seek(0.5); // scrub: the state at 0.5 s, with no callbacks
    intro.play();

    // A value that follows the pointer, with a spring on the UI ticker.
    const follower = spring({
      ticker: ui,
      stiffness: 200,
      onUpdate: (x) => {
        panelEl.style.left = `${x}px`;
      },
    });
    follower.to = pointerX;

    // An element: transform and opacity, batched with every other animation of the frame.
    animateElement(card, { x: 120, opacity: 0 }, { duration: 0.3, ticker: ui, ease: "quadOut" });

## Time and the clock

A playable moves by the step its ticker is given. A ticker made with a `GameClock` multiplies that step
by the clock's speed, and gives it nothing while the clock is paused. The core loop's step is the one
that the game gives both, so the two stay in step. A ticker made without a clock uses the step as it
is.

Every playable's state is a pure function of its own time. A tween at 0.5 s shows the same value
whether it reached that time in one step or in a hundred, and a timeline can be scrubbed to any time
in any order. `seek` draws a state and runs no completion callbacks. Callbacks run when the playhead
passes them while playing.

`Ticker.advance` does nothing with a step of zero, except that it still runs the frame-end callbacks,
so a write that was queued while the clock was paused still reaches the screen.

## Tweens

    const value = tween({ from: { x: 0, y: 0 }, to: { x: 100, y: 40 }, duration: 1, ticker: motion, set: place });

| Option | Meaning |
|---|---|
| `from`, `to` | The two ends. They must be the same kind: numbers, arrays of numbers, objects of numbers with the same keys, or colour texts. |
| `duration` | The length of one cycle, in seconds. |
| `delay` | Seconds to wait first. The value stays at `from` during the wait. |
| `ease` | A function, or a name from `ease(name)`. Default `"linear"`. |
| `repeat` | Extra cycles. `Infinity` repeats forever. |
| `yoyo` | Every other cycle plays backward, so the value ends back at `from`. |
| `set`, `onUpdate` | Called with each value that is drawn. `set` is for writing; `onUpdate` runs after it. |
| `onComplete` | Runs once, when the tween reaches its end. |
| `ticker`, `target`, `tag`, `timeScale` | Where it runs; what it moves, for `killTweensOf`; a name, for `killTag`; a speed factor. |

The controls are `play()`, `reverse()`, `pause()`, `resume()`, `seek(time)`, `progress` (a fraction;
setting it seeks), `cancel()` and `timeScale`. `finished` resolves once, with `"complete"` or
`"cancelled"`. A completed tween plays again after `seek(0)` and `resume()`.

Overshooting easings (`backOut`, `elasticOut`) make the value go past the ends, and it is extrapolated
there. Colours are returned as `rgb(r, g, b)` or `rgba(r, g, b, a)` text, and the two ends are returned
exactly as they were given.

## Timelines

    const scene = timeline({ ticker: motion });
    scene.add(a);             // at the end of what is placed so far: a sequence
    scene.add(b, 0);          // at 0: parallel with a
    scene.label("beat", 1.5); // a named time
    scene.add(c, "beat");     // at a label
    scene.add(d, "+=0.5");    // 0.5 s after the end; "-=0.5" is 0.5 s before the end
    scene.add(e, "<");        // with the previous child's start; "<0.2" shifts it; ">" uses its end
    scene.call(() => sfx.play(), "beat"); // runs when the playhead passes it, in either direction

A child joins one timeline only, and a timeline cannot hold itself. Children are moved by the timeline,
so they do not run on their own ticker. Nested timelines work the same way. For an editor's timeline view, `spans()` lists the items with their
starts and durations, `labelTimes()` lists the labels, and `duration` is the end of the last item.

## Springs

`spring({ to, from, velocity, stiffness, mass, damping, restDelta, restSpeed, onUpdate, onComplete })`.
The defaults are stiffness 170, mass 1, and critical damping, which reaches the goal without overshoot.
A spring settles when it is within `restDelta` of the goal and slower than `restSpeed`: it then sets
the goal exactly, leaves its ticker and completes. Changing `to` wakes a settled spring. Motion is
sub-stepped at no more than 1/240 s (less for a stiff spring), so a long frame is stable. The cost
grows with the step, so drive a spring with frames, not with one long step.

## DOM helpers

`animateElement(el, to, options)` animates the parts of `to` that it is given: `x` and `y` in pixels,
`scale`, `rotate` in degrees, and `opacity`. The parts that are left out keep their values, and they
persist, so one animation can move `x` while another changes `opacity`. Each element writes its
`transform` and `opacity` once per frame of its ticker, however many animations move it. Opacity is
clamped to 0 to 1 when it is written.

With `prefers-reduced-motion: reduce`, the element jumps to its end state at once. Pass
`skipIfReduced: false` to animate anyway. `placeElement(el, motion)` sets a motion with no animation,
and `isReducedMotion()` reads the setting.

## Tests

From `js/`, run `pnpm test` for the whole workspace, or `pnpm exec vitest run packages/tween` for this
package alone. The tests use fake time: each test has its own ticker and advances it by hand, and a
fake frame scheduler drives the real-time driver. The DOM tests run in happy-dom, and check the easings
against the CSS keywords, the timelines by scrubbing, the clock's pause and speed, the spring's
stability, and the frame-by-frame writes of the DOM helpers.

## Not included

- Named colours (`"red"`) and `hsl()` text. Only hex and `rgb()` / `rgba()` are read.
- Keyframes inside one tween. Use a timeline of tweens for that.
- A `requestAnimationFrame` loop of its own for the game. The game's loop calls `advance`, and the
  realtime driver is only for a ticker that has no loop.
