// The drawing of the level, on a scene of @velvet/scene2d at the stage's native pixels. There are three layers: the
// sky, which holds the scenery and moves at a parallax factor; the world, which holds the tiles (drawn by
// @velvet/tiles through a node of its own), the coins, the checkpoints, the goal and the hero; and the screen,
// which holds the score line, the key hints and the red flash of a hit. The view reads a run and draws it. It
// changes nothing in the run, and it has no rules of its own.

import type { GameClock } from "@velvet/core";
import {
  Animation,
  Group,
  Matrix2D,
  Node,
  Shape,
  Sprite,
  Text,
  createAnimation,
  createScene2D,
  followStage,
  type Context2D,
  type Scene2D,
  type Scene2DCanvas,
  type SpriteFrame,
  type StageSource,
} from "@velvet/scene2d";
import {
  drawTileLayer,
  findLayer,
  type TileImages,
  type TileLayer,
  type TileMap,
  type TileView,
} from "@velvet/tiles";
import { tween, type Ticker } from "@velvet/tween";
import type { Art } from "./art";
import { tileFrame } from "./art";
import { CAMERA, HERO, HURT_FLASH_SECONDS, LAYERS, SKY, SURFACE, TILE_IDS, TILESET_NAME } from "./config";
import type { Level } from "./level";
import type { Run } from "./world";

/** A translation: the game's text lookup, with the holes filled from `args`. */
export type TranslateFn = (key: string, args?: Readonly<Record<string, string | number>>) => string;

/** The part of a draw pass that a node in this view uses. The scene's own pass has all of it. */
interface PassLike {
  readonly context: Context2D;
  readonly surfaceWidth: number;
  readonly surfaceHeight: number;
  place(surface: Readonly<Matrix2D>): void;
  alpha(value: number): void;
  smooth(on: boolean): void;
  count(): void;
}

/** The box of a node's own content, in the node's units. */
interface BoxLike {
  x: number;
  y: number;
  width: number;
  height: number;
}

/**
 * A layer of tiles of the map, drawn in the node's own units, which are level pixels. The node covers the whole
 * map for culling, and it draws only the tiles that the surface shows.
 */
class TileLayerNode extends Node {
  private readonly map: TileMap;
  private readonly layer: TileLayer;
  private readonly images: TileImages;
  private readonly now: () => number;
  private readonly inverse = new Matrix2D();

  /**
   * @param map - the map that the layer belongs to.
   * @param layer - the tile layer to draw.
   * @param images - the picture of each tileset, by name.
   * @param now - the time in milliseconds, for the animated tiles.
   */
  constructor(map: TileMap, layer: TileLayer, images: TileImages, now: () => number) {
    super({ name: `tiles:${layer.name}`, pixelArt: true });
    this.map = map;
    this.layer = layer;
    this.images = images;
    this.now = now;
  }

  /** The whole map, so that the node is culled only when none of it shows. */
  protected override contentRect(out: BoxLike): boolean {
    out.x = 0;
    out.y = 0;
    out.width = this.map.width * this.map.tileWidth;
    out.height = this.map.height * this.map.tileHeight;
    return true;
  }

  /** Draws the visible tiles of the layer, on the scene's transform. */
  protected override drawSelf(pass: PassLike, alpha: number, pixelArt: boolean): void {
    pass.place(this.surfaceMatrix);
    pass.alpha(alpha);
    pass.smooth(!pixelArt);
    // The tiles snap to device pixels (drawTileLayer reads the context's transform), so neighbours leave no seam.
    drawTileLayer(pass.context, this.map, this.layer, this.images, this.visibleArea(pass), this.now());
    pass.count();
  }

  /** The part of the map that the surface shows, in the node's units: the surface's corners, mapped back. */
  private visibleArea(pass: PassLike): TileView {
    if (!this.inverse.invertFrom(this.surfaceMatrix)) return { x: 0, y: 0, width: 0, height: 0 };
    const corners: ReadonlyArray<readonly [number, number]> = [
      [0, 0],
      [pass.surfaceWidth, 0],
      [0, pass.surfaceHeight],
      [pass.surfaceWidth, pass.surfaceHeight],
    ];
    let left = Infinity;
    let top = Infinity;
    let right = -Infinity;
    let bottom = -Infinity;
    for (const [x, y] of corners) {
      const point = this.inverse.apply(x, y);
      left = Math.min(left, point.x);
      top = Math.min(top, point.y);
      right = Math.max(right, point.x);
      bottom = Math.max(bottom, point.y);
    }
    return { x: left, y: top, width: right - left, height: bottom - top };
  }
}

