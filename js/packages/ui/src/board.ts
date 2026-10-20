// The board view: a pannable, zoomable world inside the stage, for card games and maps.
//
// The stage's root is the logical surface (1920 by 1080 unless the game says otherwise). The board adds
// one layer to the root, `el`, and gives it a transform: a world point (x, y) is drawn at the logical
// point (x * zoom + pan.x, y * zoom + pan.y). One world unit is one logical pixel at zoom 1. Items are
// elements in the layer, placed at world positions with CSS transforms. Moving an item changes its
// transform only, and a pan or a zoom changes the layer's transform only, so neither causes a layout.
//
// The view is clamped to the world bounds. A world wider or taller than the surface always covers it,
// so no empty space can be panned into view past an edge. A smaller world is centred. The zoom stays
// inside its range.
//
// Input. A press on empty space (the root itself, or the layer outside every item) and a drag pans the
// view, in logical pixels, so the view follows the pointer at any zoom. The wheel over the board zooms
// around the pointer. A press on an item goes to `enableDrag`, which reports the drag with the pointer in
// world units and the item under it. The stage absorbs presses and wheel turns on its bars, so none of
// these fire there. A drag that leaves the surface keeps going: the pressed element captures the pointer,
// and the position is clamped to the nearest edge (Stage.toLogicalClamped).
//
// Visible area. A HUD panel can cover part of the surface, such as a top bar or a side panel. The board can be
// told the insets of that cover (the `visibleArea` option, or `setVisibleArea`), and then it frames the world in
// the rest: the centring, `fit`, the zoom around the centre and the clamp of the pan all use the visible area.
// `visibleWorldRect` gives the world rectangle that the visible area shows now.

import { releaseFocus } from "./components/dom";
import type { LogicalPoint, LogicalRect, Stage } from "./stage";

/** The smallest zoom when the options name none. */
const DEFAULT_MIN_ZOOM = 0.25;

/** The largest zoom when the options name none. */
const DEFAULT_MAX_ZOOM = 4;

/** How far, in logical pixels, a press must move before it counts as a drag, when the options name none. */
const DEFAULT_THRESHOLD = 6;

/** The zoom change per pixel of a wheel turn: 100 pixels zoom by about 14 per cent. */
const WHEEL_STEP = 0.0015;

/** The pixels that one wheel line stands for, when the browser reports lines. */
const WHEEL_LINE_PIXELS = 16;

/** The pixels that one wheel page stands for, when the browser reports pages. */
const WHEEL_PAGE_PIXELS = 800;

/** The `deltaMode` values of a wheel event: lines and pages. Pixels is 0. */
const DOM_DELTA_LINE = 1;
const DOM_DELTA_PAGE = 2;

/** A point in world units. */
export interface WorldPoint {
  /** Across, in world units. */
  readonly x: number;
  /** Down, in world units. */
  readonly y: number;
}

/** A rectangle in world units: its top-left corner, then its width and its height. */
export interface WorldRect {
  /** The left edge, in world units. */
  readonly x: number;
  /** The top edge, in world units. */
  readonly y: number;
  /** The width, in world units. */
  readonly w: number;
  /** The height, in world units. */
  readonly h: number;
}

/**
 * The part of the stage's surface that the board frames, given as the insets of the HUD panels that cover it, in
 * logical pixels. A side that is left out is 0.
 */
export interface BoardInsets {
  /** The space above the visible area, in logical pixels. Default 0. */
  readonly top?: number;
  /** The space on the right of the visible area, in logical pixels. Default 0. */
  readonly right?: number;
  /** The space below the visible area, in logical pixels. Default 0. */
  readonly bottom?: number;
  /** The space on the left of the visible area, in logical pixels. Default 0. */
  readonly left?: number;
}

/** Where an item sits on the board, in world units. */
export interface ItemSpec extends WorldRect {
  /** The stacking order: a higher item is drawn above, and hit first. An integer. Default 0. */
  readonly z?: number;
}

/** The changes that {@link BoardView.moveItem} makes. A field that is left out keeps its value. */
export interface ItemMove {
  /** The new left edge, in world units. */
  readonly x?: number;
  /** The new top edge, in world units. */
  readonly y?: number;
  /** The new stacking order, an integer. */
  readonly z?: number;
}

/** The view at one moment: the zoom, in logical pixels per world unit, and the pan, in logical pixels. */
export interface BoardViewState {
  /** Logical pixels per world unit. */
  readonly zoom: number;
  /** Where the world's origin sits, in logical pixels. */
  readonly pan: LogicalPoint;
}

/** A listener of {@link BoardView.onChange}. It receives the new view. */
export type BoardListener = (state: BoardViewState) => void;

