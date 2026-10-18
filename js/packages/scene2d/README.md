# @velvet/scene2d

A retained 2D scene for games that are not card games: platformers, top-down and shmups, and visual
novel effects. Nodes (groups, sprites, nine-slices, text and shapes) sit in layers. A camera moves the
world, and each layer has a parallax factor. The scene is drawn with Canvas 2D at the stage's native
pixels, and hit tests use the same transforms as the drawing. Atlases and animations bring sprite sheets
in. The package imports `@velvet/core` for the game clock, and nothing else.

- **Scene** (`createScene2D`, `Scene2D`): logical coordinates (1920 by 1080 by default), `resize` and
  `setPixelScale` for the canvas's own pixels, `followStage` for a ui stage, `render`, `hitTest`, `stats`.
- **Nodes**: `Group`, `Sprite` (an image, a frame of it, flips), `NineSlice` (corners kept, edges
  stretched), `Text` (wrap, alignment, baseline, outline, shadow), `Shape` (rectangles, rounded rectangles,
  circles, ellipses, lines, polygons). Each node has a place, a turn, a scale, a pivot and an anchor, an
  opacity, a visibility, a zIndex, and whether it can be hit.
- **Layers and camera**: a layer with parallax 0 is fixed to the screen, 1 is the world, and values between
  them are backgrounds that move more slowly. The camera has a position, a zoom, a turn, limits, a follow with
  a lerp and a deadzone, and a seeded shake.
- **Atlases and animations**: TexturePacker (hash and array forms, turned and trimmed frames) and Aseprite
  (frames, durations and frame tags with forward, reverse, ping-pong and reversed ping-pong). An animation is
  ticked from the game loop and obeys the game clock.
- **Hit testing**: the topmost visible, interactive node under a point, through the exact inverse of each
  node's transform.
- **Stats**: nodes drawn, subtrees culled, draw calls and the time of the last render, for the devtools.

## Usage

Make the scene next to the clock, and tick it from the game loop:

    import { GameClock, GameLoop } from "@velvet/core";
    import {
      createAnimation,
      createScene2D,
      followStage,
      parseAseprite,
      Sprite,
      Text,
    } from "@velvet/scene2d";

    const clock = new GameClock();
    const canvas = document.createElement("canvas");
    canvas.style.width = "100%";
    canvas.style.height = "100%";
    stage.root.append(canvas);

    const scene = createScene2D({ canvas, clock });
    followStage(scene, stage);                       // draw at the stage's native pixels

    const world = scene.createLayer({ name: "world", parallax: 1 });
    const background = scene.createLayer({ name: "sky", parallax: 0.3 });
    const hud = scene.createLayer({ name: "hud", parallax: 0, zIndex: 10 });

    // knightImage is a loaded image element, and knightJson the parsed Aseprite export; the game's loader gives both.
    const sheet = parseAseprite(knightJson);
    const walk = sheet.tag("walk");
    const knight = new Sprite({
      image: knightImage,
      frame: sheet.frames[0],
      x: 400,
      y: 600,
      anchorX: 0.5,                                  // the pivot is the middle of the feet
      anchorY: 1,
      interactive: true,
    });
    const walking = createAnimation(walk.frames, {
      durations: walk.durations,
      direction: walk.direction,
      clock,
      sprite: knight,
    });
    world.add(knight);

    hud.add(new Text({ text: "Demon Queen", fontSize: 48, fill: "#ffd27a", maxWidth: 800 }));

    scene.camera.x = 960;                            // the world origin, top left, at the screen's top left
    scene.camera.y = 540;
    scene.camera.bounds = { x: 0, y: 0, width: 4000, height: 1080 };
    scene.camera.follow(knight, { lerp: 0.15, deadzone: { width: 240, height: 140 } });

    const loop = new GameLoop({
      update(dt) {
        clock.advance(dt);
        scene.tick(dt);                              // the camera follows and shakes
        walking.tick(dt);                            // the animation runs at the clock's speed
      },
      render() {
        scene.render();
      },
    });
    loop.start();

    // A pointer click: stage.toLogical gives logical pixels, or null on the bars.
    stage.root.addEventListener("pointerdown", (event) => {
      const point = stage.toLogical(event.clientX, event.clientY);
      const hit = point === null ? null : scene.hitTest(point);
      if (hit === knight) scene.camera.shake(8, 0.25);
    });

## Coordinates and the camera

Every position is in logical pixels of the surface (1920 by 1080 unless `width` and `height` say otherwise).
The camera position is the world point at the centre of the surface. A new camera at (0, 0) therefore shows
the world origin in the middle of the screen. To start with the origin in the top-left corner, set the
camera to (960, 540), as the example does.