/** The poses of the hero, which pick its frames. */
type Pose = "idle" | "run" | "jump" | "fall";

/** What the view needs from the game to draw the level. */
export interface ViewOptions {
  /** The canvas that the scene draws into. The game appends it to the page. */
  readonly canvas: Scene2DCanvas;
  /** The game clock: the animations and the camera obey its pause and speed. */
  readonly clock: GameClock;
  /** The ticker of the game's motion, for the pickup pops and the flash. */
  readonly motion: Ticker;
  /** The level, as the map and its objects give it. */
  readonly level: Level;
  /** The pictures and sheets. */
  readonly art: Art;
  /** The game's text lookup, for the score line and the hints. */
  readonly translate: TranslateFn;
}

/** The game's picture of a level: a scene with three layers, the camera, and the screen's own texts. */
export class GameView {
  /** The scene that draws the level. */
  readonly scene: Scene2D;
  private readonly level: Level;
  private readonly motion: Ticker;
  private readonly translate: TranslateFn;
  private readonly focus = { x: 0, y: 0 };
  private readonly images: TileImages;
  private readonly hero: Sprite;
  private readonly heroFrames: { readonly idle: SpriteFrame; readonly jump: SpriteFrame; readonly fall: SpriteFrame };
  private readonly idleAnimation: Animation;
  private readonly runAnimation: Animation;
  private readonly coinSprites: Sprite[] = [];
  private readonly coinAnimations: Animation[] = [];
  private readonly checkpointPennants: Shape[] = [];
  private readonly flash: Shape;
  private readonly coinsText: Text;
  private readonly timeText: Text;
  private pose: Pose | null = null;

