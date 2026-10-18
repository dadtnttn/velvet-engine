// A* search over the cells of a collision grid. A cell blocks the search when it is solid, or partial. A
// one-way cell is walkable, because the search does not model jumps. Diagonal steps cost the square root of 2,
// and an optional cost callback adds a non-negative cost to each cell that a step enters (Infinity forbids it).
//
// The search is deterministic: the neighbours are visited in a fixed order, and when two open cells have the
// same estimate, the one with the smaller remaining distance and then the smaller index is expanded first. The
// same grid and the same options always give the same path.
//
// A PathSearch keeps its working arrays and grows them only when a larger grid arrives, so a search that is
// repeated allocates nothing for its own work. The path it returns is a new array of cells.

import type { CollisionGrid } from "./collision";
import { TilesError } from "./error";

/** A cell of the grid, in map cells. */
export interface PathCell {
  readonly x: number;
  readonly y: number;
}

/** Options for a search. */
export interface PathOptions {
  /** Allow the eight directions, not just the four. Default false. */
  readonly diagonal?: boolean;
  /**
   * Allow a diagonal step past the corner of a blocked cell. Default false: a diagonal step needs both of the
   * cells beside it to be walkable.
   */
  readonly cornerCutting?: boolean;
  /** The most cells the search may expand before it gives up. Default: no limit. */
  readonly maxNodes?: number;
  /**
   * An extra cost for entering a cell, from zero up. Infinity makes the cell impassable. Called once for each
   * neighbour that the search looks at, so it should be quick.
   */
  readonly cost?: (x: number, y: number) => number;
}

/** The steps of the four directions: right, down, left and up, as x and y pairs. */
const FOUR_STEPS: readonly number[] = [1, 0, 0, 1, -1, 0, 0, -1];
/** The steps of the eight directions: the four, then the four diagonals. */
const EIGHT_STEPS: readonly number[] = [...FOUR_STEPS, 1, 1, -1, 1, -1, -1, 1, -1];
/** The cost of a diagonal step. */
const DIAGONAL_COST = Math.SQRT2;

/** A reusable A* search. Make one and call `find` as often as needed. */
export class PathSearch {
  private cellCapacity = 0;
  private gScore = new Float64Array(0);
  private fScore = new Float64Array(0);
  private hScore = new Float64Array(0);
  private parent = new Int32Array(0);
  private openStamp = new Uint32Array(0);
  private closedStamp = new Uint32Array(0);
  private heap = new Int32Array(0);
  private heapPosition = new Int32Array(0);
  private heapSize = 0;
  private generation = 0;

