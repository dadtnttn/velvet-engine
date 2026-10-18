// The game: the story, the stage, the menus, the sound and the saves, wired together.
//
// The loop runs in fixed steps. Each step reads the keys, advances the clock by the real time of the step (so
// the text stops while the game is paused, and runs faster with the clock's speed), moves the dialogue and the
// tweens, and applies what the story's boxes say about the background and the music. The screens (title, game,
// pause, settings, saves, ending) are the stack's; the game screen is the one under the others.

import {
  ActionMap,
  EventEmitter,
  GameClock,
  GameLoop,
  createSaveStore,
  startPadMenuKeys,
  type Binding,
  type GameLoopOptions,
  type SaveBackend,
  type SaveStore,
  type SettingsBackend,
} from "@velvet/core";
import { createAudio, parseAudioManifest, type AudioEngine, type AudioManifest } from "@velvet/audio";
import { GuionLibrary } from "@velvet/guion";
import type { I18n } from "@velvet/i18n";
import { Ticker, animateElement, placeElement } from "@velvet/tween";
import {
  createDialogueBox,
  createNotices,
  createSaveSlots,
  createScreenStack,
  createStage,
  type DialogueBox,
  type DialogueLine,
  type DialogueShown,
  type Notices,
  type SaveSlotDraft,
  type SaveSlotRecord,
  type SaveSlots,
  type Screen,
  type ScreenStack,
  type Stage,
} from "@velvet/ui";
import { createArt, type Art } from "./art";
import {
  ART_MANIFEST_URL,
  AUDIO_MANIFEST_URL,
  AUTO_DELAYS,
  GAME_ID,
  GLASS_VAR,
  LANGUAGES,
  NOTICE,
  SAVE_VERSION,
  SOUNDS,
  START_SCENE,
  STORY_FILES,
  SURFACE,
  TEXT_SPEEDS,
  VOLUME_STEPS,
  CAST,
  type Language,
} from "./config";
import { GameView } from "./stage-view";
import { asLanguage, createTexts, SCENE_KEYS } from "./texts";
import { describeError, logLine } from "./platform";
import { buildSave, parseSave } from "./save-data";
import { createGameSettings, currentSettings, nextStep, type GameSettingsStore } from "./settings";
import { NO_VARS, Story, type StageState, type StoryHooks } from "./story";
import {
  buildEnding,
  buildPause,
  buildSaves,
  buildSettings,
  buildTitle,
  type EndingView,
  type ScreenKit,
  type SettingId,
} from "./screens";

/** The game's own keys: the ones that advance the dialogue. Enter and Space are not counted while a control has the focus. */
type Action = "advance";

/** The shortcuts. They work while a control has the focus too: the numbers choose an option, F fast-forwards. */
type Shortcut = "choice1" | "choice2" | "choice3" | "choice4" | "fast" | "menu";

/** The shortcuts that choose the options of a choice, in order. */
const CHOICE_SHORTCUTS: readonly Shortcut[] = ["choice1", "choice2", "choice3", "choice4"];

/** The keys that go on past a line. */
const ADVANCE_BINDINGS: readonly Binding<Action>[] = [
  { action: "advance", control: { type: "key", key: "Enter" } },
  { action: "advance", control: { type: "key", key: "NumpadEnter" } },
  { action: "advance", control: { type: "key", key: "Space" } },
];

/** The shortcuts. A pad's Start button opens and closes the menu. */
const SHORTCUT_BINDINGS: readonly Binding<Shortcut>[] = [
  { action: "choice1", control: { type: "key", key: "Digit1" } },
  { action: "choice1", control: { type: "key", key: "Numpad1" } },
  { action: "choice2", control: { type: "key", key: "Digit2" } },
  { action: "choice2", control: { type: "key", key: "Numpad2" } },
  { action: "choice3", control: { type: "key", key: "Digit3" } },
  { action: "choice3", control: { type: "key", key: "Numpad3" } },
  { action: "choice4", control: { type: "key", key: "Digit4" } },
  { action: "choice4", control: { type: "key", key: "Numpad4" } },
  { action: "fast", control: { type: "key", key: "KeyF" } },
  { action: "menu", control: { type: "pad-button", button: "start" } },
];

/** What the game tells the developer tools and any listener: the scene, the choice and the ending. */
export interface GameEvents {
  readonly scene: { readonly scene: string };
  readonly choice: { readonly index: number };
  readonly ending: { readonly ending: number };
  readonly saved: { readonly slot: string };
}

