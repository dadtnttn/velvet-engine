// The game: the level, the menus, the sound, the achievements and the settings, wired together.
//
// The loop runs in fixed steps of a sixtieth of a second. Each step reads the keys, advances the game clock (so the
// level stops while a menu is open), moves the run by the step and plays what the run reports, and advances the
// view. The menus are the stack's screens; the level is the screen under them.

import {
  ActionMap,
  EventEmitter,
  GameClock,
  GameLoop,
  captureError,
  startPadMenuKeys,
  type Binding,
  type GameLoopOptions,
  type Logger,
  type SettingsBackend,
} from "@velvet/core";
import { createAudio, parseAudioManifest, type AudioEngine, type AudioManifest } from "@velvet/audio";
import type { Achievements } from "@velvet/achievements";
import { Ticker } from "@velvet/tween";
import {
  createNotices,
  createScreenStack,
  createStage,
  type Notices,
  type Screen,
  type ScreenStack,
  type Stage,
} from "@velvet/ui";
import {
  ACHIEVEMENTS,
  ACHIEVEMENTS_URL,
  AUDIO_MANIFEST_URL,
  COUNTERS,
  LEVEL_URL,
  MAPS_FOLDER,
  NOTICE,
  SOUNDS,
  SPEEDRUN_SECONDS,
  SURFACE,
  type Language,
} from "./config";
import { loadArt, loadImageElement, type Art, type ImageLoader } from "./art";
import { createGameAchievements } from "./achievements";
import { createGameSettings, currentSettings, effectsLevel, type GameSettingsStore } from "./settings";
import { asLanguage, createTexts } from "./texts";
import { fingerprint, sortedJson } from "./hash";
import { createGameLogger } from "./log";
import { loadLevel, type Level } from "./level";
import { Run, type WorldEvent } from "./world";
import { GameView } from "./view";
import {
  buildPause,
  buildResult,
  buildSettings,
  buildTitle,
  type ResultView,
  type ScreenKit,
} from "./screens";
import type { I18n } from "@velvet/i18n";
import type { Intent } from "./physics";
import type { Devtools } from "@velvet/devtools";
import type { DevtoolsTarget } from "./devtools";

/** The keys of the game: the controls of the hero, the jump, and the pause. */
type Action = "left" | "right" | "jump" | "pause";

/** The bindings. The keyboard and a pad both move and jump; a pad's Start, or the P key, pauses. Escape pauses too,
 * through the stack's back action. */
const BINDINGS: readonly Binding<Action>[] = [
  { action: "left", control: { type: "key", key: "ArrowLeft" } },
  { action: "left", control: { type: "key", key: "KeyA" } },
  { action: "left", control: { type: "pad-button", button: "dpadLeft" } },
  { action: "left", control: { type: "pad-stick", stick: "left", direction: "left" } },
  { action: "right", control: { type: "key", key: "ArrowRight" } },
  { action: "right", control: { type: "key", key: "KeyD" } },
  { action: "right", control: { type: "pad-button", button: "dpadRight" } },
  { action: "right", control: { type: "pad-stick", stick: "left", direction: "right" } },
  { action: "jump", control: { type: "key", key: "Space" } },
  { action: "jump", control: { type: "key", key: "ArrowUp" } },
  { action: "jump", control: { type: "key", key: "KeyW" } },
  { action: "jump", control: { type: "pad-button", button: "a" } },
  { action: "pause", control: { type: "pad-button", button: "start" } },
  { action: "pause", control: { type: "key", key: "KeyP" } },
];

/** What the game tells the developer tools and any listener: the coins, the hits, the checkpoints and the flag. */
export interface GameEvents {
  readonly coin: { readonly index: number; readonly coins: number };
  readonly hurt: { readonly hurts: number };
  readonly checkpoint: { readonly index: number };
  readonly finish: { readonly time: number; readonly coins: number; readonly hurts: number };
}

/** Where the saved progress of the game is kept, for tests and custom hosts. */
export interface Backends {
  /** The settings' store. */
  readonly settings: SettingsBackend;
  /** The achievements' progress. */
  readonly achievements: SettingsBackend;
}

