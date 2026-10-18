// The camera: where the view looks, how far it zooms and turns, the limits it stays within, what it
// follows, and the shake that jolts it. The camera moves only when it is ticked, by the step that its
// clock gives. So pausing the game also pauses the camera, and a replay with the same steps and the same
// seed moves it the same way.
//
// The position is the world point at the centre of the surface. The view zooms and turns about that
// centre. A layer with a parallax factor p gets a view that is p of the camera's: it moves p times as far,
// turns p times as much, and its zoom changes by p times the camera's change of zoom. A factor of 0 pins the
// layer to the surface. The scene asks for each layer's view with `viewFor`.

import type { GameClock } from "@velvet/core";
import {
  clamp,
  isNonNegativeFinite,
  isPositiveFinite,
  Matrix2D,
  type MutablePoint,
  type Point2D,
  type Rect2D,
} from "./math";
import { Node } from "./node";
import { clockStep } from "./time";

/** What a camera can follow: a plain point in world space, or a node, whose pivot is followed in its layer's space. */
export type CameraTarget = Point2D | Node;

/** Options for {@link Camera2D.follow}. */
export interface FollowOptions {
  /**
   * The share of the remaining distance that the camera covers in one frame at 60 Hz, from 0 to 1. A
   * value of 1 snaps to the target. The pace does not depend on the frame rate. Default 1.
   */
  lerp?: number;
  /**
   * A box centred on the camera. The camera moves only when the target leaves the box, and then it moves
   * just far enough to keep the target on the edge. Default: no box.
   */
  deadzone?: { readonly width: number; readonly height: number } | null;
}

/** Options for the camera. The scene makes its camera from its own options. */
export interface CameraOptions {
  /** The surface size in logical pixels: the view centres on half of it. */
  readonly width: number;
  readonly height: number;
  /** A game clock. Its pause and speed apply to the camera. Without one, the camera uses the real step. */
  readonly clock?: GameClock;
  /** The seed of the shake. Equal seeds and equal steps give equal shakes. Default 1. */
  readonly seed?: number;
}

/** The number of frames per second that `lerp` is measured in. */
const LERP_RATE = 60;

/** The camera. The scene owns one, as `scene.camera`. */
export class Camera2D {
  private readonly viewWidth: number;
  private readonly viewHeight: number;
  private readonly clock: GameClock | undefined;
  private readonly random: () => number;
  private posX = 0;
  private posY = 0;
  private zoomLevel = 1;
  private limits: Rect2D | null = null;
  private target: CameraTarget | null = null;
  private followLerp = 1;
  private deadzone: { readonly width: number; readonly height: number } = { width: 0, height: 0 };
  private shakeStrength = 0;
  private shakeLength = 0;
  private shakeTime = 0;
  private shakeX = 0;
  private shakeY = 0;
  private readonly scratch = new Matrix2D();
  private readonly inverseScratch = new Matrix2D();
  /** The rotation in radians, clockwise on the screen. The view turns about the camera's position. */
  rotation = 0;

  /**
   * @param options - see {@link CameraOptions}.
   * @throws RangeError when the size or the seed is not valid.
   */
  constructor(options: CameraOptions) {
    if (!isPositiveFinite(options.width) || !isPositiveFinite(options.height)) {
      throw new RangeError("the camera needs a positive surface size");
    }
    const seed = options.seed ?? 1;
    if (!Number.isFinite(seed)) throw new RangeError(`seed must be a finite number, got ${seed}`);
    this.viewWidth = options.width;
    this.viewHeight = options.height;
    this.clock = options.clock;
    this.random = mulberry32(seed);
  }

  /** The x coordinate of the world point at the centre of the surface. Limits apply to it. */
  get x(): number {
    return this.posX;
  }

  set x(value: number) {
    this.posX = this.clampX(value);
  }

  /** The y coordinate of the world point at the centre of the surface. Limits apply to it. */
  get y(): number {
    return this.posY;
  }

  set y(value: number) {
    this.posY = this.clampY(value);
  }

  /** The zoom: 2 shows half as much world, and 0.5 twice as much. Always above zero. */
  get zoom(): number {
    return this.zoomLevel;
  }