/** Options for {@link Game.start}. Every option has a default that a player's build uses. */
export interface GameOptions {
  /** Makes the loop from its options. Default: a new GameLoop. A test passes the driver's `createLoop`. */
  readonly createLoop?: (options: GameLoopOptions) => GameLoop;
  /** False plays without sound: no audio engine is made. Default true. */
  readonly sound?: boolean;
  /** Adds the developer tools. Default: only in development builds, where `import.meta.env.DEV` is true. */
  readonly devtools?: boolean;
  /** Where the saves are kept. Default: the bridge, else localStorage, else memory. */
  readonly saveBackend?: SaveBackend;
  /** Where the settings are kept. Default: the bridge, else localStorage, else memory. */
  readonly settingsBackend?: SettingsBackend;
}

/** The screens of the game, which the stack shows. */
interface Kits {
  readonly title: ScreenKit;
  readonly pause: ScreenKit;
  readonly settings: ScreenKit;
  readonly saves: ScreenKit & { setMode(mode: "save" | "load"): void };
  readonly ending: ScreenKit & { show(view: EndingView): void };
}

/** What `Game.start` loads before the game is built. */
interface Loaded {
  readonly stage: Stage;
  readonly settings: GameSettingsStore;
  readonly texts: I18n;
  readonly art: Art;
  readonly library: GuionLibrary;
  readonly fileIds: Readonly<Record<Language, string>>;
  readonly audioManifest: AudioManifest | null;
}

/** Fetches a text file, and rejects when the server does not have it. */
async function fetchText(url: string): Promise<string> {
  const response = await fetch(url);
  if (!response.ok) throw new Error(`${url} answered with status ${response.status}`);
  return response.text();
}

/**
 * Writes a value as JSON with its object keys sorted, so that two equal states give the same text whatever order
 * the objects were built in.
 * @param value - the value to write.
 */
function sortedJson(value: unknown): string {
  if (Array.isArray(value)) return `[${value.map(sortedJson).join(",")}]`;
  if (typeof value === "object" && value !== null) {
    const record = value as Record<string, unknown>;
    const parts = Object.keys(record)
      .sort()
      .map((key) => `${JSON.stringify(key)}:${sortedJson(record[key])}`);
    return `{${parts.join(",")}}`;
  }
  const text: string | undefined = JSON.stringify(value);
  return text ?? "null";
}

/**
 * A 32-bit FNV-1a fingerprint of a text, as eight hex digits. Two equal texts give the same fingerprint, which is
 * what a replay compares.
 * @param text - the text to fingerprint.
 */
function fingerprint(text: string): string {
  let hash = 0x811c9dc5;
  for (let index = 0; index < text.length; index += 1) {
    hash ^= text.charCodeAt(index);
    hash = Math.imul(hash, 0x01000193) >>> 0;
  }
  return hash.toString(16).padStart(8, "0");
}

/** The whole game. Create it with `Game.start`; it runs until `dispose`. */
export class Game {
  private readonly stage: Stage;
  private readonly texts: I18n;
  private readonly settings: GameSettingsStore;
  private readonly art: Art;
  private readonly audio: AudioEngine | null;
  private readonly events = new EventEmitter<GameEvents>();
  private readonly clock: GameClock;
  private readonly motion: Ticker;
  private readonly ui: Ticker;
  private readonly loop: GameLoop;
  private readonly keys: ActionMap<Action>;
  private readonly shortcuts: ActionMap<Shortcut>;
  private readonly stopPadKeys: () => void;
  private readonly saves: SaveStore;
  private readonly story: Story;
  private readonly stack: ScreenStack;
  private readonly view: GameView;
  private readonly dialogue: DialogueBox;
  private readonly notices: Notices;
  private readonly slots: SaveSlots;
  private readonly kits: Kits;
  private readonly gameScreen: Screen;
  private readonly unsubscribeLanguage: () => void;
  private devtools: { destroy(): void } | null = null;
  private applied: StageState = { background: null, music: null };
  private seenRevision = -1;
  private shownPlace: string | null = null;
  private shownGlass = -1;
  private savingNow = false;

