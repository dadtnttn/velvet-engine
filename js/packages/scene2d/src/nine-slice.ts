// Nine-slice: a box drawn from nine pieces of one picture. The four corners keep their size. The four
// edges stretch along their own side, and the middle stretches in both directions. A panel or a frame
// keeps its borders sharp at any size.

import { clamp, isNonNegativeFinite } from "./math";
import type { DrawPass } from "./pass";
import { ImageNode, type ImageNodeOptions, type SpriteFrame } from "./sprite";

/** The widths of the four corners, in source pixels measured from each edge of the frame. */
export interface NineSliceInsets {
  readonly left: number;
  readonly top: number;
  readonly right: number;
  readonly bottom: number;
}

/** Options for {@link NineSlice}. */
export interface NineSliceOptions extends ImageNodeOptions {
  /** The corner widths. Default: no corners, so the whole picture stretches. */
  insets?: NineSliceInsets;
}

/** The three columns and rows of a nine-slice, in order: start, middle and end. */
const SLICES = [0, 1, 2] as const;

/**
 * The start and size of each column and row, for the source and for the box. A nine-slice keeps one grid and
 * rewrites it on each draw, so that a draw allocates nothing.
 */
interface NineGrid {
  readonly sourceX: number[];
  readonly sourceY: number[];
  readonly sourceWidths: number[];
  readonly sourceHeights: number[];
  readonly boxX: number[];
  readonly boxY: number[];
  readonly boxWidths: number[];
  readonly boxHeights: number[];
}

/** A grid with every value at zero. */
function newGrid(): NineGrid {
  return {
    sourceX: [0, 0, 0],
    sourceY: [0, 0, 0],
    sourceWidths: [0, 0, 0],
    sourceHeights: [0, 0, 0],
    boxX: [0, 0, 0],
    boxY: [0, 0, 0],
    boxWidths: [0, 0, 0],
    boxHeights: [0, 0, 0],
  };
}

/**
 * A box drawn from nine pieces of a picture. The `width` and `height` of the node are the size of the box.
 * A nine-slice draws the frame's stored rectangle, so a turned frame is refused, and a trimmed one is drawn
 * from its stored pixels at the top-left of the box.
 */
export class NineSlice extends ImageNode {
  /** The corner widths, in source pixels. */
  insets: NineSliceInsets;
  private readonly grid: NineGrid = newGrid();

  /**
   * @param options - see {@link NineSliceOptions}.
   * @throws RangeError when an inset is negative or not finite.
   */
  constructor(options: NineSliceOptions = {}) {
    super(options);
    const insets = options.insets ?? { left: 0, top: 0, right: 0, bottom: 0 };
    for (const [name, value] of Object.entries(insets)) {
      if (!isNonNegativeFinite(value)) {
        throw new RangeError(`insets.${name} must be a finite number from 0, got ${value}`);
      }
    }
    this.insets = { left: insets.left, top: insets.top, right: insets.right, bottom: insets.bottom };
  }

  protected override checkFrame(frame: SpriteFrame): void {
    if (frame.rotated === true) throw new RangeError("a nine-slice cannot draw a turned frame");
  }

  protected override drawSelf(pass: DrawPass, alpha: number, pixelArt: boolean): void {
    const image = this.image;
    const p = this.placementNow();
    if (image === null || p === null || p.width <= 0 || p.height <= 0) return;
    const boxWidth = this.width ?? p.sourceWidth;
    const boxHeight = this.height ?? p.sourceHeight;

    // The corners in source pixels, kept inside the frame.
    const left = clamp(this.insets.left, 0, p.width);
    const right = clamp(this.insets.right, 0, p.width - left);
    const top = clamp(this.insets.top, 0, p.height);
    const bottom = clamp(this.insets.bottom, 0, p.height - top);

    // A corner keeps its size, unless the two corners of one side do not fit in the box. Then both
    // shrink in the same ratio.
    const fitX = left + right > boxWidth && left + right > 0 ? boxWidth / (left + right) : 1;
    const fitY = top + bottom > boxHeight && top + bottom > 0 ? boxHeight / (top + bottom) : 1;
    const cornerLeft = left * fitX;
    const cornerRight = right * fitX;
    const cornerTop = top * fitY;
    const cornerBottom = bottom * fitY;

    // Columns and rows: the start, the middle and the end. The source is on the left and the box on the right.
    const grid = this.grid;
    grid.sourceX[0] = p.x;
    grid.sourceX[1] = p.x + left;
    grid.sourceX[2] = p.x + p.width - right;
    grid.sourceY[0] = p.y;
    grid.sourceY[1] = p.y + top;
    grid.sourceY[2] = p.y + p.height - bottom;
    grid.sourceWidths[0] = left;
    grid.sourceWidths[1] = p.width - left - right;
    grid.sourceWidths[2] = right;
    grid.sourceHeights[0] = top;
    grid.sourceHeights[1] = p.height - top - bottom;
    grid.sourceHeights[2] = bottom;
    grid.boxX[0] = 0;
    grid.boxX[1] = cornerLeft;
    grid.boxX[2] = boxWidth - cornerRight;
    grid.boxY[0] = 0;
    grid.boxY[1] = cornerTop;
    grid.boxY[2] = boxHeight - cornerBottom;
    grid.boxWidths[0] = cornerLeft;
    grid.boxWidths[1] = boxWidth - cornerLeft - cornerRight;
    grid.boxWidths[2] = cornerRight;
    grid.boxHeights[0] = cornerTop;
    grid.boxHeights[1] = boxHeight - cornerTop - cornerBottom;
    grid.boxHeights[2] = cornerBottom;

    const ctx = pass.context;
    pass.place(this.surfaceMatrix);
    pass.alpha(alpha);
    pass.smooth(!pixelArt);
    for (const row of SLICES) {
      for (const column of SLICES) {
        const sw = grid.sourceWidths[column] ?? 0;
        const sh = grid.sourceHeights[row] ?? 0;
        const dw = grid.boxWidths[column] ?? 0;
        const dh = grid.boxHeights[row] ?? 0;
        if (sw <= 0 || sh <= 0 || dw <= 0 || dh <= 0) continue;
        ctx.drawImage(
          image,
          grid.sourceX[column] ?? 0,
          grid.sourceY[row] ?? 0,
          sw,
          sh,
          grid.boxX[column] ?? 0,
          grid.boxY[row] ?? 0,
          dw,
          dh,
        );
        pass.count();
      }
    }
  }
}