/** Options of {@link createBoardView}. */
export interface BoardOptions {
  /** The world rectangle that the view keeps covered. Items may sit outside it. */
  readonly worldBounds: WorldRect;
  /** The smallest zoom. Default 0.25. */
  readonly minZoom?: number;
  /** The largest zoom. Default 4. */
  readonly maxZoom?: number;
  /**
   * The zoom at the start, which must lie in the zoom range. Default 1, or the nearest zoom in range when 1
   * is out of it.
   */
  readonly initialZoom?: number;
  /**
   * The part of the surface that the view frames, as insets from its edges, for HUD panels that cover it. The
   * world is centred and fitted in what is left, and the zoom goes around its centre. Default: the whole surface.
   * @throws RangeError when a side is negative, or the sides leave no room.
   */
  readonly visibleArea?: BoardInsets;
  /**
   * Draws the layer at whole pixels of the screen: the pan is rounded on the device's pixel grid when it is written
   * into the transform (the view keeps the exact pan). A layer moved by part of a pixel is drawn again by the browser
   * at every move, to keep it sharp, which makes panning a big world of items heavy. Default false.
   */
  readonly snapToPixels?: boolean;
}

/** A drag of an item, as the callbacks of {@link BoardView.enableDrag} see it. */
export interface BoardDrag {
  /** The item that is dragged. */
  readonly el: HTMLElement;
  /** The pointer now, in world units. */
  readonly world: WorldPoint;
  /** Where the press happened, in world units. */
  readonly start: WorldPoint;
  /**
   * Where the item was when the press happened, in world units. If the game moves the item with the
   * pointer, the item belongs at `origin + (world - start)`.
   */
  readonly origin: WorldPoint;
  /** The item under the pointer, other than the dragged one: the drop target. Null when there is none. */
  readonly target: HTMLElement | null;
}

/** The callbacks of {@link BoardView.enableDrag}. Each one is optional. */
export interface BoardDragCallbacks {
  /** The pointer moved past the threshold: the drag starts. */
  readonly onStart?: (drag: BoardDrag) => void;
  /** The pointer moved on during the drag. */
  readonly onMove?: (drag: BoardDrag) => void;
  /** The pointer was released during the drag. */
  readonly onDrop?: (drag: BoardDrag) => void;
  /** The drag ended without a drop: the pointer was cancelled, or the drag helper was stopped during it. */
  readonly onCancel?: (drag: BoardDrag) => void;
}

/** Options of {@link BoardView.enableDrag}. */
export interface DragOptions {
  /** How far the pointer must move, in logical pixels, before a press counts as a drag. Default 6. */
  readonly threshold?: number;
}

/**
 * Options of {@link BoardView.fit}. Each one is optional, and the default keeps the whole visible area and the
 * board's own zoom range.
 */
export interface FitOptions {
  /**
   * The space that stays free inside the visible area, on every side, in logical pixels. The rectangle is fitted in
   * what is left, and it stays centred. Default 0.
   */
  readonly padding?: number;
  /**
   * The largest zoom for this fit. It can narrow the board's zoom range, but not widen it. Default: the board's
   * largest zoom.
   */
  readonly maxZoom?: number;
  /**
   * The smallest zoom for this fit. It can narrow the board's zoom range, but not widen it. When the rectangle is too
   * large to show whole at this zoom, the view stays at it and shows the middle of the rectangle. Default: the
   * board's smallest zoom.
   */
  readonly minZoom?: number;
}

/**
 * A pannable, zoomable world inside a stage. Create it with {@link createBoardView}. Every position
 * here is in world units unless the name says logical: logical pixels are the stage's own surface.
 */