  /**
   * Builds the game inside the host element: it loads the texts, the art, the guiones, the settings and the sounds,
   * then starts the loop on the title screen.
   * @param host - the element that the stage fills.
   * @param options - see {@link GameOptions}.
   * @throws when a data file cannot be loaded or the WebAssembly module cannot start.
   */
  static async start(host: HTMLElement, options: GameOptions = {}): Promise<Game> {
    const stage = createStage(host, { width: SURFACE.width, height: SURFACE.height });
    try {
      const loaded = await Game.load(stage, options);
      const game = new Game(loaded, options);
      // Written with the flag first, so that a production build drops the branch and the tools with it.
      if (import.meta.env.DEV && options.devtools !== false) {
        const { mountDevtools } = await import("./devtools");
        game.devtools = mountDevtools({
          host,
          clock: game.clock,
          loop: game.loop,
          stage,
          events: game.events,
          story: game.story,
          jump: (scene) => game.jumpTo(scene),
        });
      }
      game.loop.start();
      return game;
    } catch (error) {
      stage.destroy();
      throw error;
    }
  }

  /** Loads everything the game reads before it is built. */
  private static async load(stage: Stage, options: GameOptions): Promise<Loaded> {
    const settings = createGameSettings(options.settingsBackend);
    await settings.load();
    const texts = await createTexts();
    texts.setLanguage(asLanguage(currentSettings(settings).language) ?? "es");
    const art = createArt(await fetchText(ART_MANIFEST_URL));
    const library = await GuionLibrary.create({ characters: CAST.characters });
    const fileIds: Record<Language, string> = { es: "", en: "" };
    for (const language of LANGUAGES) {
      const file = STORY_FILES[language];
      const summary = library.add(file.name, await fetchText(file.url));
      for (const problem of summary.problems) {
        logLine("warn", `${problem.file}:${problem.line}: ${problem.message}`);
      }
      fileIds[language] = summary.id;
    }
    const audioManifest =
      options.sound === false ? null : parseAudioManifest(await fetchText(AUDIO_MANIFEST_URL));
    return { stage, settings, texts, art, library, fileIds, audioManifest };
  }

  /** Builds every part of the game from what was loaded. Use `Game.start`. */
  private constructor(loaded: Loaded, options: GameOptions) {
    this.stage = loaded.stage;
    this.texts = loaded.texts;
    this.settings = loaded.settings;
    this.art = loaded.art;

    this.clock = new GameClock();
    this.clock.pause();
    this.motion = new Ticker({ clock: this.clock });
    this.ui = new Ticker();

    this.loop = (options.createLoop ?? ((loopOptions: GameLoopOptions) => new GameLoop(loopOptions)))({
      update: (dt: number) => this.update(dt),
    });

    this.keys = new ActionMap<Action>(ADVANCE_BINDINGS);
    this.keys.attach(window);
    this.shortcuts = new ActionMap<Shortcut>(SHORTCUT_BINDINGS, { captureControls: true });
    this.shortcuts.attach(window);
    this.stopPadKeys = startPadMenuKeys();

    this.audio =
      loaded.audioManifest === null
        ? null
        : createAudio({ manifest: loaded.audioManifest, clock: this.clock, loop: this.loop, settings: null });

    this.saves = createSaveStore({ game: GAME_ID, version: SAVE_VERSION, backend: options.saveBackend });

    const hooks: StoryHooks = {
      onChoice: (index) => {
        this.audio?.play(SOUNDS.click);
        this.events.emit("choice", { index });
      },
      onScene: (scene) => {
        this.events.emit("scene", { scene });
        this.autosave();
      },
    };
    this.story = new Story(loaded.library, loaded.fileIds, this.currentLanguage(), hooks);

    this.stack = createScreenStack(this.stage.root);
    this.view = new GameView(this.art, this.texts, () => this.toggleMenu());
    this.dialogue = createDialogueBox({
      speed: 40,
      autoDelaySeconds: 1.5,
      portrait: (line) => this.portraitOf(line),
      t: this.translate,
      onLetter: () => {
        this.audio?.play(SOUNDS.blip);
      },
      onEnd: (shown) => this.onStoryEnd(shown),
    });
    this.view.el.append(this.dialogue.el);
    this.gameScreen = {
      id: "game",
      el: this.view.el,
      onEnter: () => this.clock.resume(),
      onPause: () => this.clock.pause(),
      onResume: () => this.clock.resume(),
      onLeave: () => this.clock.pause(),
      onBack: () => {
        this.stack.push(this.kits.pause.screen);
        return false;
      },
    };

    this.slots = createSaveSlots({
      store: this.saves,
      mode: "load",
      t: this.translate,
      autosave: "auto",
      collect: () => this.draft(),
      onLoad: (record) => this.loadState(record),
      onSaved: (record) => {
        this.notify("notice.saved", "info");
        this.events.emit("saved", { slot: record.slot });
      },
      onError: (error) => {
        this.fail(error);
        this.notify(this.savingNow ? "notice.saveFailed" : "notice.loadFailed", "danger");
      },
    });

    this.kits = this.buildKits();
    this.notices = createNotices({
      host: this.stage.root,
      anchor: "bottom-right",
      avoid: () => (this.dialogue.isOpen ? this.stage.rectOf(this.dialogue.el) : null),
      t: this.translate,
      maxVisible: NOTICE.maxVisible,
      durationMs: NOTICE.durationMs,
      fadeMs: NOTICE.fadeMs,
    });

    this.unsubscribeLanguage = this.texts.onChange((language) => this.paintLanguage(language));
    this.applySettings();
    this.stack.push(this.kits.title.screen);
    this.paint();
  }