/** Options for {@link Game.start}. Every option has a default that a player's build uses. */
export interface GameOptions {
  /** The logger. Default: the game's own logger (the console and the Velvet Desktop log). */
  readonly logger?: Logger;
  /** Makes the loop from its options. Default: a new GameLoop. A test passes the driver's `createLoop`. */
  readonly createLoop?: (options: GameLoopOptions) => GameLoop;
  /** False plays without sound: no audio engine is made. Default true. */
  readonly sound?: boolean;
  /** Adds the developer tools. Default: only in development builds, where `import.meta.env.DEV` is true. */
  readonly devtools?: boolean;
  /** Where the settings and the achievements are kept. Default: the bridge, else localStorage, else memory. */
  readonly backends?: Backends;
  /** Loads a picture. Default: an image element in the browser. */
  readonly loadImage?: ImageLoader;
  /** Makes the canvas that the level is drawn on. Default: a new canvas element. */
  readonly createCanvas?: () => HTMLCanvasElement;
  /** Reads a text file by its URL. Default: fetch. */
  readonly fetchText?: (url: string) => Promise<string>;
}

/** The screens of the game, which the stack shows. */
interface Kits {
  readonly title: ScreenKit;
  readonly pause: ScreenKit;
  readonly settings: ScreenKit;
  readonly result: ScreenKit & { show(view: ResultView): void };
}

/** What `Game.start` loads before the game is built. */
interface Loaded {
  readonly stage: Stage;
  readonly logger: Logger;
  readonly settings: GameSettingsStore;
  readonly texts: I18n;
  readonly definitions: string;
  readonly level: Level;
  readonly art: Art;
  readonly audioManifest: AudioManifest | null;
  readonly canvas: HTMLCanvasElement;
  readonly fetchText: (url: string) => Promise<string>;
}

/** Reads a text file, and rejects when the server does not have it. */
async function defaultFetchText(url: string): Promise<string> {
  const response = await fetch(url);
  if (!response.ok) throw new Error(`${url} answered with status ${response.status}`);
  return response.text();
}

/** Makes the canvas that the level is drawn on. */
function defaultCanvas(): HTMLCanvasElement {
  return document.createElement("canvas");
}

/** The whole game. Create it with `Game.start`; it runs until `dispose`. */
export class Game {
  private readonly stage: Stage;
  private readonly logger: Logger;
  private readonly texts: I18n;
  private readonly settings: GameSettingsStore;
  private readonly level: Level;
  private readonly audio: AudioEngine | null;
  private readonly events = new EventEmitter<GameEvents>();
  private readonly clock: GameClock;
  private readonly motion: Ticker;
  private readonly ui: Ticker;
  private readonly loop: GameLoop;
  private readonly keys: ActionMap<Action>;
  private readonly stopPadKeys: () => void;
  private readonly achievementRuntime: Achievements;
  private readonly runValue: Run;
  private readonly stack: ScreenStack;
  private readonly view: GameView;
  private readonly notices: Notices;
  private readonly kits: Kits;
  private readonly gameScreen: Screen;
  private readonly gameElement: HTMLElement;
  private readonly stopFollowing: () => void;
  private readonly unsubscribers: Array<() => void> = [];
  private devtools: Devtools | null = null;

  /**
   * Builds the game inside the host element: it loads the texts, the settings, the level, the pictures and the sounds,
   * then starts the loop on the title screen.
   * @param host - the element that the stage fills.
   * @param options - see {@link GameOptions}.
   * @throws when a file cannot be loaded, or the level or the achievements are not valid.
   */
  static async start(host: HTMLElement, options: GameOptions = {}): Promise<Game> {
    const logger = options.logger ?? createGameLogger();
    const stage = createStage(host, { width: SURFACE.width, height: SURFACE.height });
    try {
      const loaded = await Game.load(stage, logger, options);
      const game = new Game(loaded, options);
      // The saved achievements are read before the first step, so that this run's progress merges with them and a
      // save never writes over what an earlier run unlocked.
      const progress = await game.achievementRuntime.load();
      if (progress.status === "damaged" || progress.status === "unavailable") {
        logger.warn(`the saved achievements could not be read (${progress.status}); they start again`);
      }
      // Written with the flag first, so that a production build drops the branch and the tools with it.
      if (import.meta.env.DEV && options.devtools !== false) {
        const { mountDevtools } = await import("./devtools");
        game.devtools = mountDevtools(game.devtoolsTarget(host));
      }
      game.loop.start();
      return game;
    } catch (error) {
      stage.destroy();
      throw error;
    }
  }

