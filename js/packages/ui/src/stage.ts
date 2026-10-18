// The stage: a fixed 16:9 surface that fits any window.
//
// Games write every position, size and click in logical pixels, 1920 by 1080 unless the game
// says otherwise. The stage holds a root element of exactly that size and scales it with a CSS
// transform into the largest area of the host that keeps the logical aspect ratio. The rest of
// the host is letterbox bars. The picture is never stretched.
//
//   host                  the element the game gives us; its size is the window area
//     .vu-viewport        covers the host; its background is the bars, and bar clicks stop here
//       .vu-root          logical size, scaled around its top-left corner and centred
//
// Pointer conversion reads the root's rectangle on screen, which is the rectangle the browser
// draws, so the inverse is exact at any window size, pixel ratio or scroll position.
//
// Pixels: a canvas inside the stage is sized to `pixelWidth` by `pixelHeight` and drawn with
// `k` pixels per logical pixel. With `native` on (the default), those are the screen's own
// pixels, so text stays sharp. With `native` off they are the logical size, and the compositor
// scales the canvas up. That is the fallback for weak GPUs.

/** Logical width and height when the options name none. */
const DEFAULT_WIDTH = 1920;
const DEFAULT_HEIGHT = 1080;

/** The events that a click on the bars absorbs, so that they never reach the game's listeners. */
const BAR_EVENTS = [
  "pointerdown",
  "pointerup",
  "pointermove",
  "pointercancel",
  "click",
  "dblclick",
  "auxclick",
  "contextmenu",
  "wheel",
] as const;

/** A point in logical pixels, with the origin at the top-left corner of the root. */
export interface LogicalPoint {
  readonly x: number;
  readonly y: number;
}

/** A box in logical pixels, measured from the top-left corner of the root. */
export interface LogicalRect {
  readonly left: number;
  readonly top: number;
  readonly width: number;
  readonly height: number;
}

/** How the stage is presented. A snapshot: read `stage.layout` again after a resize. */
export interface StageLayout {
  /** Logical width of the authoring surface. */
  readonly width: number;
  /** Logical height of the authoring surface. */
  readonly height: number;
  /** CSS pixels per logical pixel. */
  readonly scale: number;
  /** Where the presented area starts, in CSS pixels from the top-left corner of the viewport. */
  readonly left: number;
  readonly top: number;
  /** Size of the presented area in CSS pixels. */
  readonly cssWidth: number;
  readonly cssHeight: number;
  /** The device pixel ratio that this layout was measured with. */
  readonly devicePixelRatio: number;
  /** Backing-store pixels per logical pixel: `pixelWidth / width`. Device pixels when native. */
  readonly k: number;
  /** Size of a canvas backing store that covers the presented area. */
  readonly pixelWidth: number;
  readonly pixelHeight: number;
}

/** Options for {@link createStage}. */
export interface StageOptions {
  /** Logical width. Default 1920. */
  readonly width?: number;
  /** Logical height. Default 1080. */
  readonly height?: number;
  /**
   * Draw at the screen's own pixels (true, the default), or at the logical size for the
   * compositor to scale (false, for weak GPUs).
   */
  readonly native?: boolean;
}

/** A listener for {@link Stage.onResize}. It receives the new layout. */
export type StageListener = (layout: StageLayout) => void;

/**
 * Works out how a logical surface is presented in an area. It is a pure function, so it has no
 * DOM access and tests can call it directly.
 *
 * @param area - the window area in CSS pixels.
 * @param logical - the logical size of the surface.
 * @param devicePixelRatio - device pixels per CSS pixel.
 * @param native - whether the backing store uses the device pixels (true) or the logical size.
 */
