// The dialogue box: a visual-novel box that plays a conversation. It shows the speaker's name on the plate,
// the portrait that the caller resolves, the text written out letter by letter, and the choices under the
// text. It never decides a game rule. The conversation (a guion session) says what the box shows, and the box
// hands back the player's choices. Everything the box knows about a line comes from that session.
//
// Time: the box has no timers of its own. The game calls `tick(seconds)` from its loop, so the text and the
// auto-advance stop when the game is paused, the same as the rest of the game.
//
// Going on: `advance()` finishes a line that is still being written, and otherwise goes on to the next box.
// A click on the text does the same, and so does the Next button, which takes the focus when a line is shown,
// so a keyboard or a pad (through startPadMenuKeys) can go on with Enter. The buttons Auto (go on by itself)
// and Skip (go on through the read text at once, until a choice or the end) are toggles. Log opens the backlog
// of the lines and the choices so far.
//
// The types of the session are declared here, the same shape as @velvet/guion's `Shown` and its `Conversation`,
// so the UI package does not import guion. A guion `Conversation` can be passed to `play` as it is.

import { createButton } from "./components/button";
import type { VuComponent } from "./components/component";
import { createElement, releaseFocus } from "./components/dom";
import { createMenu, type Menu, type MenuItem } from "./components/menu";
import { createPlate, type Plate } from "./components/plate";
import { setShown } from "./dirty";
import type { TranslateFn } from "./rebind-panel";

/** A line of dialogue, as the conversation reports it. */
export interface DialogueLine {
  readonly type: "line";
  /** The character id of the speaker. Narration has the id "narrator". */
  readonly who: string;
  /** The name on the plate. Empty for narration and sounds. */
  readonly name: string;
  /** The text of the line. */
  readonly text: string;
  /** How the speaker looks, such as "smiling", or null. */
  readonly mood: string | null;
  /** The kind of line, such as "dialogue", "narration", "sound" or "thought". Optional. */
  readonly kind?: string;
}

/** One option of a choice. An option that is not available cannot be taken. */
export interface DialogueOption {
  readonly label: string;
  readonly available: boolean;
}

/** A choice that waits for the player. */
export interface DialogueChoice {
  readonly type: "choice";
  /** The question that the choice asks, or null. */
  readonly prompt: string | null;
  readonly options: readonly DialogueOption[];
}

/** The conversation hands over to a visual-novel scene. */
export interface DialogueHandOver {
  readonly type: "vn";
  /** The scene that takes over. */
  readonly to: string;
}

/** The conversation is over. */
export interface DialogueEnd {
  readonly type: "end";
}

/** What the box shows: a line, a choice, a hand-over to a scene, or the end. */
export type DialogueShown = DialogueLine | DialogueChoice | DialogueHandOver | DialogueEnd;

/** The part of a conversation that the box plays. A guion `Conversation` has these methods. */
export interface DialogueSession {
  /** What the box shows now. Changes nothing. */
  current(): DialogueShown;
  /** Goes past the line on screen, and returns what follows. */
  next(): DialogueShown;
  /** Takes an option of the choice on screen, and returns what follows. */
  choose(index: number): DialogueShown;
}

/** One line or choice that was shown, for the backlog. */
export interface BacklogEntry {
  /** The name of the speaker, or an empty string for narration and for a choice that the player took. */
  readonly name: string;
  /** The text of the line, or the label of the option that was taken. */
  readonly text: string;
  /** "line" for a line, "choice" for an option that the player took. */
  readonly kind: "line" | "choice";
}

/** Props of the dialogue box. They can change while a conversation plays. */
export interface DialogueBoxProps {
  /** Goes on by itself, a while after each line is written. Default false. */
  readonly autoAdvance?: boolean;
  /** Letters per second. Zero writes each line at once. Default 40. */
  readonly speed?: number;
  /** How much faster the text runs while fast-forward is on. Default 4. Must be at least 1. */
  readonly fastForwardFactor?: number;
  /** Seconds that a finished line waits before auto-advance goes on. Default 1.5. */
  readonly autoDelaySeconds?: number;
  /** Gives the portrait of a line: the element to show, or null for none. The caller resolves art ids here. */
  readonly portrait?: (line: DialogueLine) => Element | null;
  /** The text lookup for the buttons and the backlog. Without one, the English fallbacks are used. */
  readonly t?: TranslateFn;
}