A layer with parallax `p` is drawn with a view that is `p` of the camera's: it moves `p` times as far as the
camera, turns `p` times as much, and its change of zoom is `p` times the camera's. `scene.camera.viewFor(p,
matrix)` gives that view. The camera converts
between surface points and world points with `screenToWorld` and `worldToScreen`, which are exact inverses,
and which include the shake.

## Native pixels

`followStage(scene, stage)` draws at the pixel size of a ui stage, now and after each change of the layout.
The function only needs `pixelWidth`, `pixelHeight` and `onResize`, so this package does not import
`@velvet/ui`. `scene.resize(pixelWidth, pixelHeight)` does the same for any canvas. The scale `k` (device
pixels per logical pixel) is `scene.pixelScale`. Smoothing is bilinear by default (`smoothingQuality` "low");
a node or a layer with `pixelArt: true` turns it off for its images.

## Time and the clock

The camera (`scene.tick(dt)`) and every animation (`animation.tick(dt)`) take the real step of the loop. A
clock given to `createScene2D` or `createAnimation` scales that step the way `GameClock` does: nothing moves
while the game is paused, and everything moves at the clock's speed. A shake or a follow therefore does not
run during a pause, and it replays the same way when the steps are the same.

## Hit testing

`scene.hitTest(point)` returns the topmost node under a logical point, and `scene.nodesAt(point)` returns
them all, topmost first. A node is hit when it is `interactive`, when it and its parents are visible, and
when its opacity is above zero. The test uses the inverse of the node's own transform, so a turned and
scaled sprite is hit where it is drawn. The node's shape is its box (for a sprite, its untrimmed box), its
geometry (for a shape, the inside, and a line within half its width or 2 logical pixels, whichever is more), or its `hitArea` when that
is set. The hit test refreshes the transforms first, so a node that moved since the last render is hit where
it now is.

## Atlases

`parseTexturePacker(json)` reads the hash or the array form. A turned frame is stored rotated 90 degrees
clockwise, and the reader keeps the flag, so the sprite turns it back. A trimmed frame is drawn back at its
place inside the untrimmed size, so the pivot of an animation does not move when the trim changes.

`parseAseprite(json)` reads the frames, their durations (milliseconds in the file, seconds here) and the
`frameTags`. A tag has its frames, its durations and its direction, and the three together go to
`createAnimation`. Both readers accept the parsed object or the JSON text, and they throw a `TypeError` or a
`RangeError` that names the bad field.

## API

| Export | What it is |
|---|---|
| `createScene2D(options)`, `Scene2D` | The scene. Options: `canvas`, `width`, `height`, `background`, `smoothingQuality`, `clock`, `seed`, `now`. |
| `scene.createLayer(options)`, `removeLayer`, `layers` | Layers, in drawing order (zIndex, then creation). |
| `scene.resize`, `setPixelScale`, `pixelScale`, `canvas` | The backing store and the device-pixel scale. |
| `scene.tick(dt)`, `update()`, `render()` | The camera step, the transform pass, and one frame. |
| `scene.hitTest(point)`, `nodesAt(point)` | The topmost interactive node under a point, or all of them. |
| `scene.stats()` | `{ nodes, culled, drawCalls, renderMs }` for the last render. |
| `Node`, `Group` | The base of the tree: `add`, `remove`, `clear`, `children`, `parent`, `pivotInLayer()`, `layerMatrix`, `surfaceMatrix`. |
| `Node` fields | `name`, `x`, `y`, `rotation`, `scaleX`, `scaleY`, `pivotX`, `pivotY`, `anchorX`, `anchorY`, `alpha`, `visible`, `zIndex`, `interactive`, `pixelArt`, `hitArea`. |
| `Layer` | A group with `parallax`. |
| `Sprite` | `image`, `frame`, `width`, `height`, `flipX`, `flipY`. The image is an `HTMLImageElement`, an `ImageBitmap`, an `HTMLCanvasElement` or an `OffscreenCanvas`. |
| `NineSlice` | `image`, `frame`, `width`, `height`, `insets` (`left`, `top`, `right`, `bottom`, in source pixels). |
| `Text` | `text`, `fontFamily`, `fontSize`, `fontWeight`, `italic`, `fill`, `align`, `baseline`, `maxWidth`, `lineHeight`, `outline`, `shadow`, and `lines` (the laid-out lines). |
| `Shape` | `geometry` (`rect`, `roundRect`, `circle`, `ellipse`, `line`, `polygon`), `fill`, `stroke`, `lineWidth`. |
| `Camera2D` (`scene.camera`) | `x`, `y`, `zoom`, `rotation`, `bounds`, `followTarget`, `follow(target, { lerp, deadzone })`, `unfollow()`, `shake(intensity, seconds)`, `tick(dt)`, `viewFor(parallax, out)`, `screenToWorld(point)`, `worldToScreen(point)`. |
| `createAnimation(frames, options)`, `Animation` | Options: `fps` or `durations`, `loop`, `direction`, `autoplay`, `clock`, `sprite`, `onFrame`, `onComplete`. Methods: `tick`, `play`, `pause`, `reset`, `seek`. Fields: `frame`, `index`, `time`, `duration`, `playing`, `finished`. |
| `parseTexturePacker(json)`, `Atlas` | `count`, `names`, `size`, `get(name)`, `frame(name)`. |
| `parseAseprite(json)`, `AsepriteSheet` | `frames`, `durations`, `tags`, `size`, `tag(name)`. A tag has `name`, `from`, `to`, `direction`, `frames`, `durations`. |
| `followStage(scene, stage)` | Keeps the scene at a stage's pixel size. Returns a function that stops it. |
| `SpriteFrame` | A rectangle of an image: `x`, `y`, `width`, `height`, and optionally `rotated`, `trimX`, `trimY`, `sourceWidth`, `sourceHeight`. |
| `Matrix2D` | The affine matrix that every node keeps, for tools. |

## Limits

- There are no blend modes, masks or clipping regions, and no batching: each node is one or more canvas draw
  calls (a text with an outline and a fill is two).
- A node's hit area is its box, not its alpha. A nine-slice cannot draw a turned frame, and it draws a frame
  from its stored pixels at the top-left of its box.
- The Aseprite `repeat` count of a tag is not read.
- A line or a polygon keeps its points as they are; its anchor is measured from the origin to the far corner
  of its geometry.
- Culling is by box, and the stats count the subtrees that were skipped, not the nodes inside them.
