// The game: the table, the clock, the saves and the texts, wired together.
//
// The loop runs the clock and the board in fixed steps: the clock turns real seconds into game
// seconds (a speed of 2 doubles them), and the board advances its jobs by exactly those game seconds.
// Board events become effects (see effects.ts). The frame draws the table from the board's snapshots,
// so the board is the only source of truth for what the table shows.

import {
  Board,
  loadCatalog,
  type BoardEvent,
  type Catalog,
  type CardDef,
} from "@velvet/cards";
import {
  ActionMap,
  createSaveStore,
  GameClock,
  GameLoop,
  type Binding,
  type LoadedSave,
  type SaveStore,
} from "@velvet/core";
import { GuionLibrary } from "@velvet/guion";
import type { I18n, LocaleArgs } from "@velvet/i18n";
import { createStage, createToast, type Stage, type Toast, type ToastTone } from "@velvet/ui";
import { BoardView, type ViewLabels } from "./board-view";
import {
  CATALOG_URL,
  DAY_LENGTH,
  GAME_ID,
  INTRO_FILES,
  LANGUAGES,
  MANUAL_SLOT,
  QUICK_SLOT,
  SAVE_VERSION,
  SPEEDS,
  START_CARDS,
  SURFACE,
  TOAST,
  type Language,
} from "./config";
import { DragController } from "./drag";
import { Dialogue } from "./dialogue";
import { effectsOf, type Effect } from "./effects";
import { findFreeSpot, stackBox } from "./layout";
import { Hud, type HudState } from "./hud";
import { PauseMenu } from "./pause-menu";
import { buildSaveData, parseSaveData, type GameSaveData } from "./save-data";
import { asLanguage, createTexts } from "./texts";
import { isFullscreen, logLine, quitApp, readSetting, setFullscreen, writeSetting } from "./platform";
import { focusIsControl } from "./keys";

/** The keys and buttons that the game listens to, by name. */
type Action = "pause" | "speed1" | "speed2" | "speed3" | "quickSave" | "quickLoad" | "menu" | "confirm";

/** The keys, with the speed each of the first three speed actions sets. */
const SPEED_ACTIONS: readonly Action[] = ["speed1", "speed2", "speed3"];

/** The bindings of the actions. Enter and Space both confirm a line of dialogue. */
const BINDINGS: readonly Binding<Action>[] = [
  { action: "pause", control: { type: "key", key: "Space" } },
  { action: "speed1", control: { type: "key", key: "Digit1" } },
  { action: "speed1", control: { type: "key", key: "Numpad1" } },
  { action: "speed2", control: { type: "key", key: "Digit2" } },
  { action: "speed2", control: { type: "key", key: "Numpad2" } },
  { action: "speed3", control: { type: "key", key: "Digit3" } },
  { action: "speed3", control: { type: "key", key: "Numpad3" } },
  { action: "quickSave", control: { type: "key", key: "F5" } },
  { action: "quickLoad", control: { type: "key", key: "F9" } },
  { action: "menu", control: { type: "key", key: "Escape" } },
  { action: "confirm", control: { type: "key", key: "Enter" } },
  { action: "confirm", control: { type: "key", key: "NumpadEnter" } },
];

/** Fetches a text file, and rejects when the server does not have it. */
async function fetchText(url: string): Promise<string> {
  const response = await fetch(url);
  if (!response.ok) throw new Error(`${url} answered with status ${response.status}`);
  return response.text();
}

/** A new board: the starting cards, and no events from placing them (they are the table's start, not news). */
async function newBoard(catalog: Catalog): Promise<Board> {
  const seed = Math.floor(Math.random() * Number.MAX_SAFE_INTEGER);
  const board = await Board.create(catalog, seed);
  for (const start of START_CARDS) board.spawn(start.def, start.x, start.y);
  board.takeEvents();
  return board;
}

/** A readable line for any thrown value. */
function describe(error: unknown): string {
  if (error instanceof Error) return `${error.name}: ${error.message}`;
  return String(error);
}

/** The whole game. Create it with `Game.start`; it runs until the page closes. */
export class Game {
  private readonly stage: Stage;
  private readonly texts: I18n;
  private readonly catalog: Catalog;
  private readonly defs = new Map<string, CardDef>();
  private readonly clock: GameClock;
  private readonly keys: ActionMap<Action>;
  private readonly saves: SaveStore;
  private readonly view: BoardView;
  private readonly hud: Hud;
  private readonly dialogue: Dialogue;
  private readonly pause: PauseMenu;
  private readonly drag: DragController;
  private readonly toast: Toast;
  private readonly loop: GameLoop;
  private board: Board;
  private fullscreenOn = false;