  /**
   * Loads everything the game reads before it is built.
   * @param stage - the stage, already made.
   * @param logger - the logger.
   * @param options - the options of the game.
   */
  private static async load(stage: Stage, logger: Logger, options: GameOptions): Promise<Loaded> {
    const fetchText = options.fetchText ?? defaultFetchText;
    const settings = createGameSettings(options.backends?.settings);
    await settings.load();
    const texts = await createTexts(logger);
    texts.setLanguage(currentSettings(settings).language);
    const definitions = await fetchText(ACHIEVEMENTS_URL);
    const level = await loadLevel(await fetchText(LEVEL_URL), async (source) => {
      return JSON.parse(await fetchText(`${MAPS_FOLDER}${source}`)) as object;
    });
    const art = await loadArt({ loadImage: options.loadImage ?? loadImageElement, fetchText });
    const audioManifest = options.sound === false ? null : parseAudioManifest(await fetchText(AUDIO_MANIFEST_URL));
    const canvas = (options.createCanvas ?? defaultCanvas)();
    return { stage, logger, settings, texts, definitions, level, art, audioManifest, canvas, fetchText };
  }

  /**
   * Builds every part of the game from what was loaded. Use `Game.start`.
   * @param loaded - what `load` read.
   * @param options - the options of the game.
   */
  private constructor(loaded: Loaded, options: GameOptions) {
    this.stage = loaded.stage;
    this.logger = loaded.logger;
    this.texts = loaded.texts;
    this.settings = loaded.settings;
    this.level = loaded.level;

    this.clock = new GameClock();
    this.clock.pause();
    this.motion = new Ticker({ clock: this.clock });
    this.ui = new Ticker();
    this.runValue = new Run(loaded.level);
    this.loop = (options.createLoop ?? ((loopOptions: GameLoopOptions) => new GameLoop(loopOptions)))({
      update: (dt: number) => this.update(dt),
      render: () => this.render(),
    });

    this.keys = new ActionMap<Action>(BINDINGS);
    this.keys.attach(window);
    this.stopPadKeys = startPadMenuKeys();

    this.audio =
      loaded.audioManifest === null
        ? null
        : createAudio({ manifest: loaded.audioManifest, clock: this.clock, loop: this.loop, settings: null });
    this.audio?.on("error", ({ id, message }) => {
      this.logger.warn(`sound ${id}: ${message}`);
    });
    this.audio?.preload().catch((error: unknown) => this.fail(error));
    this.unsubscribers.push(
      this.settings.subscribe("sfxVolume", () => this.applySettings()),
      this.settings.subscribe("language", (value) => {
        const language = asLanguage(value);
        if (language !== null && language !== this.texts.language) this.texts.setLanguage(language);
      }),
    );

    this.achievementRuntime = createGameAchievements({
      definitions: loaded.definitions,
      onUnlock: (unlock) => this.logger.info(`achievement unlocked: ${unlock.id}`),
      ...(options.backends === undefined ? {} : { backend: options.backends.achievements, storage: null, bridge: null }),
    });
    this.achievementRuntime.on("unlocked", ({ id }) => this.announceUnlock(id));

    this.gameElement = document.createElement("section");
    this.gameElement.className = "h2d-game";
    loaded.canvas.className = "h2d-canvas";
    this.gameElement.append(loaded.canvas);
    this.view = new GameView({
      canvas: loaded.canvas,
      clock: this.clock,
      motion: this.motion,
      level: loaded.level,
      art: loaded.art,
      translate: this.translate,
    });
    this.stopFollowing = this.view.attach(this.stage);

    this.stack = createScreenStack(this.stage.root);
    this.gameScreen = {
      id: "game",
      el: this.gameElement,
      onEnter: () => this.clock.resume(),
      onPause: () => this.clock.pause(),
      onResume: () => this.clock.resume(),
      onLeave: () => this.clock.pause(),
      onBack: () => {
        this.openPause();
        return false;
      },
    };
    this.kits = this.buildKits();
    this.notices = createNotices({
      host: this.stage.root,
      anchor: "bottom-right",
      t: this.translate,
      maxVisible: NOTICE.maxVisible,
      durationMs: NOTICE.durationMs,
      fadeMs: NOTICE.fadeMs,
    });
    this.unsubscribers.push(this.texts.onChange((language) => this.paintLanguage(language)));

    this.applySettings();
    this.stack.push(this.kits.title.screen);
    this.paint();
  }

  /** The screen on top of the stack, by id; null when the stack is empty. */
  get screen(): string | null {
    return this.stack.top?.id ?? null;
  }

  /** The run of the level: the hero, the coins, the checkpoint, the hits and the time. */
  get run(): Run {
    return this.runValue;
  }

  /** The achievements of the game. */
  get achievements(): Achievements {
    return this.achievementRuntime;
  }

  /** The language in use. */
  get language(): Language {
    return asLanguage(this.texts.language) ?? "es";
  }

  /**
   * Switches the language of the texts, as the settings panel does.
   * @param language - the language to switch to.
   */
  setLanguage(language: Language): void {
    this.texts.setLanguage(language);
  }