/** Options for {@link createDialogueBox}: the props, and the hooks for voice and sound. */
export interface DialogueBoxOptions extends DialogueBoxProps {
  /** Runs when a line is shown, with the line. Use it to start a voice clip. */
  readonly onLine?: (line: DialogueLine) => void;
  /** Runs for each letter that the text writes (not for spaces). Throttle it in the game if it is loud. */
  readonly onLetter?: (letter: string) => void;
  /** Runs when a choice is shown, with the choice. */
  readonly onChoice?: (choice: DialogueChoice) => void;
  /** Runs once when the conversation ends, or hands over to a visual-novel scene. */
  readonly onEnd?: (shown: DialogueEnd | DialogueHandOver) => void;
}

/** The dialogue box. It starts hidden, and shows itself when a conversation plays. */
export interface DialogueBox extends VuComponent<DialogueBoxProps> {
  /** True while a conversation plays on the box. */
  readonly isOpen: boolean;
  /**
   * What the box shows now: the line or the choice on screen, or the end or hand-over that finished the
   * conversation (so the game can read `to`). Null before the first conversation.
   */
  readonly shown: DialogueShown | null;
  /** The lines and choices so far, oldest first. */
  readonly backlog: readonly BacklogEntry[];
  /** True while fast-forward is on. */
  readonly fastForward: boolean;
  /** True while skip is on. */
  readonly skipping: boolean;
  /**
   * Plays a conversation on the box. The first box is shown at once, so a conversation that has already
   * ended calls `onEnd` straight away. A conversation that is playing is replaced.
   */
  play(session: DialogueSession): void;
  /** Hides the box without a call to `onEnd`. The game uses it when it closes the conversation itself. */
  stop(): void;
  /**
   * Advances the text and the auto-advance timer by `seconds` of game time. Call it once per update of the game.
   * Nothing moves when no conversation plays, or while the game is paused and the update does not run.
   */
  tick(seconds: number): void;
  /** Goes on: finishes a line that is being written, or goes on to the next box. Does nothing on a choice. */
  advance(): void;
  /** Takes an option of the choice on screen. An option that is not available is ignored. */
  choose(index: number): void;
  /** Turns fast-forward on or off. It runs the text and the auto-advance faster. */
  setFastForward(on: boolean): void;
  /** Turns skip on or off. While it is on, each line is written at once and goes on by itself, up to a choice. */
  setSkipping(on: boolean): void;
  /** Opens the backlog. Its close button takes the focus. */
  openBacklog(): void;
  /** Closes the backlog, and gives the focus back to the element that had it. */
  closeBacklog(): void;
}

/** The default letter speed, in letters per second. */
const DEFAULT_SPEED = 40;
/** The default speed-up of fast-forward. */
const DEFAULT_FAST_FORWARD = 4;
/** The default wait on a finished line before auto-advance goes on, in seconds. */
const DEFAULT_AUTO_DELAY = 1.5;
/** The most lines the backlog keeps. */
const BACKLOG_LIMIT = 200;

/** The state of the props, with every default applied. */
interface ResolvedProps {
  autoAdvance: boolean;
  speed: number;
  fastForwardFactor: number;
  autoDelaySeconds: number;
  portrait: ((line: DialogueLine) => Element | null) | undefined;
  t: TranslateFn | undefined;
}

/**
 * Creates the dialogue box. Append its `el` to the stage's root, above the board and below the notices.
 * @param options - the props and the hooks.
 * @throws RangeError when a speed, a factor or a delay is out of range.
 */