  /** The screens of the game, built with their callbacks. */
  private buildKits(): Kits {
    const ui = this.ui;
    const title = buildTitle(this.texts, this.art, ui, {
      onNew: () => this.startNew(),
      onLoad: () => this.openSaves("load"),
      onSettings: () => this.openSettings(),
    });
    const pause = buildPause(this.texts, ui, {
      onContinue: () => this.stack.pop(),
      onSave: () => this.openSaves("save"),
      onLoad: () => this.openSaves("load"),
      onSettings: () => this.openSettings(),
      onToTitle: () => this.toTitle(),
    });
    const settings = buildSettings(
      this.texts,
      ui,
      () => ({ settings: currentSettings(this.settings), language: this.currentLanguage() }),
      (id) => this.pickSetting(id),
    );
    const saves = buildSaves(this.texts, ui, this.slots, () => this.stack.pop());
    const ending = buildEnding(this.texts, ui, () => this.toTitle());
    return { title, pause, settings, saves, ending };
  }

  /** Stops the loop, the keys and the stage, and everything that listens to them. A page never needs it; the tests do. */
  dispose(): void {
    this.loop.stop();
    this.keys.detach();
    this.shortcuts.detach();
    this.stopPadKeys();
    this.unsubscribeLanguage();
    this.devtools?.destroy();
    this.dialogue.destroy();
    this.notices.destroy();
    this.slots.destroy();
    this.stack.destroy();
    void this.audio?.close().catch((error: unknown) => this.fail(error));
    this.stage.destroy();
  }

  /** The screen on top of the stack, by id; null when the stack is empty. */
  get screen(): string | null {
    return this.stack.top?.id ?? null;
  }

  /** The box of the dialogue on screen, or null when no conversation plays. */
  get box(): DialogueShown | null {
    return this.dialogue.shown;
  }

  /** True once the story has ended. */
  get finished(): boolean {
    return this.story.finished;
  }

  /** The number of the ending that the player reached (1 or 2), or 0 while the story has not ended. */
  get ending(): number {
    return this.story.endingNumber;
  }

  /** The language in use. */
  get language(): Language {
    return this.currentLanguage();
  }

  /**
   * Switches the language of the texts and the story, as the settings menu does. The open conversation goes on in
   * the other language, at the same box.
   * @param language - the language to switch to.
   */
  setLanguage(language: Language): void {
    this.texts.setLanguage(language);
  }

  /**
   * Starts a new story at its first scene, on the game screen.
   */
  startNew(): void {
    this.enterGame();
    this.story.begin(START_SCENE, NO_VARS);
    this.dialogue.play(this.story);
  }

  /**
   * Starts a scene of the story from the developer console. The variables stay as they are.
   * @param scene - the label of the scene.
   */
  jumpTo(scene: string): void {
    try {
      this.enterGame();
      this.story.begin(scene, this.story.vars);
      this.dialogue.play(this.story);
    } catch (error) {
      this.fail(error);
    }
  }

  /** The hash of the game's state now: the same text for the same state. A replay compares it at each checkpoint. */
  hash(): string {
    const record = this.story.started ? this.story.snapshot() : null;
    return fingerprint(
      sortedJson({
        screen: this.screen,
        story: record,
        stage: this.story.stage,
        finished: this.story.finished,
        clock: Math.round(this.clock.elapsed * 1000),
        lines: this.dialogue.backlog.length,
        box: this.dialogue.shown?.type ?? null,
      }),
    );
  }