  /** Starts the level from its beginning on the game screen, as the title's Play does. */
  startNew(): void {
    this.startLevel();
  }

  /**
   * The state of the game now, as a fingerprint: the same state gives the same text. A replay compares it at each
   * checkpoint.
   */
  hash(): string {
    const hero = this.runValue.hero;
    const taken = this.level.coins.flatMap((_, index) => (this.runValue.isTaken(index) ? [index] : []));
    return fingerprint(
      sortedJson({
        screen: this.screen,
        phase: this.runValue.phase,
        hero: { x: hero.x, y: hero.y, vx: hero.vx, vy: hero.vy, grounded: hero.grounded, facing: hero.facing },
        elapsed: Math.round(this.runValue.elapsed * 1000),
        hurts: this.runValue.hurts,
        checkpoint: this.runValue.checkpoint,
        taken,
        clock: Math.round(this.clock.elapsed * 1000),
      }),
    );
  }

  /** Stops the loop, the keys, the stage and everything that listens to them. A page never needs it; the tests do. */
  dispose(): void {
    this.loop.stop();
    this.keys.detach();
    this.stopPadKeys();
    for (const unsubscribe of this.unsubscribers.splice(0)) unsubscribe();
    this.stopFollowing();
    this.devtools?.destroy();
    this.notices.destroy();
    this.stack.destroy();
    void this.audio?.close().catch((error: unknown) => this.fail(error));
    this.stage.destroy();
  }

  /**
   * The parts that the developer tools read and drive. The tools' commands call these methods of the game.
   * @param host - the element that the tools' overlay is added to.
   */
  private devtoolsTarget(host: HTMLElement): DevtoolsTarget {
    return {
      host,
      clock: this.clock,
      loop: this.loop,
      stage: this.stage,
      events: this.events,
      logger: this.logger,
      level: this.level,
      run: () => this.runValue,
      teleport: (x, y) => {
        this.runValue.teleport(x, y);
        this.view.snapCamera();
      },
      giveCoins: (count) => this.playEvents(this.runValue.giveCoins(count)),
      setGod: (on) => this.runValue.setGod(on),
    };
  }

  /** The screens of the game, built with their callbacks. */
  private buildKits(): Kits {
    const title = buildTitle(this.translate, this.ui, {
      onPlay: () => this.startLevel(),
      onSettings: () => this.stack.push(this.kits.settings.screen),
    });
    const pause = buildPause(this.translate, this.ui, {
      onContinue: () => this.stack.pop(),
      onRestart: () => {
        this.stack.popTo("game");
        this.restartLevel();
      },
      onSettings: () => this.stack.push(this.kits.settings.screen),
      onToTitle: () => this.stack.popTo("title"),
    });
    const settings = buildSettings(this.translate, this.ui, this.settings, () => this.stack.pop());
    const result = buildResult(this.translate, this.ui, {
      onAgain: () => {
        this.stack.popTo("game");
        this.restartLevel();
      },
      onToTitle: () => this.stack.popTo("title"),
    });
    return { title, pause, settings, result };
  }

  /**
   * Makes sure the game screen is on top: it pops the menus above it, or it pushes the game onto the title.
   */
  private enterGame(): void {
    if (this.stack.has("game")) {
      this.stack.popTo("game");
      return;
    }
    this.stack.popTo("title");
    this.stack.push(this.gameScreen);
  }

  /** Starts the level from the beginning: the run restarts, and the game screen comes on top. */
  private startLevel(): void {
    this.restartLevel();
    this.enterGame();
    this.play(SOUNDS.click);
  }

  /** Starts the run again from the start of the level, and sets the coins of this run to none. */
  private restartLevel(): void {
    this.runValue.restart();
    this.achievementRuntime.setCounter(COUNTERS.runCoins, 0, { allowDecrease: true });
    this.view.snapCamera();
  }

  /** Opens the pause menu over the level. */
  private openPause(): void {
    this.stack.push(this.kits.pause.screen);
  }

  /**
   * One fixed step: the keys, the clock and the motion by the same step, the run while the level is on top, and the
   * view. The menus stop the clock, so the run stops with them.
   * @param dt - the length of the step, in seconds.
   */
  private update(dt: number): void {
    if (this.keys.pressed("pause") && this.stack.top?.id === "game") this.openPause();
    this.clock.advance(dt);
    if (this.stack.top?.id === "game" && !this.clock.paused) {
      this.playEvents(this.runValue.step(this.readIntent(), dt));
    }
    this.motion.advance(dt);
    this.ui.advance(dt);
    this.view.update(dt, this.runValue);
    this.keys.endFrame();
  }