export interface BoardView {
  /** The world layer, inside the stage's root. Items are added to it with addItem. */
  readonly el: HTMLElement;
  /** The smallest zoom allowed. */
  readonly minZoom: number;
  /** The largest zoom allowed. */
  readonly maxZoom: number;
  /**
   * The zoom, in logical pixels per world unit. Setting it zooms around the centre of the visible area. The
   * value is clamped to the zoom range, and the view to the world bounds.
   * @throws RangeError when the value is not a finite number.
   */
  zoom: number;
  /**
   * Where the world's origin sits, in logical pixels. Setting it pans the view; the value is clamped to
   * the world bounds.
   * @throws RangeError when a coordinate is not a finite number.
   */
  pan: LogicalPoint;
  /**
   * Zooms to `zoom` (clamped to the range) around a logical point. The world point under that point stays
   * under it, unless the world bounds clamp the pan.
   * @throws RangeError when the zoom or a coordinate is not a finite number.
   */
  zoomAt(zoom: number, point: LogicalPoint): void;
  /**
   * Pans by a distance in logical pixels, the same space as the stage. Use it from the keyboard, with the
   * distance of the frame's time step. The view stays clamped to the world bounds.
   * @throws RangeError when dx or dy is not a finite number.
   */
  panBy(dx: number, dy: number): void;
  /**
   * Shows a world rectangle as large as the visible area allows, centred in it. The zoom is clamped to the zoom
   * range, and the view to the world bounds. The options keep a margin inside the visible area, and narrow the zoom
   * range for this call (see {@link FitOptions}).
   * @throws RangeError when the rectangle's width or height is not a positive number, an option is not valid, the
   * zoom range of the options does not overlap the board's, or the padding leaves no room.
   */
  fit(rect: WorldRect, options?: FitOptions): void;
  /**
   * Sets the part of the surface that the view frames (see the `visibleArea` option). A side that is left out is
   * 0. The view is clamped again to the new area, and the change listeners run whenever the area changes, even
   * when the view stays where it is. Setting the same area again does nothing.
   * @throws RangeError when a side is negative or not finite, or the sides leave no room.
   */
  setVisibleArea(insets: BoardInsets): void;
  /** The world rectangle that the visible area shows at this moment. */
  visibleWorldRect(): WorldRect;
  /** Converts a world point to logical pixels. */
  worldToLogical(x: number, y: number): LogicalPoint;
  /** Converts a logical point to world units. The inverse of {@link BoardView.worldToLogical}. */
  logicalToWorld(x: number, y: number): WorldPoint;
  /**
   * Adds a listener that runs after the view changes: a pan or a zoom, by any means, or a change of the visible
   * area. It does not run for item moves, and not for a change that the clamp undoes.
   * @returns a function that removes the listener.
   */
  onChange(listener: BoardListener): () => void;
  /**
   * Adds an element as an item, at a world rectangle. The element joins the layer, and its box is set in
   * world units: it is positioned with a transform, and it is sized with its width and height.
   * @throws Error when the element is already an item of this board.
   * @throws RangeError when a coordinate is not finite, a size is negative, or z is not an integer.
   */
  addItem(el: HTMLElement, spec: ItemSpec): void;
  /**
   * Moves an item, or changes its stacking order. Only the transform changes, unless z changes too. An
   * element that is not an item is left alone.
   * @throws RangeError when a given value is not a finite number, or z is not an integer.
   */
  moveItem(el: HTMLElement, move: ItemMove): void;
  /**
   * Takes an item off the board. Its element leaves the layer but is not destroyed, so it can be added
   * again. Its drag helpers are stopped too, and a drag in progress on it ends with onCancel: enable the
   * drag again after adding it back. An element that is not an item is left alone.
   */
  removeItem(el: HTMLElement): void;
  /**
   * The topmost item at a world point, or null. Items are compared by z first, then by the order in which
   * they were added (the later one is on top). A point on an item's left or top edge is inside it; a point
   * on its right or bottom edge is not.
   */
  itemAt(x: number, y: number): HTMLElement | null;
  /**
   * Reports a press and drag of an item to the callbacks. A press that does not pass the threshold is a
   * click, and it reports nothing. The drop target of a callback is the item under the pointer, other than
   * the dragged one. Presses on an element that is not an item do nothing.
   * @returns a function that stops the helper. Stopping it during a drag ends that drag with onCancel.
   */
  enableDrag(el: HTMLElement, callbacks: BoardDragCallbacks, options?: DragOptions): () => void;
  /**
   * Removes the layer, with its items, and every listener, drag helper and pan. The board must not be used
   * after this.
   */
  destroy(): void;
}

/** One item of the board, as the board keeps it. */
interface ItemRecord {
  readonly el: HTMLElement;
  x: number;
  y: number;
  readonly w: number;
  readonly h: number;
  z: number;
  /** The order of addition: the later item is on top of an equal one. */
  readonly seq: number;
}

/** A view after the clamp: the zoom, and the pan as two numbers. */
interface ClampedView {
  readonly zoom: number;
  readonly x: number;
  readonly y: number;
}

/** A drag in progress, from the press until the release or the cancel. */
interface DragSession {
  readonly pointerId: number;
  /** The press, in logical pixels: the threshold is measured from it. */
  readonly pressed: LogicalPoint;
  /** The press, in world units. */
  readonly start: WorldPoint;
  /** The item's position at the press, in world units. */
  readonly origin: WorldPoint;
  /** The pointer's last position, in world units. */
  world: WorldPoint;
  /** False until the pointer has moved past the threshold. */
  started: boolean;
}

/** What {@link followPointer} calls. */
interface PointerFollower {
  /** A move of the followed pointer. */
  readonly move: (event: PointerEvent) => void;
  /** The release of the followed pointer. */
  readonly end: (event: PointerEvent) => void;
  /** The cancel of the followed pointer. */
  readonly cancel: () => void;
}

/** The board itself. The factory {@link createBoardView} is the way to make one. */
class Board implements BoardView {
  readonly el: HTMLElement;
  readonly minZoom: number;
  readonly maxZoom: number;

  private readonly stage: Stage;
  private readonly bounds: WorldRect;
  /** True when the layer is written at whole pixels of the screen (the `snapToPixels` option). */
  private readonly snapToPixels: boolean;
  private readonly items = new Map<Element, ItemRecord>();
  private readonly listeners = new Set<BoardListener>();
  /** Stops following the stage's size. */
  private readonly unwatchStage: () => void;
  /** The drag helpers of each element, so that taking an item off the board can release them. */
  private readonly dragHelpers = new Map<Element, Set<() => void>>();
  private zoomLevel: number;
  private panX = 0;
  private panY = 0;
  /** The part of the surface that the view frames, with every side given. */
  private insets: Required<BoardInsets>;
  private sequence = 0;
  /** Stops following the pointer of a pan in progress, or null when no pan is in progress. */
  private panStop: (() => void) | null = null;
  private destroyed = false;