export function computeStageLayout(
  area: { readonly width: number; readonly height: number },
  logical: { readonly width: number; readonly height: number },
  devicePixelRatio: number,
  native: boolean,
): StageLayout {
  const areaWidth = nonNegative(area.width);
  const areaHeight = nonNegative(area.height);
  // The largest scale at which the logical surface fits: this keeps the aspect ratio.
  const scale = Math.min(areaWidth / logical.width, areaHeight / logical.height);
  const cssWidth = logical.width * scale;
  const cssHeight = logical.height * scale;
  const pixelScale = native ? scale * devicePixelRatio : 1;
  const pixelWidth = Math.max(1, Math.round(logical.width * pixelScale));
  const pixelHeight = Math.max(1, Math.round(logical.height * pixelScale));
  return {
    width: logical.width,
    height: logical.height,
    scale,
    left: (areaWidth - cssWidth) / 2,
    top: (areaHeight - cssHeight) / 2,
    cssWidth,
    cssHeight,
    devicePixelRatio,
    k: pixelWidth / logical.width,
    pixelWidth,
    pixelHeight,
  };
}

/**
 * The 16:9 presentation of one logical surface inside a host element. Create it with
 * {@link createStage}, and put the game's own elements inside `root`: their positions are logical.
 */
export class Stage {
  /** The logical-size element. Its children are laid out in logical pixels. */
  readonly root: HTMLElement;

  private readonly host: HTMLElement;
  private readonly viewport: HTMLElement;
  private readonly logicalWidth: number;
  private readonly logicalHeight: number;
  /** The host's inline position before the stage changed it, or null when it did not change it. */
  private readonly previousHostPosition: string | null;
  private drawsNative: boolean;
  private current: StageLayout;
  private readonly listeners = new Set<StageListener>();
  private readonly cleanups: Array<() => void> = [];
  private destroyed = false;

  /**
   * @param host - the element that the stage fills. It needs a size of its own.
   * @param options - see {@link StageOptions}.
   * @throws RangeError when the width or the height is not a positive finite number.
   */
  constructor(host: HTMLElement, options: StageOptions = {}) {
    const width = options.width ?? DEFAULT_WIDTH;
    const height = options.height ?? DEFAULT_HEIGHT;
    if (!isPositive(width)) throw new RangeError(`width must be a positive number, got ${width}`);
    if (!isPositive(height)) throw new RangeError(`height must be a positive number, got ${height}`);
    this.logicalWidth = width;
    this.logicalHeight = height;
    this.drawsNative = options.native ?? true;
    this.host = host;

    // The viewport is positioned against the host, so the host must be a positioning context.
    const hostPosition = getComputedStyle(host).position;
    if (hostPosition === "" || hostPosition === "static") {
      this.previousHostPosition = host.style.position;
      host.style.position = "relative";
    } else {
      this.previousHostPosition = null;
    }

    this.viewport = createBox("div", "vu-viewport");
    this.root = createBox("div", "vu-root");
    setStyles(this.viewport, { position: "absolute", left: "0", top: "0", right: "0", bottom: "0", overflow: "hidden" });
    setStyles(this.root, { position: "absolute", transformOrigin: "0 0", overflow: "hidden" });
    this.viewport.append(this.root);
    host.append(this.viewport);

    this.current = this.measure();
    this.place(this.current);
    this.watch();
  }

  /** The current presentation: scale, offsets and pixel sizes. */
  get layout(): StageLayout {
    return this.current;
  }

  /** Logical width of the surface. */
  get width(): number {
    return this.logicalWidth;
  }

  /** Logical height of the surface. */
  get height(): number {
    return this.logicalHeight;
  }

  /** CSS pixels per logical pixel. */
  get scale(): number {
    return this.current.scale;
  }

  /** Width of a canvas backing store that covers the presented area. */
  get pixelWidth(): number {
    return this.current.pixelWidth;
  }

  /** Height of a canvas backing store that covers the presented area. */
  get pixelHeight(): number {
    return this.current.pixelHeight;
  }

