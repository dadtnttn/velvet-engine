// Nodes: the parts of a scene. A node has a place (x, y), a turn, a scale, a pivot, an opacity and a
// visibility, and it holds child nodes. A parent draws before its children. Siblings draw in zIndex
// order, and then in the order they were added. The drawing nodes (sprites, nine-slices, text and
// shapes) add their own content to this base class.
//
// Each frame the scene walks the tree twice. The update walk computes every node's matrices and its
// culling box, from the top down. The draw walk and the hit walk then read them. The matrices and boxes
// belong to the node and are rewritten in place, so no object is built per node per frame.

import { Aabb, Matrix2D, type MutablePoint, type MutableRect, type Point2D, type Rect2D } from "./math";
import type { DrawPass, TextMeasure } from "./pass";

/** The options that every node takes. All of them are optional. */
export interface NodeOptions {
  /** A name, for tools and for finding a node while debugging. */
  name?: string;
  /** Where the pivot sits in the parent's space. Default 0. */
  x?: number;
  y?: number;
  /** The turn in radians, clockwise on the screen. Default 0. */
  rotation?: number;
  /** The scale along the node's own axes. Default 1. */
  scaleX?: number;
  scaleY?: number;
  /** An offset of the pivot from the anchor point, in the node's own units. Default 0. */
  pivotX?: number;
  pivotY?: number;
  /**
   * The anchor point as a fraction of the node's box: 0 is the left or top edge, 0.5 the middle and 1
   * the right or bottom edge. The pivot is the anchor point plus the pivot offset. Default 0.
   */
  anchorX?: number;
  anchorY?: number;
  /** The opacity from 0 to 1. It is multiplied by the opacity of the parents. Default 1. */
  alpha?: number;
  /** Whether the node and its children are drawn and can be hit. Default true. */
  visible?: boolean;
  /** The drawing order among siblings: a higher value draws later, on top. Default 0. */
  zIndex?: number;
  /** Whether a pointer can hit the node. Default false. */
  interactive?: boolean;
  /** Turns image smoothing off for this node and its children, for pixel art. Unset, it follows the parent. */
  pixelArt?: boolean;
  /** A rectangle in the node's own units. It replaces the node's shape for hit tests. Default none. */
  hitArea?: Rect2D | null;
}

/** Counts the nodes as they are added, so that siblings with equal zIndex keep their order. */
let insertions = 0;

/** Scratch matrices for the update and hit walks. Each is used between two calls, never across a recursive call. */
const localScratch = new Matrix2D();
const inverseScratch = new Matrix2D();
/** Scratch point for the hit walk. */
const pointScratch: MutablePoint = { x: 0, y: 0 };

/**
 * The base of every node. Make nodes with the subclasses: `Group`, `Layer`, `Sprite`, `NineSlice`,
 * `Text` and `Shape`.
 */
export abstract class Node {
  /** A name, for tools and debugging. */
  name: string | undefined;
  /** The position of the pivot in the parent's space. */
  x: number;
  /** The position of the pivot in the parent's space. */
  y: number;
  /** The turn in radians, clockwise on the screen. */
  rotation: number;
  /** The scale along the node's own x axis. */
  scaleX: number;
  /** The scale along the node's own y axis. */
  scaleY: number;
  /** An offset of the pivot from the anchor point, in the node's own units. */
  pivotX: number;
  /** An offset of the pivot from the anchor point, in the node's own units. */
  pivotY: number;
  /** The anchor point as a fraction of the node's box, from 0 to 1. */
  anchorX: number;
  /** The anchor point as a fraction of the node's box, from 0 to 1. */
  anchorY: number;
  /** The opacity from 0 to 1, before the parents' opacity is multiplied in. */
  alpha: number;
  /** Whether the node and its children are drawn and can be hit. */
  visible: boolean;
  /** Whether a pointer can hit the node. */
  interactive: boolean;
  /** Turns image smoothing off for this node and its children, or undefined to follow the parent. */
  pixelArt: boolean | undefined;
  /** A rectangle in the node's own units that replaces its shape for hit tests, or null. */
  hitArea: Rect2D | null;
  private zOrder: number;
  private parentNode: Node | null = null;
  private readonly kids: Node[] = [];
  private sortPending = false;
  private insertion = 0;
  private readonly layerMat = new Matrix2D();
  private readonly surfaceMat = new Matrix2D();
  private readonly cullBox = new Aabb();
  private readonly boxScratch: MutableRect = { x: 0, y: 0, width: 0, height: 0 };

