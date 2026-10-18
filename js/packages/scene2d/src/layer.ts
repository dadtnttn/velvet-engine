// Layers: the top-level groups of a scene. Each layer has a parallax factor that decides how much the
// camera moves it. A factor of 1 is the world: the layer moves with the camera. A factor of 0 is the
// screen: the layer stays where it is, as a HUD does. Values in between make backgrounds that move more
// slowly than the world. A layer also has its own visibility, opacity and zIndex, and it draws in zIndex
// order, then in the order it was created.

import { Matrix2D } from "./math";
import { Group, type NodeOptions } from "./node";

/** Options for {@link Layer}. */
export interface LayerOptions extends NodeOptions {
  /**
   * How much the camera moves the layer: 1 is the world, 0 is the screen, and values between them
   * are parallax. Default 1.
   */
  parallax?: number;
}

/** A top-level group of nodes with a parallax factor. Create it with `Scene2D.createLayer`. */
export class Layer extends Group {
  /** How much the camera moves the layer. */
  parallax: number;
  /** The view that the camera gives this layer. The scene writes it at each update. */
  readonly viewMatrix = new Matrix2D();

  /**
   * @param options - see {@link LayerOptions}.
   * @throws RangeError when the parallax factor is not a finite number.
   */
  constructor(options: LayerOptions = {}) {
    super(options);
    const parallax = options.parallax ?? 1;
    if (!Number.isFinite(parallax)) {
      throw new RangeError(`parallax must be a finite number, got ${parallax}`);
    }
    this.parallax = parallax;
  }
}