export function createDialogueBox(options: DialogueBoxOptions): DialogueBox {
  let props: ResolvedProps = {
    autoAdvance: options.autoAdvance ?? false,
    speed: checkedSpeed(options.speed ?? DEFAULT_SPEED, "speed"),
    fastForwardFactor: checkedFactor(options.fastForwardFactor ?? DEFAULT_FAST_FORWARD),
    autoDelaySeconds: checkedSeconds(options.autoDelaySeconds ?? DEFAULT_AUTO_DELAY),
    portrait: options.portrait,
    t: options.t,
  };
  let session: DialogueSession | null = null;
  let shown: DialogueShown | null = null;
  /** The letters of the line on screen, as code points, and how many are written. */
  let letters: string[] = [];
  let written = 0;
  /** Letters that the typewriter owes, counted as a fraction so that a slow frame still moves it. */
  let due = 0;
  let lineDone = true;
  /** Seconds that a finished line has waited, for auto-advance. */
  let waited = 0;
  let fastForwardOn = false;
  let skipOn = false;
  const log: BacklogEntry[] = [];
  let logReturnFocus: HTMLElement | null = null;
  let destroyed = false;

  const el = createElement("div", "vu-dialogue");
  el.setAttribute("role", "region");
  el.hidden = true;
  const portraitSlot = createElement("div", "vu-dialogue-portrait");
  portraitSlot.hidden = true;
  const panel: Plate = createPlate({ title: "" });
  panel.el.classList.add("vu-dialogue-panel");
  const textLine = createElement("p", "vu-dialogue-text");
  const choiceMenu: Menu = createMenu({ items: [], label: "", onSelect: (id) => choose(Number(id)) });
  choiceMenu.el.classList.add("vu-dialogue-choices");
  const actions = createElement("div", "vu-dialogue-actions");
  const nextButton = createButton({ label: "", onPress: () => advance() });
  const autoButton = createButton({ label: "", pressed: props.autoAdvance, onPress: () => applyProps({ autoAdvance: !props.autoAdvance }) });
  const skipButton = createButton({ label: "", pressed: false, onPress: () => setSkipping(!skipOn) });
  const logButton = createButton({ label: "", onPress: () => openBacklog() });
  actions.append(nextButton.el, autoButton.el, skipButton.el, logButton.el);
  panel.body.append(textLine, choiceMenu.el, actions);
  el.append(portraitSlot, panel.el);

  const backlogPlate: Plate = createPlate({ title: "" });
  backlogPlate.el.classList.add("vu-dialogue-backlog");
  backlogPlate.el.hidden = true;
  backlogPlate.el.setAttribute("role", "dialog");
  const logList = document.createElement("ol");
  logList.className = "vu-dialogue-log";
  const logClose = createButton({ label: "", onPress: () => closeBacklog() });
  backlogPlate.body.append(logList, logClose.el);
  el.append(backlogPlate.el);

  /** The text of a key, or its English fallback when the lookup has no text for it. */
  const text = (key: string, fallback: string): string => {
    if (props.t === undefined) return fallback;
    const value = props.t(key);
    return value === key ? fallback : value;
  };

  /** Writes the words of the buttons and the backlog in the language in use. */
  const paintTexts = (): void => {
    nextButton.update({ label: text("dialogue.next", "Next") });
    autoButton.update({ label: text("dialogue.auto", "Auto") });
    skipButton.update({ label: text("dialogue.skip", "Skip") });
    logButton.update({ label: text("dialogue.log", "Log") });
    logClose.update({ label: text("dialogue.log.close", "Close") });
    backlogPlate.update({ title: text("dialogue.log.title", "Log") });
    choiceMenu.update({ label: text("dialogue.choices", "Choices") });
  };

  /** Writes the portrait of a line into the slot, or hides the slot. */
  const paintPortrait = (line: DialogueLine | null): void => {
    const node = line === null || props.portrait === undefined ? null : props.portrait(line);
    portraitSlot.replaceChildren(...(node === null ? [] : [node]));
    setShown(portraitSlot, node !== null);
  };

  /** Merges new props into the current ones, and redraws what they change. */
  const applyProps = (next: Partial<DialogueBoxProps>): void => {
    if (destroyed) return;
    props = {
      autoAdvance: next.autoAdvance ?? props.autoAdvance,
      speed: next.speed === undefined ? props.speed : checkedSpeed(next.speed, "speed"),
      fastForwardFactor: next.fastForwardFactor === undefined ? props.fastForwardFactor : checkedFactor(next.fastForwardFactor),
      autoDelaySeconds: next.autoDelaySeconds === undefined ? props.autoDelaySeconds : checkedSeconds(next.autoDelaySeconds),
      portrait: next.portrait ?? props.portrait,
      t: next.t ?? props.t,
    };
    autoButton.update({ pressed: props.autoAdvance });
    paintTexts();
    if (shown?.type === "line") paintPortrait(shown);
  };

  /** Writes the next letters of the line, up to `target`, and tells the game about each one. */
  const writeTo = (target: number): void => {
    const from = written;
    written = Math.min(letters.length, target);
    for (let index = from; index < written; index += 1) {
      const letter = letters[index];
      if (letter !== undefined && letter.trim() !== "") options.onLetter?.(letter);
    }
    textLine.textContent = letters.slice(0, written).join("");
    if (written >= letters.length) lineDone = true;
  };

  /** Writes the whole line at once. */
  const writeAll = (): void => {
    writeTo(letters.length);
  };

  /** Starts a line: it is written from the first letter, or at once when the speed is zero. */
  const startLine = (line: DialogueLine): void => {
    letters = Array.from(line.text);
    written = 0;
    due = 0;
    waited = 0;
    lineDone = letters.length === 0;
    textLine.textContent = "";
    if (props.speed <= 0 || skipOn) writeAll();
  };

  /** Takes the focus into the box when a box needs it: the Next button, or the first option of a choice. */
  const focusBox = (node: HTMLElement): void => {
    if (node.isConnected) node.focus({ preventScroll: true });
  };

  const showLine = (line: DialogueLine, takeFocus: boolean): void => {
    setShown(choiceMenu.el, false);
    setShown(nextButton.el, true);
    panel.update({ title: line.name });
    paintPortrait(line);
    startLine(line);
    log.push({ name: line.name, text: line.text, kind: "line" });
    trimLog();
    repaintLog();
    options.onLine?.(line);
    const focus = document.activeElement;
    const focusLost = focus === null || focus === document.body;
    if (takeFocus || focusLost || el.contains(focus)) focusBox(nextButton.el);
  };

  const showChoice = (choice: DialogueChoice, takeFocus: boolean): void => {
    letters = [];
    written = 0;
    lineDone = true;
    textLine.textContent = choice.prompt ?? "";
    panel.update({ title: "" });
    paintPortrait(null);
    const items: MenuItem[] = choice.options.map((option, index) => ({
      id: String(index),
      label: option.label,
      disabled: !option.available,
    }));
    choiceMenu.update({ items });
    setShown(choiceMenu.el, true);
    setShown(nextButton.el, false);
    options.onChoice?.(choice);
    if (takeFocus || el.contains(document.activeElement) || document.activeElement === document.body) {
      choiceMenu.focus(0);
    }
  };

  /** The conversation is over, or has handed over. The box hides and the game hears of it once. */
  const finish = (end: DialogueEnd | DialogueHandOver): void => {
    session = null;
    shown = end;
    letters = [];
    el.hidden = true;
    releaseFocus(el);
    options.onEnd?.(end);
  };

  /** Shows what a conversation returned. */
  const show = (next: DialogueShown, takeFocus: boolean): void => {
    shown = next;
    if (next.type === "line") showLine(next, takeFocus);
    else if (next.type === "choice") showChoice(next, takeFocus);
    else finish(next);
  };

  /** Adds an entry to the backlog, and keeps only the newest ones. */
  const trimLog = (): void => {
    if (log.length > BACKLOG_LIMIT) log.splice(0, log.length - BACKLOG_LIMIT);
  };

  /** Writes the backlog list again, newest at the bottom. */
  const repaintLog = (): void => {
    if (backlogPlate.el.hidden) return;
    const rows = log.map((entry) => {
      const row = document.createElement("li");
      row.className = "vu-dialogue-log-item";
      row.dataset["kind"] = entry.kind;
      if (entry.name !== "") row.append(createElement("span", "vu-dialogue-log-name", entry.name));
      row.append(createElement("span", "vu-dialogue-log-text", entry.text));
      return row;
    });
    logList.replaceChildren(...rows);
    logList.scrollTop = logList.scrollHeight;
  };

  /** Takes an option of the choice on screen, and shows what follows it. */
  const choose = (index: number): void => {
    if (destroyed || session === null || shown === null || shown.type !== "choice") return;
    const option = shown.options[index];
    if (option === undefined || !option.available) return;
    log.push({ name: "", text: option.label, kind: "choice" });
    trimLog();
    repaintLog();
    show(session.choose(index), false);
  };

  function advance(): void {
    if (destroyed || session === null || shown === null) return;
    if (shown.type !== "line") return;
    if (!lineDone) {
      writeAll();
      return;
    }
    show(session.next(), false);
  }

  /** Turns skip on or off. A line that is being written is finished at once when skip turns on. */
  function setSkipping(on: boolean): void {
    skipOn = on;
    skipButton.update({ pressed: on });
    if (on && shown?.type === "line" && !lineDone) writeAll();
  }

  function openBacklog(): void {
    if (destroyed || !backlogPlate.el.hidden) return;
    logReturnFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    backlogPlate.el.hidden = false;
    repaintLog();
    logClose.el.focus({ preventScroll: true });
  }

  function closeBacklog(): void {
    if (destroyed || backlogPlate.el.hidden) return;
    backlogPlate.el.hidden = true;
    const back = logReturnFocus;
    logReturnFocus = null;
    if (back !== null && back.isConnected && el.contains(back)) back.focus({ preventScroll: true });
    else if (!el.hidden) nextButton.el.focus({ preventScroll: true });
    releaseFocus(backlogPlate.el);
  }

  /** Escape closes the backlog, and the box keeps the key otherwise. */
  const onBacklogKeyDown = (event: KeyboardEvent): void => {
    if (event.key !== "Escape") return;
    event.preventDefault();
    closeBacklog();
  };
  backlogPlate.el.addEventListener("keydown", onBacklogKeyDown);

  /** A click on the text goes on, as a click in a visual novel does. */
  const onTextClick = (): void => {
    advance();
  };
  textLine.addEventListener("click", onTextClick);

  paintTexts();

  return {
    el,
    get isOpen(): boolean {
      return session !== null;
    },
    get shown(): DialogueShown | null {
      return shown;
    },
    get backlog(): readonly BacklogEntry[] {
      return log.map((entry) => ({ ...entry }));
    },
    get fastForward(): boolean {
      return fastForwardOn;
    },
    get skipping(): boolean {
      return skipOn;
    },
    play(next: DialogueSession): void {
      if (destroyed) return;
      session = next;
      shown = null;
      el.hidden = false;
      show(next.current(), true);
    },
    stop(): void {
      if (destroyed) return;
      session = null;
      shown = null;
      letters = [];
      el.hidden = true;
      if (!backlogPlate.el.hidden) closeBacklog();
      releaseFocus(el);
    },
    tick(seconds: number): void {
      if (destroyed || session === null || shown === null || shown.type !== "line") return;
      if (!Number.isFinite(seconds) || seconds < 0) return;
      const pace = fastForwardOn ? props.fastForwardFactor : 1;
      if (!lineDone) {
        if (skipOn || props.speed <= 0) {
          writeAll();
        } else {
          due += seconds * props.speed * pace;
          const target = Math.floor(due);
          if (target > written) writeTo(target);
        }
      }
      if (!lineDone) return;
      if (!skipOn && !props.autoAdvance) return;
      waited += seconds;
      const needed = skipOn ? 0 : props.autoDelaySeconds / pace;
      if (waited >= needed) advance();
    },
    advance,
    choose,
    setFastForward(on: boolean): void {
      fastForwardOn = on;
    },
    setSkipping,
    openBacklog,
    closeBacklog,
    update: applyProps,
    destroy(): void {
      if (destroyed) return;
      destroyed = true;
      session = null;
      shown = null;
      backlogPlate.el.removeEventListener("keydown", onBacklogKeyDown);
      textLine.removeEventListener("click", onTextClick);
      choiceMenu.destroy();
      nextButton.destroy();
      autoButton.destroy();
      skipButton.destroy();
      logButton.destroy();
      logClose.destroy();
      panel.destroy();
      backlogPlate.destroy();
      releaseFocus(el);
      el.remove();
    },
  };
}

/**
 * Checks a letter speed: a number from zero, where zero writes each line at once.
 * @param value - the speed.
 * @param name - the name for the message.
 * @throws RangeError when it is not a finite number from zero.
 */
function checkedSpeed(value: number, name: string): number {
  if (!Number.isFinite(value) || value < 0) throw new RangeError(`${name} must be a number from 0, got ${value}`);
  return value;
}

/**
 * Checks the fast-forward factor: a number of at least one.
 * @param value - the factor.
 * @throws RangeError when it is not a finite number of at least 1.
 */
function checkedFactor(value: number): number {
  if (!Number.isFinite(value) || value < 1) throw new RangeError(`fastForwardFactor must be a number of at least 1, got ${value}`);
  return value;
}

/**
 * Checks a delay in seconds.
 * @param value - the delay.
 * @throws RangeError when it is not a finite number from zero.
 */
function checkedSeconds(value: number): number {
  if (!Number.isFinite(value) || value < 0) throw new RangeError(`autoDelaySeconds must be a number from 0, got ${value}`);
  return value;
}