  /**
   * @param stage - the stage whose root holds the board.
   * @param options - see {@link BoardOptions}.
   * @throws RangeError when the world bounds or the zoom range are not valid.
   */
  constructor(stage: Stage, options: BoardOptions) {
    const { x, y, w, h } = options.worldBounds;
    requireFinite(x, "worldBounds.x");
    requireFinite(y, "worldBounds.y");
    requirePositive(w, "worldBounds.w");
    requirePositive(h, "worldBounds.h");
    const minZoom = options.minZoom ?? DEFAULT_MIN_ZOOM;
    const maxZoom = options.maxZoom ?? DEFAULT_MAX_ZOOM;
    requirePositive(minZoom, "minZoom");
    requirePositive(maxZoom, "maxZoom");
    if (minZoom > maxZoom) throw new RangeError(`minZoom (${minZoom}) is above maxZoom (${maxZoom})`);
    const initialZoom = options.initialZoom ?? clampTo(1, minZoom, maxZoom);
    requirePositive(initialZoom, "initialZoom");
    if (initialZoom < minZoom || initialZoom > maxZoom) {
      throw new RangeError(`initialZoom (${initialZoom}) is outside ${minZoom} to ${maxZoom}`);
    }
    const insets = normaliseInsets(options.visibleArea ?? {}, stage);

    this.stage = stage;
    this.bounds = { x, y, w, h };
    this.minZoom = minZoom;
    this.maxZoom = maxZoom;
    this.zoomLevel = initialZoom;
    this.insets = insets;
    this.snapToPixels = options.snapToPixels ?? false;

    this.el = document.createElement("div");
    this.el.className = "vu-board";
    setStyles(this.el, {
      position: "absolute",
      left: "0px",
      top: "0px",
      width: "0px",
      height: "0px",
      transformOrigin: "0 0",
    });
    stage.root.append(this.el);

    // The view starts with the world centred on the surface.
    const centre = this.viewCentred(x + w / 2, y + h / 2, initialZoom);
    this.panX = centre.x;
    this.panY = centre.y;
    this.writeLayer();

    stage.root.addEventListener("pointerdown", this.onRootPress);
    stage.root.addEventListener("wheel", this.onWheel, { passive: false });

    // When the surface changes size (an expanding stage in a resized window), the world point at the centre of the
    // visible area stays at its centre.
    let box = this.visibleBox();
    this.unwatchStage = stage.onResize(() => {
      const worldX = (box.left + box.width / 2 - this.panX) / this.zoomLevel;
      const worldY = (box.top + box.height / 2 - this.panY) / this.zoomLevel;
      box = this.visibleBox();
      const next = this.viewCentred(worldX, worldY, this.zoomLevel);
      this.apply(this.zoomLevel, next.x, next.y);
    });
  }

  /** The zoom, in logical pixels per world unit. */
  get zoom(): number {
    return this.zoomLevel;
  }

  /** Sets the zoom around the centre of the visible area. */
  set zoom(value: number) {
    const box = this.visibleBox();
    this.zoomAt(value, { x: box.left + box.width / 2, y: box.top + box.height / 2 });
  }

  /** Where the world's origin sits, in logical pixels. */
  get pan(): LogicalPoint {
    return { x: this.panX, y: this.panY };
  }

  /** Pans to a logical position of the world's origin, clamped. */
  set pan(value: LogicalPoint) {
    requireFinite(value.x, "pan.x");
    requireFinite(value.y, "pan.y");
    this.apply(this.zoomLevel, value.x, value.y);
  }

  zoomAt(zoom: number, point: LogicalPoint): void {
    requireFinite(zoom, "zoom");
    requireFinite(point.x, "point.x");
    requireFinite(point.y, "point.y");
    const next = clampTo(zoom, this.minZoom, this.maxZoom);
    const world = this.logicalToWorld(point.x, point.y);
    this.apply(next, point.x - world.x * next, point.y - world.y * next);
  }

  panBy(dx: number, dy: number): void {
    requireFinite(dx, "dx");
    requireFinite(dy, "dy");
    this.apply(this.zoomLevel, this.panX + dx, this.panY + dy);
  }

  fit(rect: WorldRect, options: FitOptions = {}): void {
    requireFinite(rect.x, "rect.x");
    requireFinite(rect.y, "rect.y");
    requirePositive(rect.w, "rect.w");
    requirePositive(rect.h, "rect.h");
    const padding = options.padding ?? 0;
    requireNonNegative(padding, "padding");
    const minZoom = options.minZoom ?? this.minZoom;
    const maxZoom = options.maxZoom ?? this.maxZoom;
    requirePositive(minZoom, "minZoom");
    requirePositive(maxZoom, "maxZoom");
    // The fit's own range can only narrow the board's range: the zoom never leaves the limits of the board.
    const low = Math.max(minZoom, this.minZoom);
    const high = Math.min(maxZoom, this.maxZoom);
    if (low > high) {
      throw new RangeError(
        `the zoom range ${minZoom} to ${maxZoom} does not overlap the board's range ${this.minZoom} to ${this.maxZoom}`,
      );
    }
    const box = this.visibleBox();
    const width = box.width - 2 * padding;
    const height = box.height - 2 * padding;
    if (!(width > 0 && height > 0)) throw new RangeError(`padding (${padding}) leaves no room in the visible area`);
    const wanted = Math.min(width / rect.w, height / rect.h);
    const zoom = clampTo(wanted, low, high);
    const centre = this.viewCentred(rect.x + rect.w / 2, rect.y + rect.h / 2, zoom);
    this.apply(zoom, centre.x, centre.y);
  }