  /** Reads the player's intent for the step: the direction that is held, and the jump. */
  private readIntent(): Intent {
    const move = (this.keys.held("right") ? 1 : 0) - (this.keys.held("left") ? 1 : 0);
    return { move, jumpPressed: this.keys.pressed("jump"), jumpHeld: this.keys.held("jump") };
  }

  /** Draws the level. */
  private render(): void {
    this.view.render();
  }

  /**
   * Turns what the run reports into sound, achievements, notices, the view and the result screen.
   * @param events - the events of one step, in order.
   */
  private playEvents(events: readonly WorldEvent[]): void {
    for (const event of events) {
      switch (event.type) {
        case "jump":
          this.play(SOUNDS.jump);
          break;
        case "land":
          break;
        case "coin":
          this.play(SOUNDS.coin);
          this.view.pickup(event.index);
          this.achievementRuntime.increment(COUNTERS.coins);
          this.achievementRuntime.setCounter(COUNTERS.runCoins, event.coins);
          this.events.emit("coin", { index: event.index, coins: event.coins });
          break;
        case "checkpoint":
          this.notify(this.texts.t("notice.checkpoint"), "info");
          this.events.emit("checkpoint", { index: event.index });
          break;
        case "hurt":
          this.play(SOUNDS.hurt);
          this.view.hit();
          this.events.emit("hurt", { hurts: event.hurts });
          break;
        case "respawn":
          this.view.snapCamera();
          break;
        case "finish":
          this.finish(event);
          break;
      }
    }
  }

  /**
   * The run reached the flag: the achievements that need a clean run or a fast one are unlocked, and the result shows.
   * @param event - the finish event of the run.
   */
  private finish(event: Extract<WorldEvent, { type: "finish" }>): void {
    this.play(SOUNDS.win);
    if (event.hurts === 0) this.achievementRuntime.unlock(ACHIEVEMENTS.cleanFinish);
    if (event.time < SPEEDRUN_SECONDS) this.achievementRuntime.unlock(ACHIEVEMENTS.speedrun);
    this.events.emit("finish", { time: event.time, coins: event.coins, hurts: event.hurts });
    this.kits.result.show({ time: event.time, coins: event.coins, total: this.level.coins.length, hits: event.hurts });
    this.stack.push(this.kits.result.screen);
  }

  /**
   * Shows the notice of an achievement that unlocked, with its name in the language in use.
   * @param id - the achievement's id.
   */
  private announceUnlock(id: string): void {
    const definition = this.achievementRuntime.definitions.achievements.find((entry) => entry.id === id);
    const name = definition === undefined ? id : this.texts.t(definition.name);
    this.notices.notify({ message: this.texts.t("notice.achievement", { name }), tone: "info", key: id });
  }

  /**
   * Shows a notice.
   * @param message - the text to show.
   * @param tone - the tone of the notice.
   */
  private notify(message: string, tone: "info" | "danger"): void {
    this.notices.notify({ message, tone });
  }

  /**
   * Plays a sound by its id, if the game has sound.
   * @param id - the sound's id.
   */
  private play(id: string): void {
    this.audio?.play(id);
  }

  /** Applies the settings to the sound. */
  private applySettings(): void {
    const values = currentSettings(this.settings);
    this.audio?.setVolume("sfx", effectsLevel(values.sfxVolume));
    this.audio?.setVolume("ui", effectsLevel(values.sfxVolume));
  }

  /**
   * Writes every language-dependent part of the screen again, and keeps the settings in step with the language.
   * @param tag - the new language tag.
   */
  private paintLanguage(tag: string): void {
    const language = asLanguage(tag);
    if (language === null) return;
    if (this.settings.get("language") !== language) this.settings.set("language", language);
    this.paint();
  }

  /** Writes the words again: the screens, the notices and the level's texts. */
  private paint(): void {
    document.documentElement.lang = this.texts.language;
    this.notices.update({ t: this.translate });
    this.kits.title.refresh();
    this.kits.pause.refresh();
    this.kits.settings.refresh();
    this.kits.result.refresh();
  }

  /** The text lookup of the game, in the language in use: the ui components and the error overlay take it. */
  readonly translate = (key: string, args?: Readonly<Record<string, string | number>>): string => this.texts.t(key, args);

  /**
   * Writes an error to the log, and passes it on to the error reporter.
   * @param error - the error that was caught.
   */
  private fail(error: unknown): void {
    this.logger.error(`hello-2d: ${error instanceof Error ? error.message : String(error)}`);
    captureError(error, { scene: "level" });
  }
}