  /**
   * Builds the game inside the page's host element, loads the catalog, the texts and the guiones, and
   * starts the first conversation. The returned game is running.
   *
   * @param host - the element that the stage fills.
   * @throws when a data file cannot be loaded or the WebAssembly module cannot start.
   */
  static async start(host: HTMLElement): Promise<Game> {
    const stage = createStage(host, { width: SURFACE.width, height: SURFACE.height });
    const texts = await createTexts();
    const saved = asLanguage(await readSetting("language").catch(() => null));
    if (saved !== null) texts.setLanguage(saved);

    const catalog = await loadCatalog(await fetchText(CATALOG_URL));
    const library = await GuionLibrary.create({});
    for (const language of LANGUAGES) {
      const file = INTRO_FILES[language];
      const summary = library.add(file.name, await fetchText(file.url));
      for (const problem of summary.problems) {
        logLine("warn", `${problem.file}:${problem.line}: ${problem.message}`);
      }
    }

    const board = await newBoard(catalog);
    const game = new Game(stage, texts, catalog, library, board);
    game.fullscreenOn = await isFullscreen().catch(() => false);
    game.draw();
    game.loop.start();
    game.begin();
    return game;
  }

  /** Builds every part of the game. Use `Game.start`, which also loads the data it needs. */
  private constructor(stage: Stage, texts: I18n, catalog: Catalog, library: GuionLibrary, board: Board) {
    this.stage = stage;
    this.texts = texts;
    this.catalog = catalog;
    this.board = board;
    for (const def of catalog.cards) this.defs.set(def.id, def);

    const root = stage.root;
    this.view = new BoardView(root);
    this.hud = new Hud(
      root,
      texts,
      {
        onPause: () => {
          if (this.playable()) this.togglePause();
        },
        onSpeed: (speed) => {
          if (this.playable()) this.setSpeed(speed);
        },
        onMenu: () => this.toggleMenu(),
      },
      (def) => this.cardName(def),
    );
    this.dialogue = new Dialogue(
      root,
      texts,
      library,
      () => logLine("info", "hello-cards: the conversation is over"),
      (error) => this.fail(error),
    );
    this.pause = new PauseMenu(
      root,
      texts,
      {
        onSave: () => void this.save(MANUAL_SLOT),
        onLoad: () => void this.load(MANUAL_SLOT),
        onLanguage: (language) => this.setLanguage(language),
        onFullscreen: () => void this.toggleFullscreen(),
        onQuit: () => void this.quit(),
      },
      () => ({
        language: asLanguage(this.texts.language) ?? "es",
        fullscreen: this.fullscreenOn,
      }),
    );
    this.toast = createToast({ ...TOAST });
    root.append(this.toast.el);

    this.clock = new GameClock({
      speeds: SPEEDS,
      dayLength: DAY_LENGTH,
      onDay: (day) => this.say("toast.day", "info", { day }),
    });
    this.drag = new DragController({
      stage,
      view: this.view,
      board: () => this.board,
      playable: () => this.playable(),
      say: (key, tone) => this.say(key, tone),
      fail: (error) => this.fail(error),
    });
    this.drag.attach();

    this.keys = new ActionMap<Action>(BINDINGS);
    this.keys.attach(window);
    window.addEventListener("keydown", this.onKeyDown);
    this.saves = createSaveStore({ game: GAME_ID, version: SAVE_VERSION });
    this.loop = new GameLoop({
      update: (dt) => this.update(dt),
      render: () => this.draw(),
    });

    this.paint();
    texts.onChange((language) => {
      this.paint();
      void writeSetting("language", language).catch((error: unknown) => this.fail(error));
    });
  }

  /** Stops the loop, the key and pointer listeners, and the stage. A page never needs it; the tests do. */
  dispose(): void {
    this.loop.stop();
    this.keys.detach();
    window.removeEventListener("keydown", this.onKeyDown);
    this.drag.detach();
    this.stage.destroy();
  }

  /** Whether the player may act on the table: no conversation and no pause menu is open. */
  private playable(): boolean {
    return !this.dialogue.isOpen && !this.pause.isOpen;
  }

