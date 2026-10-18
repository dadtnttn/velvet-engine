// @vitest-environment happy-dom
// Tests of the drag controller on the real page structure: a real stage, a real board view, and a real
// board. The pointer events are dispatched as the browser sends them (see test-support.ts).

import { afterEach, beforeAll, describe, expect, it, vi } from "vitest";
import { Board, loadCatalog, type StackView } from "@velvet/cards";
import { createStage, type Stage } from "@velvet/ui";
import { BoardView, type ViewLabels } from "./board-view";
import { DragController } from "./drag";
import { CARD, TABLE_ORIGIN } from "./layout";
import { centreOf, firePointer, loadWasmForTests, readPublic, stubLayout } from "./test-support";

/** The labels of the view: the definitions name themselves. */
const labels: ViewLabels = {
  cardName: (def) => def,
  cardKind: () => "resource",
  recipeName: (recipe) => recipe,
  progressLabel: "Progress",
};

beforeAll(async () => {
  await loadWasmForTests();
});

afterEach(() => {
  vi.restoreAllMocks();
  document.body.replaceChildren();
});

/** A page with a stage, a board view and a drag controller, and the notices it raised. */
interface Table {
  readonly board: Board;
  readonly stage: Stage;
  readonly view: BoardView;
  readonly drag: DragController;
  readonly notices: string[];
  /** Draws the table, as a frame would. */
  draw(): void;
  /** The element of a card. */
  element(card: number): HTMLElement;
  /** Presses a card, moves the pointer through the points, and releases on the last one. */
  dragCard(card: number, path: ReadonlyArray<{ x: number; y: number }>): void;
  /** Ends the test's page: the listeners, the stage and the board. */
  close(): void;
}

/** Builds a table whose board is set up by `setup`. */
async function makeTable(setup: (board: Board) => void): Promise<Table> {
  const catalog = await loadCatalog(await readPublic("data/catalog.json"));
  const board = await Board.create(catalog, 11);
  setup(board);
  board.takeEvents();

  stubLayout();
  const host = document.createElement("div");
  document.body.append(host);
  const stage = createStage(host, { width: 1920, height: 1080 });
  const view = new BoardView(stage.root);
  const notices: string[] = [];
  const drag = new DragController({
    stage,
    view,
    board: () => board,
    playable: () => true,
    say: (key) => {
      notices.push(key);
    },
    fail: (error) => {
      throw error;
    },
  });
  drag.attach();

  const table: Table = {
    board,
    stage,
    view,
    drag,
    notices,
    draw() {
      view.render(board, labels, drag.dragged);
    },
    element(card) {
      const found = view.table.querySelector<HTMLElement>(`[data-card="${card}"]`);
      if (found === null) throw new Error(`card ${card} is not drawn`);
      return found;
    },
    dragCard(card, path) {
      const start = centreOf(this.element(card));
      firePointer(this.element(card), "pointerdown", start.x, start.y);
      this.draw();
      for (const point of path) {
        firePointer(window, "pointermove", point.x, point.y);
        this.draw();
      }
      const last = path[path.length - 1] ?? start;
      firePointer(window, "pointerup", last.x, last.y);
      this.draw();
    },
    close() {
      drag.detach();
      stage.destroy();
      board.free();
    },
  };
  table.draw();
  return table;
}

/** The stack that holds a card, as the board reports it. */
function stackOf(board: Board, card: number): StackView | undefined {
  return board.stacks().find((stack) => stack.cards.includes(card));
}

describe("dragging on the table", () => {
  it("drops a villager onto a forest, and the gathering job starts", async () => {
    let forest = 0;
    let villager = 0;
    const table = await makeTable((board) => {
      forest = board.spawn("forest", 0, 0);
      villager = board.spawn("villager", 0, 2);
    });
    const from = centreOf(table.element(villager));
    const onto = centreOf(table.element(forest));
    table.dragCard(villager, [from, { x: from.x + 20, y: from.y - 60 }, onto]);

    const stack = stackOf(table.board, forest);
    expect(stack?.cards).toEqual([forest, villager]);
    expect(stack?.job?.recipe).toBe("gather_wood");
    expect(table.notices).toEqual([]);
    table.close();
  });

  it("springs a refused drop back to where it came from, with a notice", async () => {
    let forest = 0;
    let wood = 0;
    const table = await makeTable((board) => {
      forest = board.spawn("forest", 0, 0);
      wood = board.spawn("wood", 4, 0);
    });
    table.dragCard(wood, [centreOf(table.element(wood)), centreOf(table.element(forest))]);

    expect(table.notices).toEqual(["toast.refused"]);
    const card = table.board.cards().find((candidate) => candidate.id === wood);
    expect(card?.x).toBe(4);
    expect(card?.y).toBe(0);
    expect(stackOf(table.board, forest)?.cards).toEqual([forest]);
    table.close();
  });

  it("keeps a stack together when a stacked card is only clicked", async () => {
    let forest = 0;
    let villager = 0;
    const table = await makeTable((board) => {
      forest = board.spawn("forest", 0, 0);
      villager = board.spawn("villager", 0, 3);
      board.dropOn(villager, forest);
    });
    const at = centreOf(table.element(villager));
    firePointer(table.element(villager), "pointerdown", at.x, at.y);
    expect(table.drag.dragged).toBe(villager);
    firePointer(window, "pointerup", at.x, at.y);

    expect(stackOf(table.board, forest)?.cards).toEqual([forest, villager]);
    expect(table.notices).toEqual([]);
    table.close();
  });

  it("places a stack that is released over empty table, inside the table", async () => {
    let wood = 0;
    const table = await makeTable((board) => {
      wood = board.spawn("wood", 0, 0);
    });
    const start = centreOf(table.element(wood));
    const target = { x: 1000, y: 600 };
    table.dragCard(wood, [start, target]);

    // The card keeps the point it was grabbed at (its centre) under the pointer.
    const card = table.board.cards().find((candidate) => candidate.id === wood);
    const expectedLeft = target.x - TABLE_ORIGIN.left - CARD.width / 2;
    const expectedTop = target.y - TABLE_ORIGIN.top - CARD.height / 2;
    expect(card?.x ?? Number.NaN).toBeCloseTo(expectedLeft / 160, 3);
    expect(card?.y ?? Number.NaN).toBeCloseTo(expectedTop / 160, 3);
    table.close();
  });
});