  /** @param options - see {@link NodeOptions}. Every option is optional. */
  constructor(options: NodeOptions = {}) {
    this.name = options.name;
    this.x = options.x ?? 0;
    this.y = options.y ?? 0;
    this.rotation = options.rotation ?? 0;
    this.scaleX = options.scaleX ?? 1;
    this.scaleY = options.scaleY ?? 1;
    this.pivotX = options.pivotX ?? 0;
    this.pivotY = options.pivotY ?? 0;
    this.anchorX = options.anchorX ?? 0;
    this.anchorY = options.anchorY ?? 0;
    this.alpha = Math.min(1, Math.max(0, options.alpha ?? 1));
    this.visible = options.visible ?? true;
    this.zOrder = options.zIndex ?? 0;
    this.interactive = options.interactive ?? false;
    this.pixelArt = options.pixelArt;
    this.hitArea = options.hitArea ?? null;
  }

  /** The drawing order among siblings. Changing it re-sorts the children of the parent. */
  get zIndex(): number {
    return this.zOrder;
  }

  set zIndex(value: number) {
    if (value === this.zOrder) return;
    this.zOrder = value;
    if (this.parentNode !== null) this.parentNode.sortPending = true;
  }

  /** The parent node, or null for a node that is not in a tree. */
  get parent(): Node | null {
    return this.parentNode;
  }

  /** The child nodes, in drawing order. Read them here, and change the tree with `add` and `remove`. */
  get children(): readonly Node[] {
    this.sortChildren();
    return this.kids;
  }

  /**
   * The matrix from the node's space to the space of its layer, as the last update left it. The layer's
   * own placement is included, and the camera is not.
   */
  get layerMatrix(): Readonly<Matrix2D> {
    return this.layerMat;
  }

  /** The matrix from the node's space to the surface, in logical pixels, as the last update left it. */
  get surfaceMatrix(): Readonly<Matrix2D> {
    return this.surfaceMat;
  }

  /**
   * Adds nodes as children, after the children that are already there. A node that has another parent
   * moves here.
   * @throws RangeError when a node would become its own ancestor.
   */
  add(...nodes: Node[]): this {
    for (const node of nodes) {
      for (let ancestor: Node | null = this; ancestor !== null; ancestor = ancestor.parentNode) {
        if (ancestor === node) throw new RangeError("a node cannot be added inside itself");
      }
      node.detach();
      node.parentNode = this;
      node.insertion = ++insertions;
      this.kids.push(node);
      this.sortPending = true;
    }
    return this;
  }

  /** Removes a child. Nothing happens when the node is not a child. @returns this node. */
  remove(node: Node): this {
    const index = this.kids.indexOf(node);
    if (index >= 0) {
      this.kids.splice(index, 1);
      node.parentNode = null;
    }
    return this;
  }

  /** Removes every child. @returns this node. */
  clear(): this {
    for (const kid of this.kids) kid.parentNode = null;
    this.kids.length = 0;
    return this;
  }

  /**
   * The position of the node's pivot in its layer's space. The camera follows this point when the node
   * is its target. It reads the matrix and the box of the last update.
   */
  pivotInLayer(): Point2D {
    this.localBox(this.boxScratch);
    const pivotX = this.boxScratch.x + this.anchorX * this.boxScratch.width + this.pivotX;
    const pivotY = this.boxScratch.y + this.anchorY * this.boxScratch.height + this.pivotY;
    return this.layerMat.apply(pivotX, pivotY);
  }

  /**
   * @internal Computes the matrices and the culling box of this node and its children. The scene calls it
   * for each layer, with the layer's view, the parent's layer matrix (the identity for a layer) and the
   * text measure.
   */
  updateTree(view: Readonly<Matrix2D>, parentLayer: Readonly<Matrix2D>, measure: TextMeasure): void {
    this.prepare(measure);
    this.localBox(this.boxScratch);
    const pivotX = this.boxScratch.x + this.anchorX * this.boxScratch.width + this.pivotX;
    const pivotY = this.boxScratch.y + this.anchorY * this.boxScratch.height + this.pivotY;
    const local = localScratch.placement(
      this.x,
      this.y,
      this.rotation,
      this.scaleX,
      this.scaleY,
      pivotX,
      pivotY,
    );
    this.layerMat.product(parentLayer, local);
    this.surfaceMat.product(view, this.layerMat);
    this.sortChildren();
    this.cullBox.clear();
    if (this.contentRect(this.boxScratch)) this.addToCullBox(this.boxScratch);
    if (this.hitArea !== null) this.addToCullBox(this.hitArea);
    for (const kid of this.kids) {
      kid.updateTree(view, this.layerMat, measure);
      this.cullBox.addBox(kid.cullBox);
    }
  }