  /** One fixed step: the keys, the clock and the motion by the same real time, the dialogue, and the stage. */
  private update(dt: number): void {
    this.handleKeys();
    const before = this.clock.elapsed;
    this.clock.advance(dt);
    this.dialogue.tick(this.clock.elapsed - before);
    this.motion.advance(dt);
    this.ui.advance(dt);
    this.syncStage();
    this.keys.endFrame();
    this.shortcuts.endFrame();
  }

  /**
   * Reads the keys of this step. The menu key works on the game and on the pause menu. The story's keys work only
   * on the game screen, so that a menu on top of it keeps them to itself.
   */
  private handleKeys(): void {
    const shortcuts = this.shortcuts;
    const top = this.stack.top?.id;
    if (shortcuts.pressed("menu") && (top === "game" || top === "pause")) this.toggleMenu();
    if (this.stack.top?.id !== "game") return;
    this.dialogue.setFastForward(shortcuts.held("fast"));
    if (!this.dialogue.isOpen) return;
    if (this.keys.pressed("advance")) this.dialogue.advance();
    if (this.dialogue.shown?.type === "choice") {
      CHOICE_SHORTCUTS.forEach((action, index) => {
        if (shortcuts.pressed(action)) this.dialogue.choose(index);
      });
    }
  }

  /**
   * Makes the screens and the sound show what the story's boxes say: the background, the music, the place and the
   * sea glass. It runs each step, and does nothing when the story has not moved.
   */
  private syncStage(): void {
    const stage = this.story.stage;
    if (stage.background !== this.applied.background) {
      this.view.setBackground(stage.background, this.ui);
    }
    if (stage.music !== this.applied.music) {
      if (stage.music === null) this.audio?.stopMusic(900);
      else this.audio?.playMusic(stage.music, { crossfadeMs: 1200 });
    }
    this.applied = stage;
    if (this.story.revision === this.seenRevision) return;
    this.seenRevision = this.story.revision;
    const place = this.story.scene;
    if (place !== this.shownPlace) {
      this.shownPlace = place;
      this.view.setPlace(place);
    }
    const glass = Number(this.story.vars.values[GLASS_VAR] ?? 0) > 0 ? 1 : 0;
    if (glass !== this.shownGlass) {
      this.shownGlass = glass;
      this.view.setGlass(glass);
    }
  }

  /** Makes sure the game screen is on top: it pops the menus above it, or it pushes the game onto the title. */
  private enterGame(): void {
    if (this.stack.has("game")) {
      this.stack.popTo("game");
      return;
    }
    this.stack.popTo("title");
    this.stack.push(this.gameScreen);
  }

  /** Opens the pause menu over the game, or closes it when it is open. */
  private toggleMenu(): void {
    const top = this.stack.top?.id;
    if (top === "game") this.stack.push(this.kits.pause.screen);
    else if (top === "pause") this.stack.pop();
  }

  /** Leaves the game for the title, and stops the dialogue. */
  private toTitle(): void {
    this.dialogue.stop();
    this.stack.popTo("title");
  }

  /** Opens the save list or the load list over the menu on top. */
  private openSaves(mode: "save" | "load"): void {
    this.savingNow = mode === "save";
    this.kits.saves.setMode(mode);
    this.slots.update({ mode, t: this.translate });
    this.stack.push(this.kits.saves.screen);
  }

  /** Opens the settings over the menu on top. */
  private openSettings(): void {
    this.kits.settings.refresh();
    this.stack.push(this.kits.settings.screen);
  }

  /**
   * A row of the settings menu was picked: it changes the value, applies it, and writes the menu again.
   * @param id - the row.
   */
  private pickSetting(id: SettingId): void {
    const current = currentSettings(this.settings);
    switch (id) {
      case "speed":
        this.settings.set("textSpeed", nextStep(TEXT_SPEEDS, current.textSpeed));
        break;
      case "delay":
        this.settings.set("autoDelay", nextStep(AUTO_DELAYS, current.autoDelay));
        break;
      case "music":
        this.settings.set("musicVolume", nextStep(VOLUME_STEPS, current.musicVolume));
        break;
      case "sfx":
        this.settings.set("sfxVolume", nextStep(VOLUME_STEPS, current.sfxVolume));
        break;
      case "language": {
        const language = LANGUAGES[(LANGUAGES.indexOf(this.currentLanguage()) + 1) % LANGUAGES.length] ?? "es";
        this.texts.setLanguage(language);
        break;
      }
      case "back":
        this.stack.pop();
        return;
    }
    this.applySettings();
    this.kits.settings.refresh();
  }