  /** Backing-store pixels per logical pixel. Scale a canvas context by this to draw in logical units. */
  get k(): number {
    return this.current.k;
  }

  /** Whether the stage draws at the screen's own pixels. */
  get native(): boolean {
    return this.drawsNative;
  }

  /**
   * Switches between native drawing and the logical size, for a settings toggle. Listeners run
   * when the pixel sizes change.
   */
  setNative(value: boolean): void {
    this.drawsNative = value;
    this.refresh();
  }

  /**
   * Converts a client (CSS pixel) position to logical pixels. Returns null on the bars and
   * outside the window. Use it for pointer positions; `ActionMap.pointer()` gives client pixels.
   */
  toLogical(clientX: number, clientY: number): LogicalPoint | null {
    const frame = this.root.getBoundingClientRect();
    if (frame.width <= 0 || frame.height <= 0) return null;
    const x = ((clientX - frame.left) * this.logicalWidth) / frame.width;
    const y = ((clientY - frame.top) * this.logicalHeight) / frame.height;
    if (x < 0 || y < 0 || x >= this.logicalWidth || y >= this.logicalHeight) return null;
    return { x, y };
  }

  /** The client (CSS pixel) position of a logical point. The inverse of {@link Stage.toLogical}. */
  toClient(x: number, y: number): { readonly x: number; readonly y: number } {
    const frame = this.root.getBoundingClientRect();
    return {
      x: frame.left + (x * frame.width) / this.logicalWidth,
      y: frame.top + (y * frame.height) / this.logicalHeight,
    };
  }

  /** The box of an element on the screen, in logical pixels from the root's top-left corner. */
  rectOf(element: Element): LogicalRect {
    const frame = this.root.getBoundingClientRect();
    const box = element.getBoundingClientRect();
    if (frame.width <= 0 || frame.height <= 0) return { left: 0, top: 0, width: 0, height: 0 };
    const across = this.logicalWidth / frame.width;
    const down = this.logicalHeight / frame.height;
    return {
      left: (box.left - frame.left) * across,
      top: (box.top - frame.top) * down,
      width: box.width * across,
      height: box.height * down,
    };
  }

  /**
   * Adds a listener that runs after every change of the layout: a window resize, a new device
   * pixel ratio, or a change of `native`. It is not called straight away: read `layout` first.
   * @returns a function that removes the listener.
   */
  onResize(listener: StageListener): () => void {
    this.listeners.add(listener);
    return () => {
      this.listeners.delete(listener);
    };
  }

  /**
   * Measures the window again now. The stage does this on its own when the window or the pixel
   * ratio changes; call it after changing something that the stage cannot see.
   */
  refresh(): void {
    if (this.destroyed) return;
    const next = this.measure();
    if (sameLayout(this.current, next)) return;
    this.current = next;
    this.place(next);
    for (const listener of [...this.listeners]) listener(next);
  }

  /** Stops watching the window and removes the stage's elements (and the game's inside `root`). */
  destroy(): void {
    if (this.destroyed) return;
    this.destroyed = true;
    for (const cleanup of this.cleanups.splice(0)) cleanup();
    this.listeners.clear();
    this.viewport.remove();
    if (this.previousHostPosition !== null) this.host.style.position = this.previousHostPosition;
  }

  /** Measures the viewport and works out the presentation from it. */
  private measure(): StageLayout {
    return computeStageLayout(
      this.viewport.getBoundingClientRect(),
      { width: this.logicalWidth, height: this.logicalHeight },
      currentDevicePixelRatio(),
      this.drawsNative,
    );
  }

  /** Moves and scales the root to match a layout. Every value is inline, so no stylesheet is needed. */
  private place(layout: StageLayout): void {
    setStyles(this.root, {
      left: `${layout.left}px`,
      top: `${layout.top}px`,
      width: `${layout.width}px`,
      height: `${layout.height}px`,
      transform: `scale(${layout.scale})`,
    });
  }