  setVisibleArea(insets: BoardInsets): void {
    const next = normaliseInsets(insets, this.stage);
    if (sameInsets(next, this.insets)) return;
    this.insets = next;
    // The view is clamped to the new area, and the listeners hear of the change even when the view stays put.
    const view = this.clampView(this.zoomLevel, this.panX, this.panY);
    this.zoomLevel = view.zoom;
    this.panX = view.x;
    this.panY = view.y;
    this.writeLayer();
    this.emit();
  }

  visibleWorldRect(): WorldRect {
    const box = this.visibleBox();
    const origin = this.logicalToWorld(box.left, box.top);
    return { x: origin.x, y: origin.y, w: box.width / this.zoomLevel, h: box.height / this.zoomLevel };
  }

  worldToLogical(x: number, y: number): LogicalPoint {
    return { x: x * this.zoomLevel + this.panX, y: y * this.zoomLevel + this.panY };
  }

  logicalToWorld(x: number, y: number): WorldPoint {
    return { x: (x - this.panX) / this.zoomLevel, y: (y - this.panY) / this.zoomLevel };
  }

  onChange(listener: BoardListener): () => void {
    this.listeners.add(listener);
    return () => {
      this.listeners.delete(listener);
    };
  }

  addItem(el: HTMLElement, spec: ItemSpec): void {
    if (this.items.has(el)) throw new Error("this element is already an item of the board");
    requireFinite(spec.x, "x");
    requireFinite(spec.y, "y");
    requireNonNegative(spec.w, "w");
    requireNonNegative(spec.h, "h");
    const z = spec.z ?? 0;
    requireInteger(z, "z");

    this.sequence += 1;
    const record: ItemRecord = { el, x: spec.x, y: spec.y, w: spec.w, h: spec.h, z, seq: this.sequence };
    this.items.set(el, record);
    setStyles(el, {
      position: "absolute",
      left: "0px",
      top: "0px",
      boxSizing: "border-box",
      width: `${record.w}px`,
      height: `${record.h}px`,
      zIndex: String(z),
    });
    this.placeItem(record);
    if (el.parentElement !== this.el) this.el.append(el);
  }

  moveItem(el: HTMLElement, move: ItemMove): void {
    const record = this.items.get(el);
    if (record === undefined) return;
    const x = move.x ?? record.x;
    const y = move.y ?? record.y;
    const z = move.z ?? record.z;
    requireFinite(x, "x");
    requireFinite(y, "y");
    requireInteger(z, "z");

    const moved = x !== record.x || y !== record.y;
    record.x = x;
    record.y = y;
    if (moved) this.placeItem(record);
    if (z !== record.z) {
      record.z = z;
      el.style.zIndex = String(z);
    }
  }

  removeItem(el: HTMLElement): void {
    if (!this.items.delete(el)) return;
    this.stopDrags(el);
    releaseFocus(el);
    if (el.parentElement === this.el) el.remove();
  }

  itemAt(x: number, y: number): HTMLElement | null {
    return this.hitAt(x, y, null);
  }

  enableDrag(el: HTMLElement, callbacks: BoardDragCallbacks, options: DragOptions = {}): () => void {
    const threshold = options.threshold ?? DEFAULT_THRESHOLD;
    requireNonNegative(threshold, "threshold");
    let session: DragSession | null = null;
    let stopFollowing: (() => void) | null = null;

    /** Forgets the drag and stops following the pointer. The callbacks are not called. */
    const finish = (): void => {
      session = null;
      const stop = stopFollowing;
      stopFollowing = null;
      stop?.();
    };

    const onMove = (event: PointerEvent): void => {
      const current = session;
      if (current === null) return;
      const point = this.stage.toLogicalClamped(event.clientX, event.clientY);
      current.world = this.logicalToWorld(point.x, point.y);
      if (!current.started) {
        if (Math.hypot(point.x - current.pressed.x, point.y - current.pressed.y) < threshold) return;
        current.started = true;
        callbacks.onStart?.(this.dragReport(el, current));
        return;
      }
      callbacks.onMove?.(this.dragReport(el, current));
    };

    const onEnd = (event: PointerEvent): void => {
      const current = session;
      if (current === null) return;
      const point = this.stage.toLogicalClamped(event.clientX, event.clientY);
      current.world = this.logicalToWorld(point.x, point.y);
      finish();
      if (current.started) callbacks.onDrop?.(this.dragReport(el, current));
    };

    const onCancel = (): void => {
      const current = session;
      if (current === null) return;
      finish();
      if (current.started) callbacks.onCancel?.(this.dragReport(el, current));
    };

    const onPress = (event: PointerEvent): void => {
      if (session !== null || !event.isPrimary || event.button !== 0) return;
      const record = this.items.get(el);
      if (record === undefined) return;
      const point = this.stage.toLogicalClamped(event.clientX, event.clientY);
      const world = this.logicalToWorld(point.x, point.y);
      session = {
        pointerId: event.pointerId,
        pressed: point,
        start: world,
        origin: { x: record.x, y: record.y },
        world,
        started: false,
      };
      capturePointer(el, event.pointerId);
      stopFollowing = followPointer(event.pointerId, { move: onMove, end: onEnd, cancel: onCancel });
      event.preventDefault();
    };

    const stop = (): void => {
      el.removeEventListener("pointerdown", onPress);
      onCancel();
      const registered = this.dragHelpers.get(el);
      registered?.delete(stop);
      if (registered?.size === 0) this.dragHelpers.delete(el);
    };
    el.addEventListener("pointerdown", onPress);
    const group = this.dragHelpers.get(el) ?? new Set<() => void>();
    group.add(stop);
    this.dragHelpers.set(el, group);
    return stop;
  }

