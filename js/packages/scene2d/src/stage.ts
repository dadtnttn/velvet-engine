// Following a stage: the scene draws at the pixel size that the stage presents, so text and shapes are sharp
// at the screen's resolution. The stage is a ui Stage. This package does not import @velvet/ui: it names the
// few members it needs in a structural type, and any stage that has them can be followed.

import type { Scene2D } from "./scene";

/** The pixel size of a stage's presented area. It is what a scene needs from a stage. */
export interface StageSize {
  readonly pixelWidth: number;
  readonly pixelHeight: number;
}

/**
 * The part of a ui Stage that {@link followStage} uses: its pixel size, and a listener for each change of
 * the layout. The real `Stage` has both, and so does any object with the same members.
 */
export interface StageSource {
  /** The width of the presented area in device pixels. */
  readonly pixelWidth: number;
  /** The height of the presented area in device pixels. */
  readonly pixelHeight: number;
  /** Runs `listener` after each change of the layout. @returns a function that removes the listener. */
  onResize(listener: (layout: StageSize) => void): () => void;
}

/**
 * Makes a scene draw at the pixel size of a stage, now and after each change of the layout (a window resize,
 * a new pixel ratio, or a change of the native setting).
 * @returns a function that stops following the stage. The scene keeps its last size.
 */
export function followStage(scene: Scene2D, stage: StageSource): () => void {
  scene.resize(stage.pixelWidth, stage.pixelHeight);
  return stage.onResize((layout) => {
    scene.resize(layout.pixelWidth, layout.pixelHeight);
  });
}