  /**
   * Finds a path from one cell to another.
   * @param grid - the collision grid. The search stays inside it: a cell outside the grid is never entered.
   * @param from - the first cell. It may be blocked (a box can start inside a wall).
   * @param to - the goal cell. It must be walkable, unless it is the first cell.
   * @param options - see {@link PathOptions}.
   * @returns the cells from `from` to `to`, both included, or null when there is no path, when an end is outside
   * the grid or blocked, or when `maxNodes` is reached.
   * @throws TilesError with code "bad-input" for cells that are not whole numbers, a `maxNodes` below 1, or a cost
   * that is negative or not a number.
   */
  find(grid: CollisionGrid, from: PathCell, to: PathCell, options: PathOptions = {}): PathCell[] | null {
    const diagonal = options.diagonal ?? false;
    const cornerCutting = options.cornerCutting ?? false;
    const maxNodes = options.maxNodes ?? Infinity;
    if (!Number.isInteger(from.x) || !Number.isInteger(from.y) || !Number.isInteger(to.x) || !Number.isInteger(to.y)) {
      throw new TilesError("bad-input", "the cells of a path must have whole-number coordinates");
    }
    if (!(maxNodes >= 1)) throw new TilesError("bad-input", `maxNodes must be at least 1, got ${maxNodes}`);
    if (!grid.contains(from.x, from.y) || !grid.contains(to.x, to.y)) return null;
    if (from.x === to.x && from.y === to.y) return [{ x: from.x, y: from.y }];
    if (isBlocked(grid, to.x, to.y)) return null;

    const columns = grid.columns;
    this.reserve(columns * grid.rows);
    const generation = this.nextGeneration();
    const start = grid.indexOf(from.x, from.y);
    const goal = grid.indexOf(to.x, to.y);
    this.gScore[start] = 0;
    this.parent[start] = -1;
    this.hScore[start] = estimate(from.x, from.y, to.x, to.y, diagonal);
    this.fScore[start] = this.hScore[start] ?? 0;
    this.openStamp[start] = generation;
    this.push(start);

    const steps = diagonal ? EIGHT_STEPS : FOUR_STEPS;
    let expanded = 0;
    while (this.heapSize > 0) {
      const current = this.pop();
      if (current === goal) return this.pathTo(grid, goal);
      if (expanded >= maxNodes) return null;
      expanded += 1;
      this.closedStamp[current] = generation;
      const cx = grid.originX + (current % columns);
      const cy = grid.originY + Math.floor(current / columns);
      const gCurrent = this.gScore[current] ?? 0;

      for (let step = 0; step < steps.length; step += 2) {
        const dx = steps[step] ?? 0;
        const dy = steps[step + 1] ?? 0;
        const nx = cx + dx;
        const ny = cy + dy;
        if (!grid.contains(nx, ny) || isBlocked(grid, nx, ny)) continue;
        const isDiagonal = dx !== 0 && dy !== 0;
        if (isDiagonal && !cornerCutting && (isBlocked(grid, cx + dx, cy) || isBlocked(grid, cx, cy + dy))) {
          continue;
        }
        const extra = options.cost === undefined ? 0 : options.cost(nx, ny);
        if (typeof extra !== "number" || Number.isNaN(extra) || extra < 0) {
          throw new TilesError("bad-input", `the cost of the cell (${nx}, ${ny}) must be zero or more, got ${extra}`);
        }
        if (extra === Infinity) continue;

        const neighbour = grid.indexOf(nx, ny);
        if (this.closedStamp[neighbour] === generation) continue;
        const tentative = gCurrent + (isDiagonal ? DIAGONAL_COST : 1) + extra;
        if (this.openStamp[neighbour] !== generation) {
          this.openStamp[neighbour] = generation;
          this.gScore[neighbour] = tentative;
          this.parent[neighbour] = current;
          const remaining = estimate(nx, ny, to.x, to.y, diagonal);
          this.hScore[neighbour] = remaining;
          this.fScore[neighbour] = tentative + remaining;
          this.push(neighbour);
        } else if (tentative < (this.gScore[neighbour] ?? Infinity)) {
          this.gScore[neighbour] = tentative;
          this.parent[neighbour] = current;
          this.fScore[neighbour] = tentative + (this.hScore[neighbour] ?? 0);
          this.siftUp(this.heapPosition[neighbour] ?? 0);
        }
      }
    }
    return null;
  }

  /** Makes the working arrays large enough for `cells` cells. They only grow. */
  private reserve(cells: number): void {
    if (cells <= this.cellCapacity) return;
    this.cellCapacity = cells;
    this.gScore = new Float64Array(cells);
    this.fScore = new Float64Array(cells);
    this.hScore = new Float64Array(cells);
    this.parent = new Int32Array(cells);
    this.openStamp = new Uint32Array(cells);
    this.closedStamp = new Uint32Array(cells);
    this.heap = new Int32Array(cells);
    this.heapPosition = new Int32Array(cells);
    this.generation = 0;
  }

  /** Starts a new search: the stamps of the last one are left behind, not cleared. */
  private nextGeneration(): number {
    this.generation += 1;
    if (this.generation > 0xffffffff) {
      this.openStamp.fill(0);
      this.closedStamp.fill(0);
      this.generation = 1;
    }
    this.heapSize = 0;
    return this.generation;
  }

  /** Puts a cell in the open heap. */
  private push(cell: number): void {
    const position = this.heapSize;
    this.heapSize += 1;
    this.heap[position] = cell;
    this.heapPosition[cell] = position;
    this.siftUp(position);
  }

