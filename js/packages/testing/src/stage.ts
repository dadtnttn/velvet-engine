// The stage: a logical surface (1920 by 1080 by default) letterboxed into a window area. Scripted
// pointer positions are logical, so a script works at any window size. The placement is the same
// one that @velvet/ui computes (computeStageLayout): the largest scale that fits, centred in the area.
// This package does not import @velvet/ui, so the placement is written out here.

import { TestingError } from "./errors";

/** A box in client (CSS) pixels. */
export interface StageRect {
  readonly left: number;
  readonly top: number;
  readonly width: number;
  readonly height: number;
}

/** Where a logical surface is presented. `area` is read at each use, so a resize is seen at once. */
export interface StageMap {
  /** The logical width of the surface. */
  readonly width: number;
  /** The logical height of the surface. */
  readonly height: number;
  /** The window area, in client pixels, that the surface is letterboxed into. */
  area(): StageRect;
}

/** Options for {@link createStageMap}. */
export interface StageOptions {
  /** Logical width. Default 1920. */
  readonly width?: number;
  /** Logical height. Default 1080. */
  readonly height?: number;
  /** The window area. Default: the logical size at the client origin, so logical and client pixels are equal. */
  readonly area?: () => StageRect;
}

/** The logical width of the authoring surface, which is the same for every Velvet game. */
export const DEFAULT_STAGE_WIDTH = 1920;

/** The logical height of the authoring surface. */
export const DEFAULT_STAGE_HEIGHT = 1080;

/** A point in logical or client pixels. */
export interface StagePoint {
  readonly x: number;
  readonly y: number;
}

/** The scale and the offset of a logical surface inside its area. The offset is from the area's top-left corner. */
export interface StagePlacement {
  readonly scale: number;
  readonly offsetX: number;
  readonly offsetY: number;
}

/**
 * Creates a stage map.
 * @throws TestingError with code "bad-argument" when the logical size is not a positive number.
 */
export function createStageMap(options: StageOptions = {}): StageMap {
  const width = options.width ?? DEFAULT_STAGE_WIDTH;
  const height = options.height ?? DEFAULT_STAGE_HEIGHT;
  if (!(Number.isFinite(width) && width > 0 && Number.isFinite(height) && height > 0)) {
    throw new TestingError("bad-argument", `the stage needs a positive size, got ${width} by ${height}`);
  }
  const area = options.area ?? (() => ({ left: 0, top: 0, width, height }));
  return { width, height, area };
}

/** Works out how a logical surface sits in an area. It is a pure function, so tests can call it directly. */
export function placementOf(stage: StageMap, area: StageRect = stage.area()): StagePlacement {
  const areaWidth = Math.max(0, area.width);
  const areaHeight = Math.max(0, area.height);
  const scale = Math.min(areaWidth / stage.width, areaHeight / stage.height);
  return {
    scale,
    offsetX: (areaWidth - stage.width * scale) / 2,
    offsetY: (areaHeight - stage.height * scale) / 2,
  };
}

/** The client position of a logical point. It is the inverse of {@link clientToLogical}. */
export function logicalToClient(stage: StageMap, x: number, y: number): StagePoint {
  const area = stage.area();
  const placement = placementOf(stage, area);
  return {
    x: area.left + placement.offsetX + x * placement.scale,
    y: area.top + placement.offsetY + y * placement.scale,
  };
}

/**
 * The logical position of a client point. Points outside the presented surface give values below 0
 * or above the logical size, as they do for the page's own stage.
 * @throws TestingError with code "bad-argument" when the stage has no size on screen.
 */
export function clientToLogical(stage: StageMap, x: number, y: number): StagePoint {
  const area = stage.area();
  const placement = placementOf(stage, area);
  if (!(placement.scale > 0)) {
    throw new TestingError("bad-argument", "the stage has no size on screen, so a client point has no logical position");
  }
  return {
    x: (x - area.left - placement.offsetX) / placement.scale,
    y: (y - area.top - placement.offsetY) / placement.scale,
  };
}