  /** Logs the board's state, and starts the first conversation. */
  private begin(): void {
    const language = asLanguage(this.texts.language) ?? "es";
    try {
      this.dialogue.start(INTRO_FILES[language].scene);
    } catch (error) {
      this.fail(error);
    }
    logLine(
      "info",
      `board ready: ${this.board.cards().length} cards, ${this.catalog.recipes.length} recipes, ` +
        `language ${this.texts.language}, saves through ${this.saves.backendKind}`,
    );
  }

  /** One fixed step: the keys, then the clock and the board by the same game time, then the events. */
  private update(dt: number): void {
    this.handleKeys();
    let gameSeconds = 0;
    if (!this.clock.paused && this.playable()) {
      const before = this.clock.elapsed;
      this.clock.advance(dt);
      gameSeconds = this.clock.elapsed - before;
    }
    this.applyEvents(this.board.tick(gameSeconds));
    this.keys.endFrame();
  }

  /** Reads the keys of this step. Space and Enter belong to a focused button or menu item when one has the focus. */
  private handleKeys(): void {
    const keys = this.keys;
    if (keys.pressed("menu")) this.toggleMenu();
    if (keys.pressed("quickSave")) void this.save(QUICK_SLOT);
    if (keys.pressed("quickLoad")) void this.load(QUICK_SLOT);
    if (this.playable()) {
      SPEED_ACTIONS.forEach((action, index) => {
        const speed = SPEEDS[index];
        if (speed !== undefined && keys.pressed(action)) this.setSpeed(speed);
      });
    }
    if (focusIsControl()) return;
    if (this.dialogue.isOpen) {
      if (keys.pressed("pause") || keys.pressed("confirm")) this.dialogue.advance();
      return;
    }
    if (!this.pause.isOpen && keys.pressed("pause")) this.togglePause();
  }

  /** Stops the page's own reloads on F5 and F9, which belong to the game's saves. */
  private readonly onKeyDown = (event: KeyboardEvent): void => {
    if (event.key === "F5" || event.key === "F9") event.preventDefault();
  };

  /** Draws the table and the HUD from the board's snapshots and the clock. */
  private draw(): void {
    this.view.render(this.board, this.labels(), this.drag.dragged);
    this.hud.update(this.hudState());
  }

  /** The texts that the view draws with, read in the language in use. */
  private labels(): ViewLabels {
    return {
      cardName: (def) => this.cardName(def),
      cardKind: (def) => this.defs.get(def)?.kind ?? "",
      recipeName: (recipe) => this.texts.t(`recipe.${recipe}`),
      progressLabel: this.texts.t("job.progress"),
    };
  }

  /** The name of a card definition, in the language in use. */
  private cardName(def: string): string {
    return this.texts.t(this.defs.get(def)?.name ?? def);
  }

  /** The numbers of the HUD: the day and its time, the speed, and the cards that the top bar counts. */
  private hudState(): HudState {
    const stock = new Map<string, number>();
    let coins = 0;
    for (const card of this.board.cards()) {
      stock.set(card.def, (stock.get(card.def) ?? 0) + 1);
      if (card.def === "coin") coins += 1;
    }
    return {
      day: this.clock.day,
      timeOfDay: this.clock.timeOfDay,
      paused: this.clock.paused,
      speed: this.clock.speed,
      stock,
      coins,
    };
  }

  /** Applies the effects of a list of board events, in order. */
  private applyEvents(events: readonly BoardEvent[]): void {
    for (const event of events) {
      for (const effect of effectsOf(event)) this.apply(effect);
    }
  }

  /** Does one effect on screen, or on the board. */
  private apply(effect: Effect): void {
    switch (effect.kind) {
      case "pop":
        this.view.markNew(effect.card);
        return;
      case "fade":
        this.view.fadeOut(effect.card);
        return;
      case "settle":
        this.settle(effect.card, effect.x, effect.y);
        return;
      case "toast":
        this.toast.show(this.texts.t(effect.toast.key), effect.toast.tone);
        return;
    }
  }

  /**
   * Moves a new card to the nearest free place, when the engine put it on top of another stack. The
   * engine places copies in a row to the right of their source, which is often taken.
   */
  private settle(card: number, x: number, y: number): void {
    const stacks = this.board.stacks();
    const mine = stacks.find((stack) => stack.cards.includes(card));
    if (mine === undefined || mine.cards.length !== 1) return;
    const others = stacks
      .filter((stack) => stack !== mine)
      .map((stack) => stackBox(stack.x, stack.y, stack.cards.length));
    const spot = findFreeSpot({ x, y }, 1, others);
    if (spot !== null && (spot.x !== mine.x || spot.y !== mine.y)) {
      this.board.moveStack(card, spot.x, spot.y);
    }
  }