  /**
   * Builds the scene: its layers, the sprites of the level and the screen's texts. The camera starts on the hero.
   * @param options - see {@link ViewOptions}.
   */
  constructor(options: ViewOptions) {
    this.level = options.level;
    this.motion = options.motion;
    this.translate = options.translate;
    const map = options.level.map;
    const tileset = map.tilesets.find((candidate) => candidate.name === TILESET_NAME);
    if (tileset === undefined) throw new Error(`the map has no tileset named ${TILESET_NAME}`);
    this.images = { [TILESET_NAME]: options.art.tiles };

    this.scene = createScene2D({
      canvas: options.canvas,
      width: SURFACE.width,
      height: SURFACE.height,
      background: SKY,
      clock: options.clock,
    });
    this.scene.camera.zoom = CAMERA.zoom;
    this.scene.camera.bounds = { x: 0, y: 0, width: options.level.width, height: options.level.height };

    const now = () => options.clock.elapsed * 1000;
    const sky = this.scene.createLayer({ name: "sky", parallax: 0.4, zIndex: 0 });
    const world = this.scene.createLayer({ name: "world", parallax: 1, zIndex: 1 });
    const screen = this.scene.createLayer({ name: "screen", parallax: 0, zIndex: 2 });

    sky.add(new TileLayerNode(map, tileLayer(map, LAYERS.background), this.images, now));
    for (const name of [LAYERS.ground, LAYERS.platforms, LAYERS.hazards]) {
      world.add(new TileLayerNode(map, tileLayer(map, name), this.images, now));
    }

    world.add(
      new Sprite({
        name: "goal",
        image: options.art.tiles,
        frame: tileFrame(tileset, TILE_IDS.flag),
        x: options.level.goal.x,
        y: options.level.goal.y,
        anchorX: 0.5,
        anchorY: 1,
        zIndex: 3,
        pixelArt: true,
      }),
    );

    const spin = options.art.coinSheet.tag("spin");
    for (const coin of options.level.coins) {
      const sprite = new Sprite({
        name: "coin",
        image: options.art.coins,
        frame: spin.frames[0] ?? null,
        x: coin.x,
        y: coin.y,
        anchorX: 0.5,
        anchorY: 0.5,
        zIndex: 2,
        pixelArt: true,
      });
      this.coinSprites.push(sprite);
      this.coinAnimations.push(createAnimation(spin.frames, { durations: spin.durations, clock: options.clock, sprite }));
      world.add(sprite);
    }

    options.level.checkpoints.forEach((point, index) => {
      if (index === 0) return;
      const pennant = new Shape({
        name: "pennant",
        geometry: {
          kind: "polygon",
          points: [
            { x: 2, y: -56 },
            { x: 26, y: -46 },
            { x: 2, y: -36 },
          ],
        },
        fill: "#8a8f9c",
        zIndex: 2,
      });
      this.checkpointPennants.push(pennant);
      const checkpoint = new Group({ name: "checkpoint", x: point.x, y: point.y, zIndex: 2 });
      checkpoint.add(new Shape({ name: "pole", geometry: { kind: "rect", width: 4, height: 56 }, x: -2, y: -56, fill: "#5b5f6e" }), pennant);
      world.add(checkpoint);
    });

    const heroAtlas = options.art.heroAtlas;
    const idleFrames = [heroAtlas.frame("idle_0"), heroAtlas.frame("idle_1")];
    const runFrames = ["run_0", "run_1", "run_2", "run_3"].map((name) => heroAtlas.frame(name));
    this.heroFrames = {
      idle: heroAtlas.frame("idle_0"),
      jump: heroAtlas.frame("jump_0"),
      fall: heroAtlas.frame("fall_0"),
    };
    this.hero = new Sprite({
      name: "hero",
      image: options.art.hero,
      frame: this.heroFrames.idle,
      anchorX: 0.5,
      anchorY: 1,
      zIndex: 5,
      pixelArt: true,
    });
    this.idleAnimation = createAnimation(idleFrames, { durations: [0.6, 0.6], clock: options.clock, sprite: this.hero });
    this.runAnimation = createAnimation(runFrames, { fps: 10, clock: options.clock, sprite: this.hero, autoplay: false });
    world.add(this.hero);

    this.flash = new Shape({ name: "flash", geometry: { kind: "rect", width: SURFACE.width, height: SURFACE.height }, fill: "#ff3b5c", alpha: 0 });
    this.coinsText = new Text({ x: 96, y: 36, fontSize: 40, fontWeight: 800, fill: "#fff7d6", outline: { color: "#2b1d24", width: 6 } });
    this.timeText = new Text({ x: 96, y: 92, fontSize: 30, fontWeight: 700, fill: "#ffffff", outline: { color: "#2b1d24", width: 5 } });
    const coinIcon = new Sprite({
      name: "coin-icon",
      image: options.art.tiles,
      frame: tileFrame(tileset, TILE_IDS.coin),
      x: 40,
      y: 30,
      width: 44,
      height: 44,
      pixelArt: true,
    });
    const levelName = new Text({
      x: SURFACE.width - 48,
      y: 36,
      anchorX: 1,
      align: "right",
      fontSize: 30,
      fontWeight: 700,
      fill: "#ffffff",
      outline: { color: "#2b1d24", width: 5 },
    });
    const hint = new Text({
      x: 40,
      y: SURFACE.height - 32,
      anchorY: 1,
      fontSize: 26,
      fill: "#ffffff",
      outline: { color: "#2b1d24", width: 4 },
    });
    levelName.text = options.translate("hud.level");
    hint.text = options.translate("hud.hint");
    screen.add(this.flash, coinIcon, this.coinsText, this.timeText, levelName, hint);

    this.focus.x = options.level.spawn.x;
    this.focus.y = options.level.spawn.y - HERO.height / 2;
    this.scene.camera.follow(this.focus, { lerp: CAMERA.lerp, deadzone: CAMERA.deadzone });
    this.snapCamera();
  }

  /**
   * Keeps the scene at the stage's native pixels, now and after each change of the layout.
   * @param stage - the stage that the level is shown in.
   * @returns a function that stops following the stage.
   */
  attach(stage: StageSource): () => void {
    return followStage(this.scene, stage);
  }