  destroy(): void {
    if (this.destroyed) return;
    this.destroyed = true;
    this.unwatchStage();
    this.endPan();
    for (const el of [...this.dragHelpers.keys()]) this.stopDrags(el);
    this.stage.root.removeEventListener("pointerdown", this.onRootPress);
    this.stage.root.removeEventListener("wheel", this.onWheel);
    releaseFocus(this.el);
    this.el.remove();
    this.items.clear();
    this.listeners.clear();
  }

  /** Sets the view, clamped to the zoom range and to the world bounds. Listeners run only on a change. */
  private apply(zoom: number, x: number, y: number): void {
    const next = this.clampView(zoom, x, y);
    if (next.zoom === this.zoomLevel && next.x === this.panX && next.y === this.panY) return;
    this.zoomLevel = next.zoom;
    this.panX = next.x;
    this.panY = next.y;
    this.writeLayer();
    this.emit();
  }

  /** The view as it would be set: the zoom clamped to its range, and the pan clamped on the visible area. */
  private clampView(zoom: number, x: number, y: number): ClampedView {
    const box = this.visibleBox();
    const nextZoom = clampTo(zoom, this.minZoom, this.maxZoom);
    return {
      zoom: nextZoom,
      x: clampAxis(x, nextZoom, this.bounds.x, this.bounds.w, box.left, box.width),
      y: clampAxis(y, nextZoom, this.bounds.y, this.bounds.h, box.top, box.height),
    };
  }

  /** Tells the listeners the current view. */
  private emit(): void {
    const state: BoardViewState = { zoom: this.zoomLevel, pan: { x: this.panX, y: this.panY } };
    for (const listener of [...this.listeners]) listener(state);
  }

  /** The visible area in logical pixels: its left and top edges, and its width and height. */
  private visibleBox(): LogicalRect {
    const { left, top, right, bottom } = this.insets;
    return { left, top, width: this.stage.width - left - right, height: this.stage.height - top - bottom };
  }

  /** The view that puts a world point at the centre of the visible area, clamped to the world bounds. */
  private viewCentred(worldX: number, worldY: number, zoom: number): LogicalPoint {
    const box = this.visibleBox();
    const x = box.left + box.width / 2 - worldX * zoom;
    const y = box.top + box.height / 2 - worldY * zoom;
    return {
      x: clampAxis(x, zoom, this.bounds.x, this.bounds.w, box.left, box.width),
      y: clampAxis(y, zoom, this.bounds.y, this.bounds.h, box.top, box.height),
    };
  }

  /** Writes the view into the layer's transform, at whole pixels of the screen with `snapToPixels`. */
  private writeLayer(): void {
    const x = this.snapToPixels ? this.toScreenPixel(this.panX) : this.panX;
    const y = this.snapToPixels ? this.toScreenPixel(this.panY) : this.panY;
    setStyles(this.el, { transform: `translate(${x}px, ${y}px) scale(${this.zoomLevel})` });
  }

  /** A logical length rounded to the device pixels that the stage is shown with. */
  private toScreenPixel(value: number): number {
    const { scale, devicePixelRatio } = this.stage.layout;
    const unit = scale * devicePixelRatio;
    return unit > 0 ? Math.round(value * unit) / unit : value;
  }

  /** Writes an item's place into its transform, in world units. */
  private placeItem(record: ItemRecord): void {
    setStyles(record.el, { transform: `translate(${record.x}px, ${record.y}px)` });
  }

  /** The item under a world point, other than `skip`: the topmost by z, then by the order of addition. */
  private hitAt(x: number, y: number, skip: Element | null): HTMLElement | null {
    let top: ItemRecord | null = null;
    for (const record of this.items.values()) {
      if (record.el === skip || !covers(record, x, y)) continue;
      if (top === null || record.z > top.z || (record.z === top.z && record.seq > top.seq)) top = record;
    }
    return top?.el ?? null;
  }