  /** Takes the events that actions queued, so that a save holds no event that the player has not seen. */
  private flushEvents(): void {
    this.applyEvents(this.board.takeEvents());
  }

  /** Pauses the clock, or lets it run again. */
  private togglePause(): void {
    if (this.clock.paused) this.clock.resume();
    else this.clock.pause();
  }

  /** Sets the clock's speed. */
  private setSpeed(speed: number): void {
    this.clock.setSpeed(speed);
  }

  /** Opens the pause menu, or closes it. A drag in progress keeps the menu shut. */
  private toggleMenu(): void {
    if (this.pause.isOpen) {
      this.pause.close();
      return;
    }
    if (this.playable() && this.drag.dragged === null) this.pause.open();
  }

  /** Writes the game to a slot. The table is not saved while a card is in the player's hand. */
  private async save(slot: string): Promise<void> {
    if (this.drag.dragged !== null) return;
    this.flushEvents();
    try {
      await this.saves.save(slot, buildSaveData(this.board, this.clock.snapshot()));
      this.say("toast.saved", "info");
      logLine("info", `game saved to ${slot}`);
    } catch (error) {
      this.say("toast.saveFailed", "danger");
      this.fail(error);
    }
  }

  /** Reads a slot and replaces the table and the clock with it. */
  private async load(slot: string): Promise<void> {
    if (this.drag.dragged !== null || this.dialogue.isOpen) return;
    let loaded: LoadedSave | null;
    try {
      loaded = await this.saves.load(slot);
    } catch (error) {
      this.say("toast.loadFailed", "danger");
      this.fail(error);
      return;
    }
    if (loaded === null) {
      this.say("toast.noSave", "warn");
      return;
    }
    const data = parseSaveData(loaded.data);
    if (data === null) {
      this.say("toast.loadFailed", "danger");
      this.fail(new Error(`slot ${slot} does not hold a game save`));
      return;
    }
    let next: Board;
    try {
      next = await this.restore(data);
    } catch (error) {
      this.say("toast.loadFailed", "danger");
      this.fail(error);
      return;
    }
    this.board.free();
    this.board = next;
    this.view.invalidate();
    this.say("toast.loaded", "info");
  }

  /**
   * Builds the board and the clock of a save, without touching the game yet. A card that the player
   * picked up while the load was running cancels it, because the old board is still in use.
   */
  private async restore(data: GameSaveData): Promise<Board> {
    const board = await Board.load(this.catalog, data.board);
    try {
      if (this.drag.dragged !== null) throw new Error("a card was picked up during the load");
      this.clock.restore(data.clock);
    } catch (error) {
      board.free();
      throw error;
    }
    return board;
  }

  /** Switches the language. The change reaches every text through `paint`. */
  private setLanguage(language: Language): void {
    if (this.texts.language !== language) this.texts.setLanguage(language);
  }

  /** Writes every language-dependent part of the screen again. */
  private paint(): void {
    document.documentElement.lang = this.texts.language;
    this.view.table.setAttribute("aria-label", this.texts.t("table.label"));
    this.view.invalidate();
    this.hud.refresh();
    this.dialogue.refresh();
    this.pause.refresh();
  }

  /** Turns full screen on or off, and keeps the pause menu's label in step. */
  private async toggleFullscreen(): Promise<void> {
    const next = !this.fullscreenOn;
    try {
      await setFullscreen(next);
      this.fullscreenOn = next;
    } catch (error) {
      this.fail(error);
    }
    this.pause.refresh();
  }

  /** Closes the game, or says how to when the browser cannot close the page. */
  private async quit(): Promise<void> {
    try {
      const closed = await quitApp();
      if (!closed) this.say("toast.closeTab", "info");
    } catch (error) {
      this.fail(error);
    }
  }

  /** Shows a notice, in the language in use. */
  private say(key: string, tone: ToastTone, args?: LocaleArgs): void {
    this.toast.show(this.texts.t(key, args), tone);
  }

  /** Writes an error to the log. The player sees the notices that the game chose to show. */
  private fail(error: unknown): void {
    logLine("error", `hello-cards: ${describe(error)}`);
  }
}