  /**
   * @internal Draws this node and its children. `parentAlpha` is the product of the opacities above it. A
   * subtree that lies off the surface is skipped, and counted as culled.
   */
  drawTree(pass: DrawPass, parentAlpha: number, inheritedPixelArt: boolean): void {
    if (!this.visible) return;
    const alpha = parentAlpha * this.alpha;
    if (!(alpha > 0) || this.cullBox.isEmpty) return;
    if (!this.cullBox.overlaps(0, 0, pass.surfaceWidth, pass.surfaceHeight)) {
      pass.culled += 1;
      return;
    }
    const pixelArt = this.pixelArt ?? inheritedPixelArt;
    const before = pass.calls;
    this.drawSelf(pass, alpha, pixelArt);
    if (pass.calls > before) pass.nodes += 1;
    this.sortChildren();
    for (const kid of this.kids) kid.drawTree(pass, alpha, pixelArt);
  }

  /**
   * @internal Adds the interactive nodes under a point of the surface to `out`, in drawing order, so that a
   * parent comes before its children. The point is in logical pixels.
   */
  collectAt(x: number, y: number, parentAlpha: number, out: Node[]): void {
    if (!this.visible) return;
    const alpha = parentAlpha * this.alpha;
    if (!(alpha > 0) || !this.cullBox.containsPoint(x, y)) return;
    if (this.interactive && this.hitsSurface(x, y)) out.push(this);
    this.sortChildren();
    for (const kid of this.kids) kid.collectAt(x, y, alpha, out);
  }

  /**
   * The box that the node's pivot and anchor are measured from, in the node's own units. A plain node has
   * an empty box at the origin. Subclasses that draw something give their box here.
   */
  protected localBox(out: MutableRect): void {
    out.x = 0;
    out.y = 0;
    out.width = 0;
    out.height = 0;
  }

  /**
   * The part of the node's own drawing that the culling box covers, in the node's own units. It writes
   * `out` and returns true, or it returns false when the node draws nothing.
   */
  protected contentRect(_out: MutableRect): boolean {
    return false;
  }

  /** Whether a point in the node's own units lies on its shape. It is used when no hit area is set. */
  protected hitGeometry(_x: number, _y: number): boolean {
    return false;
  }

  /** Draws the node's own content. The pass has the node's surface matrix already set up for the node. */
  protected drawSelf(_pass: DrawPass, _alpha: number, _pixelArt: boolean): void {}

  /** Runs before the node is placed. Text lays out here, with the measure that the scene gives. */
  protected prepare(_measure: TextMeasure): void {}

  /** Makes this node's culling box hold a rectangle in the node's own units. */
  private addToCullBox(rect: Readonly<MutableRect>): void {
    this.cullBox.addRect(this.surfaceMat, rect.x, rect.y, rect.width, rect.height);
  }

  /** Whether a point of the surface hits this node's own shape. */
  private hitsSurface(x: number, y: number): boolean {
    if (!inverseScratch.invertFrom(this.surfaceMat)) return false;
    const local = inverseScratch.apply(x, y, pointScratch);
    return this.hitsLocal(local.x, local.y);
  }

  /** Whether a point of the node's own units hits it: its hit area when one is set, else its shape. */
  private hitsLocal(x: number, y: number): boolean {
    const area = this.hitArea;
    if (area !== null) {
      return x >= area.x && x <= area.x + area.width && y >= area.y && y <= area.y + area.height;
    }
    return this.hitGeometry(x, y);
  }

  /** Takes the node out of its current parent, if it has one. */
  private detach(): void {
    if (this.parentNode !== null) this.parentNode.remove(this);
  }

  /** Puts the children in drawing order, when a child was added or changed its zIndex since the last sort. */
  private sortChildren(): void {
    if (!this.sortPending) return;
    this.sortPending = false;
    this.kids.sort(Node.compareDrawOrder);
  }

  /** Orders two siblings: by zIndex, then by the order in which they were added. */
  private static compareDrawOrder(a: Node, b: Node): number {
    return a.zOrder - b.zOrder || a.insertion - b.insertion;
  }
}

/** A node that draws nothing itself. It holds children, and it moves, turns, scales and fades them together. */
export class Group extends Node {}