  /** Moves the camera to the hero at once, without the glide: after a start, a restart or a respawn. */
  snapCamera(): void {
    this.scene.camera.x = this.focus.x;
    this.scene.camera.y = this.focus.y;
  }

  /**
   * Advances the animations and the camera by one fixed step, and writes the run's positions and texts to the
   * sprites. The camera follows the hero, with the clock's pause and speed.
   * @param dt - the length of the step, in seconds.
   * @param run - the run to show.
   */
  update(dt: number, run: Run): void {
    this.idleAnimation.tick(dt);
    this.runAnimation.tick(dt);
    for (const animation of this.coinAnimations) animation.tick(dt);

    const hero = run.hero;
    this.focus.x = hero.x + HERO.width / 2;
    this.focus.y = hero.y + HERO.height / 2;
    this.hero.x = hero.x + HERO.width / 2;
    this.hero.y = hero.y + HERO.height;
    this.hero.flipX = hero.facing < 0;
    this.setPose(hero.grounded ? (Math.abs(hero.vx) > 20 ? "run" : "idle") : hero.vy < 0 ? "jump" : "fall");
    this.hero.visible = run.phase !== "hurt" || Math.floor(run.elapsed * 12) % 2 === 0;

    this.checkpointPennants.forEach((pennant, index) => {
      pennant.fill = index + 1 <= run.checkpoint ? "#ffd23f" : "#8a8f9c";
    });
    this.coinsText.text = this.translate("hud.coins", { coins: run.coins, total: this.level.coins.length });
    this.timeText.text = this.translate("hud.time", { time: formatClock(run.elapsed) });

    this.scene.tick(dt);
  }

  /** Draws one frame of the scene. */
  render(): void {
    this.scene.render();
  }

  /**
   * Makes a coin pop and vanish, as it is picked up.
   * @param index - the coin's index in the level.
   */
  pickup(index: number): void {
    const sprite = this.coinSprites[index];
    this.coinAnimations[index]?.pause();
    if (sprite === undefined) return;
    tween({
      from: 0,
      to: 1,
      duration: 0.25,
      ease: "quadOut",
      ticker: this.motion,
      set: (value) => {
        sprite.scaleX = 1 + value * 0.8;
        sprite.scaleY = 1 + value * 0.8;
        sprite.alpha = 1 - value;
      },
      onComplete: () => {
        sprite.visible = false;
      },
    });
  }

  /** Shows the red flash of a hit, and shakes the camera. */
  hit(): void {
    this.scene.camera.shake(CAMERA.shake.intensity, CAMERA.shake.seconds);
    tween({
      from: 0.45,
      to: 0,
      duration: HURT_FLASH_SECONDS,
      ease: "quadOut",
      ticker: this.motion,
      set: (value) => {
        this.flash.alpha = value;
      },
    });
  }

  /**
   * Plays the hero's pose: the idle and the run are the animations; the jump and the fall are single frames.
   * @param pose - the pose now.
   */
  private setPose(pose: Pose): void {
    if (pose === this.pose) return;
    this.pose = pose;
    this.idleAnimation.pause();
    this.runAnimation.pause();
    if (pose === "idle") this.idleAnimation.play();
    else if (pose === "run") this.runAnimation.play();
    else this.hero.frame = pose === "jump" ? this.heroFrames.jump : this.heroFrames.fall;
  }
}

/**
 * The tile layer of the map with a name.
 * @param map - the map.
 * @param name - the layer's name.
 * @throws when the map has no tile layer with that name.
 */
function tileLayer(map: TileMap, name: string): TileLayer {
  const layer = findLayer(map, name);
  if (layer === undefined || layer.kind !== "tiles") throw new Error(`the map has no tile layer named ${name}`);
  return layer;
}

/**
 * The clock of the score line: minutes, seconds and tenths.
 * @param seconds - the time, from 0 up.
 */
function formatClock(seconds: number): string {
  const tenths = Math.max(0, Math.floor(seconds * 10));
  const minutes = Math.floor(tenths / 600);
  const wholeSeconds = Math.floor((tenths % 600) / 10);
  return `${minutes}:${String(wholeSeconds).padStart(2, "0")}.${tenths % 10}`;
}
