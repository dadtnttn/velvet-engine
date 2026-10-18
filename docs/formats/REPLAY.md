# Replay v1: a recorded session and its checkpoints

velvet-replay is the JSON document that holds one recorded session of a Velvet JS game: the inputs of
each frame, the named seeds and the starting save, and the game's state hashes at checkpoints. A reader
runs the game again from those, gives it the same inputs at the same frames, and compares the hashes. The
first checkpoint that differs is where the replay and the recording part ways.

This document is normative for version 1. The reference implementation is `@velvet/testing`
(`js/packages/testing`): `replay-format.ts` validates a document, `recorder.ts` writes one, `replay.ts`
plays one, and `driver.ts` feeds the inputs. The conformance fixtures are in
`docs/formats/fixtures/replay/`, and `cases.json` there lists each case with the start of its error message.
Where this text and the code disagree, the fixtures decide.

## Conventions

- JSON is UTF-8. Field names are `camelCase`, as in the save envelope.
- Readers ignore fields that the format does not define, at every level.
- A **text** is 1 to 128 characters, unless a field says otherwise. A pad's `id` may be empty.
- A **frame** is a whole number from 0 (see [Timing](#timing-frames-updates-and-checkpoints)).
- A **position** is a logical stage point: the stage's logical size (1920 by 1080 by default), with y going down.
- A **key** is a `KeyboardEvent.code` value: 1 to 32 letters or digits, such as `KeyW`, `Space` or `F12`.

## The document

| Field | Required | Value |
|---|---|---|
| `format` | yes | `"velvet-replay"` |
| `version` | yes | `1` |
| `game` | yes | The id of the game that was recorded. A replay is played by a build of the same game. |
| `gameVersion` | no | The game's version. Readers show it; they do not compare it. |
| `rate` | yes | Update steps per second of the loop that ran the session: a number above 0 and at most 1000. |
| `frames` | yes | The number of updates in the session: a whole number from 0 up. |
| `seeds` | yes | An object of named seeds (see [Seeds](#seeds-and-the-starting-save)). `{}` when there are none. |
| `save` | yes | `null` for a new game, or the starting save (see [Seeds](#seeds-and-the-starting-save)). |
| `checkEvery` | yes | Frames between checkpoints: a whole number from 1 up. |
| `checkpoints` | yes | A list of checkpoints (see [Checkpoints](#checkpoints)). May be empty. |
| `inputs` | yes | A list of inputs (see [Inputs](#inputs)). May be empty. |

    {
      "format": "velvet-replay",
      "version": 1,
      "game": "test-game",
      "rate": 60,
      "frames": 120,
      "seeds": { "coins": 7 },
      "save": null,
      "checkEvery": 60,
      "checkpoints": [ { "frame": 60, "hash": "3e64f15224fc6a56" }, { "frame": 120, "hash": "0b7c1d9e42a8f305" } ],
      "inputs": [ { "frame": 3, "type": "key", "code": "Space", "down": true } ]
    }

## Timing: frames, updates and checkpoints

- A **frame** is one update step of the game's loop. The frames of a session are numbered from 0, and
  `frames` is the count of updates, so the last frame is `frames - 1`.
- An input at frame `N` happens after `N` updates have run and before update `N` runs. An input at frame
  `frames` happens after the last update; a reader applies it and runs no update after it.
- A **checkpoint** at frame `N` holds the game's hash after `N` updates. Checkpoints sit at multiples of
  `checkEvery`, from `checkEvery` up to `frames`. Frames between checkpoints are not compared.
- Each update takes `1 / rate` seconds of game-loop time. A reader runs the loop at `rate`, so one frame is one
  update.

## Seeds and the starting save

**Seeds** name the random sources that the game built its session from, such as `{ "board": 42 }`. A seed is
a safe whole number from 0 up to 2^53 - 1, or a text of 1 to 40 decimal digits (for seeds above 2^53, as the
cards package writes them). A reader hands the seeds to the game unchanged, so the game can build the same
random sources as the recording.

**The save** is the state that the session started from. It is `null` for a new game, or an envelope in the
save file format (`velvet-save`): `format` is `"velvet-save"`, `version` is a whole number from 1 up, `game`
is the game's id, `savedAt` is a text, and `data` holds the game's save data at the store's current version.
A save that is older than the store is migrated before it is recorded, so the reader needs no migrations.

## Checkpoints

A checkpoint is `{ "frame": N, "hash": "..." }`. `frame` is a multiple of `checkEvery` from `checkEvery` up to
`frames`, and the checkpoints are in strictly increasing order of `frame`. `hash` is the text that the game's
own hash function returned after `N` updates: a non-empty text. Two equal game states give equal texts. The
format only compares texts for equality, so any hash function will do, as long as the same state always gives
the same text.

## Inputs

Each input is an object with a `frame` and the fields of its `type`. Inputs are in frame order: a frame's
inputs come together, in the order in which they happened, and a reader applies them in that order. The
types are:

| `type` | Fields | Meaning |
|---|---|---|
| `key` | `code`, `down`, `skipped`? | A key went down (`down` true) or up. |
| `button` | `button`, `down`, `x`, `y`, `skipped`? | A mouse button went down or up, with the pointer at `x`, `y`. `button` is `left`, `middle` or `right`. |
| `pointer` | `x`, `y` | The pointer moved to `x`, `y`. |
| `wheel` | `deltaX`, `deltaY`, `deltaMode`, `skipped`? | One wheel tick. `deltaMode` is 0 (pixels), 1 (lines) or 2 (pages), as in the DOM. |
| `pad` | `pad`, `connected`, and when connected: `id`, `buttons`, `axes` | The whole state of one gamepad at this frame. |
| `blur` | none | The window lost focus, so every held control is released. |

**Skipped presses.** `skipped` is written only when it is true. It marks a press that the page had already
used before the game's action maps saw it: its default was prevented, or its target is a control (an `input`,
`textarea`, `select`, `button`, an element with `contenteditable`, or a `menuitem`, or something inside one).
A reader prevents the default of the event before it delivers it, so that an action map ignores the press as
it did in the recording. Only presses (a key going down, a button going down, a wheel tick) can be skipped;
releases are never skipped.

**Pads.** `pad` is an index from 0 to 15. `buttons` is a list of at most 32 entries of `{ "pressed": true|false,
"value": 0..1 }`, and `axes` a list of at most 16 numbers from -1 to 1. The buttons and axes follow the
standard gamepad mapping: buttons 0 to 15 are `a`, `b`, `x`, `y`, `leftShoulder`, `rightShoulder`,
`leftTrigger`, `rightTrigger`, `select`, `start`, `leftStick`, `rightStick`, `dpadUp`, `dpadDown`, `dpadLeft` and
`dpadRight`; axes 0 to 3 are `leftX`, `leftY`, `rightX` and `rightY`. A pad entry replaces the whole state of
that pad from its frame on. A disconnection is `{ "pad": 0, "connected": false }`. The recorder writes a pad
only where its state differs from the frame before, sampled just before that frame's update.

**Repeats.** A key that the keyboard repeats (a keydown while the key is already down) is not written.

## Reading a replay

A reader does these steps, in order:

1. Validate the document (see [Validation](#validation)), and refuse it on the first failure.
2. Start the game from the replay's seeds and save. The game is the same build as the recording's.
3. Run the loop at `rate`, so that one frame is one update.
4. For each frame `f` from 0 to `frames - 1`: apply the inputs at frame `f` in order, then run one update. If
   `f + 1` is a checkpoint frame, compare the game's hash with the checkpoint's text.
5. Apply the inputs at frame `frames` in order, without an update.
6. The first checkpoint whose text differs ends the replay. It is reported with the frame of the checkpoint, the
   last checkpoint frame that matched (0 when none did), and the two texts.
7. Otherwise the replay passes.

## Validation

A document is refused when:

- the `format` is not `"velvet-replay"`, or the `version` is not 1 (a reader of version 1 refuses version 2 and
  every later version);
- `game` is not a text, `gameVersion` is present but not a text, or a text is empty or longer than 128 characters;
- `rate` is not a finite number above 0 and at most 1000, or `frames` or `checkEvery` is not a whole number in its
  range;
- `seeds` is not an object, or a seed is not a safe whole number from 0 up, or decimal digits of 1 to 40;
- `save` is neither `null` nor a `velvet-save` envelope with its `data` present;
- `checkpoints` is not a list, a frame is not a multiple of `checkEvery` in `1..frames`, the frames are not strictly
  increasing, or a hash is not a non-empty text;
- `inputs` is not a list, an input's `frame` is not a whole number in `0..frames`, the frames go backwards, an input
  has a type that is not listed, or an input's fields are missing or out of range (a key code, a button name, a
  delta mode from 0 to 2, a pad index from 0 to 15, a pad's button and axis values, or a `skipped` that is not
  `true` when it is written).

Each refusal names the field, for example `inputs[3].frame must not be below 7: the inputs are in frame order`.

## Versions

Version 1 is the only version that this document defines. A change that alters the meaning of a field, or that a
reader must understand to play a session correctly, is a new version. Adding a field that a version 1 reader
may ignore without changing any replay's result keeps the version. A writer writes only fields that the version
defines.

## Fixtures

The folder `docs/formats/fixtures/replay/` holds the conformance cases.

- `valid/empty.json`: a session with no frames, no seeds and no inputs.
- `valid/session.json`: a full session: a pad connected and disconnected, keys, a skipped press, a button, a pointer
  move, a wheel tick, a blur, a 20-digit seed, and two checkpoints.
- `valid/with-save.json`: a session at 30 Hz that starts from a save envelope.
- `invalid/*.json`: one document for each rule above, each with one fault.
- `cases.json`: the list of cases. Each names its file, whether a reader must accept it, and for a refusal the start
  of the message it must give.

A reader that passes every valid case, and refuses every invalid case with the message that the list names, reads
version 1 correctly.