  /** What a drag callback receives at the moment it is called. */
  private dragReport(el: HTMLElement, session: DragSession): BoardDrag {
    return {
      el,
      world: session.world,
      start: session.start,
      origin: session.origin,
      target: this.hitAt(session.world.x, session.world.y, el),
    };
  }

  /** Starts a pan when a press lands on empty space. */
  private readonly onRootPress = (event: PointerEvent): void => {
    if (this.panStop !== null || !event.isPrimary || event.button !== 0) return;
    if (!this.isBackground(event.target)) return;
    const pointerId = event.pointerId;
    const from = { x: event.clientX, y: event.clientY };
    const panAtPress = { x: this.panX, y: this.panY };
    capturePointer(this.stage.root, pointerId);
    this.panStop = followPointer(pointerId, {
      move: (move) => {
        // The pan is in logical pixels, so the screen distance is divided by the stage's scale.
        const scale = this.stage.scale;
        if (!(scale > 0)) return;
        this.apply(
          this.zoomLevel,
          panAtPress.x + (move.clientX - from.x) / scale,
          panAtPress.y + (move.clientY - from.y) / scale,
        );
      },
      end: () => {
        this.endPan();
      },
      cancel: () => {
        this.endPan();
      },
    });
    event.preventDefault();
  };

  /** Zooms around the pointer when the wheel turns over the board. */
  private readonly onWheel = (event: WheelEvent): void => {
    if (!this.isOnBoard(event.target)) return;
    const anchor = this.stage.toLogical(event.clientX, event.clientY);
    if (anchor === null) return;
    event.preventDefault();
    // The clamp comes first: a very large turn overflows the exponential to Infinity, and the clamp
    // still gives the zoom limit that the turn asked for.
    const wanted = clampTo(this.zoomLevel * Math.exp(-wheelPixels(event) * WHEEL_STEP), this.minZoom, this.maxZoom);
    this.zoomAt(wanted, anchor);
  };

  /** Stops every drag helper of an element. A drag in progress on it ends with onCancel. */
  private stopDrags(el: Element): void {
    const helpers = this.dragHelpers.get(el);
    if (helpers === undefined) return;
    this.dragHelpers.delete(el);
    for (const stop of [...helpers]) stop();
  }

  /** Stops the pan in progress, if any. */
  private endPan(): void {
    const stop = this.panStop;
    this.panStop = null;
    stop?.();
  }

  /** True for the root itself, and for an element inside the layer that is not inside an item. */
  private isBackground(target: EventTarget | null): boolean {
    if (target === this.stage.root) return true;
    let node: Element | null = target instanceof Element ? target : null;
    while (node !== null && node !== this.stage.root) {
      if (node === this.el) return true;
      if (this.items.has(node)) return false;
      node = node.parentElement;
    }
    return false;
  }

  /** True for the root and for anything inside the layer. A HUD element beside the layer is not on the board. */
  private isOnBoard(target: EventTarget | null): boolean {
    return target === this.stage.root || (target instanceof Node && this.el.contains(target));
  }
}

/**
 * Creates a board view inside the stage's root: a world layer that can be panned and zoomed, with its
 * items placed in it. The board is added on top of what the root holds at that moment.
 *
 * @example
 * const board = createBoardView(stage, { worldBounds: { x: 0, y: 0, w: 4000, h: 2000 } });
 * const card = document.createElement("div");
 * board.addItem(card, { x: 400, y: 300, w: 112, h: 150, z: 1 });
 * board.enableDrag(card, {
 *   onMove: ({ el, world, start, origin }) =>
 *     board.moveItem(el, { x: origin.x + world.x - start.x, y: origin.y + world.y - start.y }),
 * });
 * // From the keyboard, with the frame's step: board.panBy(dx, dy).
 *
 * @param stage - the stage whose root holds the board.
 * @param options - the world bounds and the zoom range, see {@link BoardOptions}.
 * @throws RangeError when the world bounds or the zoom range are not valid.
 */
export function createBoardView(stage: Stage, options: BoardOptions): BoardView {
  return new Board(stage, options);
}

/** Clamps a value into [low, high]. Callers pass low <= high. */
function clampTo(value: number, low: number, high: number): number {
  return Math.min(high, Math.max(low, value));
}

/**
 * The pan along one axis that keeps the world on the visible part of the view. A world longer than the visible
 * part stays covering it: its near edge is at or before the visible part's near edge, and its far edge is at or
 * after the far edge. A shorter world is centred on the visible part, and its pan is fixed.
 * @param pan - the wanted pan, in logical pixels.
 * @param zoom - the zoom.
 * @param start - the world's near edge on this axis.
 * @param length - the world's length on this axis.
 * @param viewStart - where the visible part starts on this axis, in logical pixels.
 * @param viewLength - the length of the visible part, in logical pixels.
 */