  set zoom(value: number) {
    if (!isPositiveFinite(value)) throw new RangeError(`zoom must be a positive number, got ${value}`);
    this.zoomLevel = value;
    this.posX = this.clampX(this.posX);
    this.posY = this.clampY(this.posY);
  }

  /**
   * The limits of the camera's position, as a world rectangle, or null for none. The view stays inside
   * them, so the camera's position never leaves the box that the view would fit in. A box smaller than
   * the view is centred.
   */
  get bounds(): Rect2D | null {
    return this.limits;
  }

  set bounds(value: Rect2D | null) {
    if (value !== null && !(isPositiveFinite(value.width) && isPositiveFinite(value.height))) {
      throw new RangeError("camera bounds need a positive width and height");
    }
    this.limits = value === null ? null : { x: value.x, y: value.y, width: value.width, height: value.height };
    this.posX = this.clampX(this.posX);
    this.posY = this.clampY(this.posY);
  }

  /** What the camera follows, or null. */
  get followTarget(): CameraTarget | null {
    return this.target;
  }

  /**
   * Makes the camera follow a target. It moves on each tick, toward the target, and it stops at the
   * limits. Following a node uses the pivot of the node as the last update left it.
   * @throws RangeError when `lerp` is not from 0 to 1, or the deadzone has a negative size.
   */
  follow(target: CameraTarget, options: FollowOptions = {}): void {
    const lerp = options.lerp ?? 1;
    if (!(lerp >= 0 && lerp <= 1)) throw new RangeError(`lerp must be from 0 to 1, got ${lerp}`);
    const zone = options.deadzone ?? null;
    if (zone !== null && !(isNonNegativeFinite(zone.width) && isNonNegativeFinite(zone.height))) {
      throw new RangeError("the deadzone needs a width and a height of zero or more");
    }
    this.target = target;
    this.followLerp = lerp;
    this.deadzone = zone === null ? { width: 0, height: 0 } : { width: zone.width, height: zone.height };
  }

  /** Stops the camera following its target. It stays where it is. */
  unfollow(): void {
    this.target = null;
  }

  /**
   * Shakes the view: an offset of up to `intensity` logical pixels, which shrinks to nothing over `seconds`.
   * A new shake replaces the running one only when it is at least as strong at this moment. A shake of zero
   * strength or zero length does nothing.
   * @throws RangeError when `intensity` or `seconds` is negative or not finite.
   */
  shake(intensity: number, seconds: number): void {
    if (!isNonNegativeFinite(intensity)) throw new RangeError(`intensity must be from 0, got ${intensity}`);
    if (!isNonNegativeFinite(seconds)) throw new RangeError(`seconds must be from 0, got ${seconds}`);
    if (intensity === 0 || seconds === 0) return;
    const running = this.shakeLength > this.shakeTime ? this.currentShakeAmplitude() : 0;
    if (intensity < running) return;
    this.shakeStrength = intensity;
    this.shakeLength = seconds;
    this.shakeTime = 0;
    this.applyShake();
  }

  /**
   * Moves the camera by a real step of `dt` seconds, scaled by the clock: it advances the shake, and it
   * moves toward the follow target.
   * @throws RangeError when `dt` is negative or not finite.
   */
  tick(dt: number): void {
    const step = clockStep(dt, this.clock);
    if (step <= 0) return;
    if (this.shakeLength > 0) {
      this.shakeTime += step;
      if (this.shakeTime >= this.shakeLength) {
        this.shakeStrength = 0;
        this.shakeLength = 0;
        this.shakeTime = 0;
        this.shakeX = 0;
        this.shakeY = 0;
      } else {
        this.applyShake();
      }
    }
    const target = this.target;
    if (target !== null) this.followStep(step, target);
  }