  /** Takes the best cell out of the open heap. The heap is not empty when this is called. */
  private pop(): number {
    const top = this.heap[0] ?? -1;
    this.heapSize -= 1;
    this.heapPosition[top] = -1;
    if (this.heapSize > 0) {
      const last = this.heap[this.heapSize] ?? -1;
      this.heap[0] = last;
      this.heapPosition[last] = 0;
      this.siftDown(0);
    }
    return top;
  }

  /** Moves the cell at a position up while it is better than its parent. */
  private siftUp(start: number): void {
    const cell = this.heap[start] ?? -1;
    let position = start;
    while (position > 0) {
      const parentPosition = (position - 1) >> 1;
      const parentCell = this.heap[parentPosition] ?? -1;
      if (!this.isBetter(cell, parentCell)) break;
      this.heap[position] = parentCell;
      this.heapPosition[parentCell] = position;
      position = parentPosition;
    }
    this.heap[position] = cell;
    this.heapPosition[cell] = position;
  }

  /** Moves the cell at a position down while a child is better than it. */
  private siftDown(start: number): void {
    const cell = this.heap[start] ?? -1;
    let position = start;
    for (;;) {
      let child = 2 * position + 1;
      if (child >= this.heapSize) break;
      const right = child + 1;
      if (right < this.heapSize && this.isBetter(this.heap[right] ?? -1, this.heap[child] ?? -1)) child = right;
      if (!this.isBetter(this.heap[child] ?? -1, cell)) break;
      const childCell = this.heap[child] ?? -1;
      this.heap[position] = childCell;
      this.heapPosition[childCell] = position;
      position = child;
    }
    this.heap[position] = cell;
    this.heapPosition[cell] = position;
  }

  /**
   * True when cell `a` is expanded before cell `b`: a smaller estimate first, then a smaller remaining distance,
   * then a smaller index. The order is total, so the path does not depend on the heap's internal layout.
   */
  private isBetter(a: number, b: number): boolean {
    const fa = this.fScore[a] ?? 0;
    const fb = this.fScore[b] ?? 0;
    if (fa !== fb) return fa < fb;
    const ha = this.hScore[a] ?? 0;
    const hb = this.hScore[b] ?? 0;
    if (ha !== hb) return ha < hb;
    return a < b;
  }

  /** Builds the path that ends at `goal`, by following the parents back to the start. */
  private pathTo(grid: CollisionGrid, goal: number): PathCell[] {
    let length = 1;
    let node = goal;
    while ((this.parent[node] ?? -1) !== -1) {
      node = this.parent[node] ?? -1;
      length += 1;
    }
    const path = new Array<PathCell>(length);
    node = goal;
    for (let index = length - 1; index >= 0; index--) {
      const column = node % grid.columns;
      const row = Math.floor(node / grid.columns);
      path[index] = { x: grid.originX + column, y: grid.originY + row };
      node = this.parent[node] ?? -1;
    }
    return path;
  }
}

/**
 * Finds a path with a search of its own. For repeated searches, keep a {@link PathSearch} and call its `find`.
 * @returns the path, or null. See {@link PathSearch.find}.
 */
export function findPath(
  grid: CollisionGrid,
  from: PathCell,
  to: PathCell,
  options: PathOptions = {},
): PathCell[] | null {
  return new PathSearch().find(grid, from, to, options);
}

/** True when a cell stops a search: a whole solid cell that is not one-way, or a partial cell. */
function isBlocked(grid: CollisionGrid, x: number, y: number): boolean {
  const kind = grid.kindAt(x, y);
  if (kind === "partial") return true;
  return kind === "solid" && !grid.isOneWayAt(x, y);
}

/** The estimate of the distance from a cell to the goal: exact for the moves that are allowed. */
function estimate(x: number, y: number, goalX: number, goalY: number, diagonal: boolean): number {
  const dx = Math.abs(x - goalX);
  const dy = Math.abs(y - goalY);
  if (!diagonal) return dx + dy;
  return Math.max(dx, dy) + (DIAGONAL_COST - 1) * Math.min(dx, dy);
}