  /** Applies the settings to the dialogue box and the sound. */
  private applySettings(): void {
    const values = currentSettings(this.settings);
    this.dialogue.update({ speed: values.textSpeed, autoDelaySeconds: values.autoDelay });
    this.audio?.setVolume("music", values.musicVolume);
    this.audio?.setVolume("ui", values.sfxVolume);
  }

  /** The language in use, as the story and the settings name it. */
  private currentLanguage(): Language {
    return asLanguage(this.texts.language) ?? "es";
  }

  /**
   * Writes every language-dependent part of the screen again. A change of language takes the open conversation
   * into the other language's guion, at the same box.
   * @param tag - the new language tag.
   */
  private paintLanguage(tag: string): void {
    const language = asLanguage(tag);
    if (language === null) return;
    this.settings.set("language", language);
    try {
      this.story.switchLanguage(language);
    } catch (error) {
      this.fail(error);
    }
    if (this.dialogue.isOpen) this.dialogue.play(this.story);
    this.paint();
  }

  /**
   * The portrait of a line's speaker, in a box that slides in from the right and fades in as the line appears.
   * The box is a plain element, so the motion can move it whatever the picture is (a file or a placeholder).
   * @param line - the line on screen.
   * @returns the box, or null when the speaker has no portrait.
   */
  private portraitOf(line: DialogueLine): HTMLElement | null {
    const picture = this.art.portrait(line.who, line.mood);
    if (picture === null) return null;
    const box = document.createElement("div");
    box.className = "vn-portrait";
    box.append(picture);
    placeElement(box, { x: 48, opacity: 0 });
    animateElement(box, { x: 0, opacity: 1 }, { duration: 0.3, ticker: this.ui, ease: "quadOut" });
    return box;
  }

  /** Writes the words again: the screens, the top bar, the dialogue, the notices and the saves. */
  private paint(): void {
    document.documentElement.lang = this.texts.language;
    this.view.refresh();
    this.dialogue.update({ t: this.translate });
    this.notices.update({ t: this.translate });
    this.slots.update({ t: this.translate });
    this.kits.title.refresh();
    this.kits.pause.refresh();
    this.kits.settings.refresh();
    this.kits.saves.refresh();
    this.kits.ending.refresh();
    this.shownPlace = null;
  }

  /** The text lookup that the ui components take. */
  private readonly translate = (key: string, args?: Readonly<Record<string, string | number>>): string =>
    this.texts.t(key, args);

  /** The state that a save holds: the story's record, with the title and the play time for the save list. */
  private draft(): SaveSlotDraft {
    const scene = this.story.scene ?? START_SCENE;
    const key = SCENE_KEYS[scene];
    return {
      title: key === undefined ? scene : this.texts.t(key),
      playTimeMs: this.clock.elapsed * 1000,
      thumbnail: null,
      state: buildSave(this.story.snapshot()),
    };
  }

  /**
   * Writes the autosave, quietly. A failure goes to the log only.
   */
  private autosave(): void {
    if (!this.story.started) return;
    this.slots.autosave().catch((error: unknown) => this.fail(error));
  }

  /**
   * Reads the state of a slot the player picked, and continues from it on the game screen.
   * @param record - the slot's record, as the save list read it.
   */
  private loadState(record: SaveSlotRecord): void {
    const save = parseSave(record.state);
    if (save === null) {
      this.notify("notice.loadFailed", "danger");
      return;
    }
    try {
      this.story.restore(save.story);
    } catch (error) {
      this.fail(error);
      this.notify("notice.loadFailed", "danger");
      return;
    }
    this.enterGame();
    this.dialogue.play(this.story);
    this.notify("notice.loaded", "info");
  }

  /**
   * The story ended: the ending card shows the ending the player reached.
   * @param shown - the end of the conversation.
   */
  private onStoryEnd(shown: DialogueShown): void {
    if (shown.type !== "end") return;
    const ending = this.story.endingNumber;
    this.events.emit("ending", { ending });
    this.kits.ending.show({ ending, boxes: this.story.stepCount });
    this.stack.push(this.kits.ending.screen);
  }

  /** Shows a notice in the language in use. */
  private notify(key: string, tone: "info" | "danger"): void {
    this.notices.notify({ message: this.texts.t(key), tone });
  }

  /** Writes an error to the log. The player sees the notices that the game chose to show. */
  private fail(error: unknown): void {
    logLine("error", `hello-vn: ${describeError(error)}`);
  }
}