  /**
   * Writes the view of a layer with the given parallax factor into `out`, and returns it. The view maps
   * world space to the surface, in logical pixels. The scene calls it once per layer and frame.
   */
  viewFor(parallax: number, out: Matrix2D): Matrix2D {
    const cx = this.posX + this.shakeX;
    const cy = this.posY + this.shakeY;
    const zoom = 1 + (this.zoomLevel - 1) * parallax;
    const angle = this.rotation * parallax;
    const cos = Math.cos(angle);
    const sin = Math.sin(angle);
    const a = zoom * cos;
    const b = zoom * sin;
    const c = -zoom * sin;
    const d = zoom * cos;
    // The view is: move the centre point to the middle of the surface, then turn and zoom about it. The
    // translation below is the centre offset, scaled by the parallax factor.
    const tx = -parallax * cx;
    const ty = -parallax * cy;
    return out.set(
      a,
      b,
      c,
      d,
      parallax * (this.viewWidth / 2) + a * tx + c * ty,
      parallax * (this.viewHeight / 2) + b * tx + d * ty,
    );
  }

  /**
   * Converts a point of the surface (logical pixels) to world space. It is the exact inverse of
   * `worldToScreen`, and it includes the shake. Pass a pointer position from `Stage.toLogical`.
   */
  screenToWorld(point: Point2D): MutablePoint {
    this.viewFor(1, this.scratch);
    this.inverseScratch.invertFrom(this.scratch);
    return this.inverseScratch.apply(point.x, point.y);
  }

  /** Converts a world point to the surface, in logical pixels. The inverse of `screenToWorld`. */
  worldToScreen(point: Point2D): MutablePoint {
    return this.viewFor(1, this.scratch).apply(point.x, point.y);
  }

  /** Moves the camera toward its target by the share that `lerp` gives for one step, outside the deadzone. */
  private followStep(step: number, target: CameraTarget): void {
    const point = target instanceof Node ? target.pivotInLayer() : target;
    const halfWidth = this.deadzone.width / 2;
    const halfHeight = this.deadzone.height / 2;
    let goalX = this.posX;
    let goalY = this.posY;
    const offsetX = point.x - this.posX;
    const offsetY = point.y - this.posY;
    if (offsetX > halfWidth) goalX = point.x - halfWidth;
    else if (offsetX < -halfWidth) goalX = point.x + halfWidth;
    if (offsetY > halfHeight) goalY = point.y - halfHeight;
    else if (offsetY < -halfHeight) goalY = point.y + halfHeight;
    const share = 1 - (1 - this.followLerp) ** (step * LERP_RATE);
    this.x = this.posX + (goalX - this.posX) * share;
    this.y = this.posY + (goalY - this.posY) * share;
  }

  /** The strength of the running shake at this moment. */
  private currentShakeAmplitude(): number {
    return this.shakeStrength * (1 - this.shakeTime / this.shakeLength);
  }

  /** Draws a new offset for the running shake, from its strength at this moment and the seeded random numbers. */
  private applyShake(): void {
    const amplitude = this.currentShakeAmplitude();
    this.shakeX = amplitude * (this.random() * 2 - 1);
    this.shakeY = amplitude * (this.random() * 2 - 1);
  }

  /** The x position, held inside the limits for the current zoom. */
  private clampX(value: number): number {
    const limits = this.limits;
    if (limits === null) return value;
    return clampToLimits(value, limits.x, limits.width, this.viewWidth / 2 / this.zoomLevel);
  }

  /** The y position, held inside the limits for the current zoom. */
  private clampY(value: number): number {
    const limits = this.limits;
    if (limits === null) return value;
    return clampToLimits(value, limits.y, limits.height, this.viewHeight / 2 / this.zoomLevel);
  }
}

/**
 * Holds a camera position inside a range of the world. The position may go as far as `half` from either
 * end of the range, so that the view stays inside it. A range smaller than the view gives its centre.
 */
function clampToLimits(value: number, start: number, length: number, half: number): number {
  const low = start + half;
  const high = start + length - half;
  if (low > high) return start + length / 2;
  return clamp(value, low, high);
}

/**
 * A seeded random number generator (mulberry32). It returns numbers from 0 up to, but not including, 1, and
 * the same seed always gives the same sequence.
 */
function mulberry32(seed: number): () => number {
  let state = seed >>> 0;
  return () => {
    state = (state + 0x6d2b79f5) >>> 0;
    let t = state;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}