function clampAxis(pan: number, zoom: number, start: number, length: number, viewStart: number, viewLength: number): number {
  const size = length * zoom;
  if (size <= viewLength) return viewStart + (viewLength - size) / 2 - start * zoom;
  // Written as the view's start minus the product, so that a world starting at zero, with the view at zero, gives +0 and not -0.
  return clampTo(pan, viewStart + viewLength - (start + length) * zoom, viewStart - start * zoom);
}

/** True when two sets of insets have the same four sides. */
function sameInsets(a: Required<BoardInsets>, b: Required<BoardInsets>): boolean {
  return a.top === b.top && a.right === b.right && a.bottom === b.bottom && a.left === b.left;
}

/**
 * The insets as the board keeps them: every side given, and a side that is left out as 0. Each side must be a
 * finite number, zero or above, and the sides must leave room for the view on both axes.
 * @param insets - the insets that were given.
 * @param stage - the stage whose surface they cover.
 * @throws RangeError when a side is not valid, or the sides leave no room.
 */
function normaliseInsets(insets: BoardInsets, stage: Stage): Required<BoardInsets> {
  const next: Required<BoardInsets> = {
    top: insets.top ?? 0,
    right: insets.right ?? 0,
    bottom: insets.bottom ?? 0,
    left: insets.left ?? 0,
  };
  requireNonNegative(next.top, "visibleArea.top");
  requireNonNegative(next.right, "visibleArea.right");
  requireNonNegative(next.bottom, "visibleArea.bottom");
  requireNonNegative(next.left, "visibleArea.left");
  if (next.left + next.right >= stage.width) {
    throw new RangeError(`visibleArea leaves no width: left (${next.left}) and right (${next.right}) fill the ${stage.width} wide surface`);
  }
  if (next.top + next.bottom >= stage.height) {
    throw new RangeError(`visibleArea leaves no height: top (${next.top}) and bottom (${next.bottom}) fill the ${stage.height} high surface`);
  }
  return next;
}

/** True when a world point is inside an item: its left and top edges are inside, its right and bottom are not. */
function covers(record: ItemRecord, x: number, y: number): boolean {
  return x >= record.x && x < record.x + record.w && y >= record.y && y < record.y + record.h;
}

/** The distance a wheel event turns, in pixels, whatever the unit the browser reports it in. */
function wheelPixels(event: WheelEvent): number {
  if (event.deltaMode === DOM_DELTA_LINE) return event.deltaY * WHEEL_LINE_PIXELS;
  if (event.deltaMode === DOM_DELTA_PAGE) return event.deltaY * WHEEL_PAGE_PIXELS;
  return event.deltaY;
}

/**
 * Follows one pointer on the window until it is released or cancelled, so that the gesture keeps going
 * when the pointer leaves the element that started it. Other pointers are ignored.
 * @returns the function that stops following.
 */
function followPointer(pointerId: number, follower: PointerFollower): () => void {
  const onMove = (event: PointerEvent): void => {
    if (event.pointerId === pointerId) follower.move(event);
  };
  const onEnd = (event: PointerEvent): void => {
    if (event.pointerId === pointerId) follower.end(event);
  };
  const onCancel = (event: PointerEvent): void => {
    if (event.pointerId === pointerId) follower.cancel();
  };
  window.addEventListener("pointermove", onMove);
  window.addEventListener("pointerup", onEnd);
  window.addEventListener("pointercancel", onCancel);
  return (): void => {
    window.removeEventListener("pointermove", onMove);
    window.removeEventListener("pointerup", onEnd);
    window.removeEventListener("pointercancel", onCancel);
  };
}

/**
 * Captures a pointer on an element, so that its moves and its release keep reaching the element while the
 * pointer is outside it. A browser that refuses the capture, because the pointer is not active, is left
 * alone: the window still gets the moves.
 */
function capturePointer(element: Element, pointerId: number): void {
  try {
    element.setPointerCapture(pointerId);
  } catch {
    // The pointer is not active, so there is nothing to capture.
  }
}

/** Throws a RangeError unless the value is a finite number. */
function requireFinite(value: number, name: string): void {
  if (!Number.isFinite(value)) throw new RangeError(`${name} must be a finite number, got ${value}`);
}

/** Throws a RangeError unless the value is a finite number above zero. */
function requirePositive(value: number, name: string): void {
  if (!(Number.isFinite(value) && value > 0)) {
    throw new RangeError(`${name} must be a positive finite number, got ${value}`);
  }
}

/** Throws a RangeError unless the value is a finite number, zero or above. */
function requireNonNegative(value: number, name: string): void {
  if (!(Number.isFinite(value) && value >= 0)) {
    throw new RangeError(`${name} must be a finite number, zero or above, got ${value}`);
  }
}

/** Throws a RangeError unless the value is an integer. */
function requireInteger(value: number, name: string): void {
  if (!Number.isInteger(value)) throw new RangeError(`${name} must be an integer, got ${value}`);
}

/** Copies inline style values onto an element. */
function setStyles(node: HTMLElement, styles: Partial<CSSStyleDeclaration>): void {
  Object.assign(node.style, styles);
}