  /** Starts the watchers: the host's size, the window, the pixel ratio, and the bars. */
  private watch(): void {
    const onChange = (): void => {
      this.refresh();
    };
    // The host's size drives the layout. The window resize event is the fallback where
    // ResizeObserver is missing, and it also catches browser zoom.
    if (typeof ResizeObserver === "function") {
      const observer = new ResizeObserver(onChange);
      observer.observe(this.host);
      this.cleanups.push(() => observer.disconnect());
    }
    window.addEventListener("resize", onChange);
    this.cleanups.push(() => window.removeEventListener("resize", onChange));
    this.watchPixelRatio(onChange);

    // Clicks and moves on the bars stop at the viewport. Clicks on the root (or on anything in
    // it) carry on, so the game receives them.
    for (const type of BAR_EVENTS) {
      const absorb = (event: Event): void => {
        if (event.target === this.viewport) event.stopPropagation();
      };
      this.viewport.addEventListener(type, absorb);
      this.cleanups.push(() => this.viewport.removeEventListener(type, absorb));
    }
  }

  /**
   * Re-measures when the device pixel ratio changes, which happens when the window moves to
   * another screen or the browser zoom changes. A media query on the current ratio tells us.
   */
  private watchPixelRatio(onChange: () => void): void {
    if (typeof matchMedia !== "function") return;
    let query: MediaQueryList | null = null;
    const handleChange = (): void => {
      onChange();
      subscribe();
    };
    const subscribe = (): void => {
      query?.removeEventListener("change", handleChange);
      query = matchMedia(`(resolution: ${currentDevicePixelRatio()}dppx)`);
      query.addEventListener("change", handleChange);
    };
    subscribe();
    this.cleanups.push(() => query?.removeEventListener("change", handleChange));
  }
}

/**
 * Creates a stage inside `host`. The stage fills the host, keeps the logical aspect ratio, and
 * draws the game's root in the largest 16:9 area it can.
 *
 * @example
 * const stage = createStage(document.getElementById("game")!, { width: 1920, height: 1080 });
 * const canvas = document.createElement("canvas");
 * stage.root.append(canvas);
 * stage.onResize(() => { canvas.width = stage.pixelWidth; canvas.height = stage.pixelHeight; });
 */
export function createStage(host: HTMLElement, options: StageOptions = {}): Stage {
  return new Stage(host, options);
}

/** True for a finite number above zero. */
function isPositive(value: number): boolean {
  return Number.isFinite(value) && value > 0;
}

/** The value, or zero when it is not a finite positive number. */
function nonNegative(value: number): number {
  return Number.isFinite(value) && value > 0 ? value : 0;
}

/** The device pixel ratio of the window, or 1 when the host gives none that makes sense. */
function currentDevicePixelRatio(): number {
  const value: unknown = Reflect.get(globalThis, "devicePixelRatio");
  return typeof value === "number" && Number.isFinite(value) && value > 0 ? value : 1;
}

/** True when two layouts have the same numbers, so that no listener needs to run. */
function sameLayout(a: StageLayout, b: StageLayout): boolean {
  return (
    a.width === b.width &&
    a.height === b.height &&
    a.scale === b.scale &&
    a.left === b.left &&
    a.top === b.top &&
    a.cssWidth === b.cssWidth &&
    a.cssHeight === b.cssHeight &&
    a.devicePixelRatio === b.devicePixelRatio &&
    a.k === b.k &&
    a.pixelWidth === b.pixelWidth &&
    a.pixelHeight === b.pixelHeight
  );
}

/** Creates a plain element with a class name. */
function createBox(tag: "div", className: string): HTMLElement {
  const node = document.createElement(tag);
  node.className = className;
  return node;
}

/** Copies inline style values onto an element. */
function setStyles(node: HTMLElement, styles: Partial<CSSStyleDeclaration>): void {
  Object.assign(node.style, styles);
}
