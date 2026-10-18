# Input bindings v1: a player's keys, buttons and sticks

velvet-input-bindings is the file that keeps a player's bindings: which key, mouse button, wheel direction
or gamepad control is bound to each slot of each action. It is written by a game's `ActionMap` and read
back into one, so that the player's rebinding survives a restart.

This document is normative for version 1. The reference implementation is `@velvet/core`
(`js/packages/core`, in `input-store.ts`, `input-controls.ts` and `input-rebind.ts`). The conformance
fixtures are in `docs/formats/fixtures/input-bindings/`. Where this text and the code disagree, the
fixtures decide.

## Conventions

- JSON is UTF-8. Field names are `camelCase`, as in the save envelope (`savedAt`).
- Readers ignore fields that the format does not define, at every level.
- A **name** (an action name or a context name) is 1 to 64 characters. An action name must be unique in
  a file.
- A **key** is a `KeyboardEvent.code` value: 1 to 32 letters or digits, such as `KeyW`, `Space` or `F12`.

## The envelope

| Field | Value |
|---|---|
| `format` | `"velvet-input-bindings"` |
| `version` | `1` |
| `actions` | A list of action entries (below). |

    {
      "format": "velvet-input-bindings",
      "version": 1,
      "actions": [
        { "action": "confirm", "slots": [ { "control": { "type": "key", "key": "Enter" } }, null ] }
      ]
    }

## Actions and slots

Each action entry holds its **slots** as a list. The number of a slot is its position in the list, so
slot 0 is the first entry. A `null` entry is an empty slot: the number is kept, so that a later slot keeps
its number when an earlier one is cleared. A writer ends the list at its last bound slot; a reader accepts
trailing `null` entries too.

A slot holds one binding:

| Field | Required | Meaning |
|---|---|---|
| `control` | yes | The control (below). |
| `scale` | no | A finite number. Default 1. Omitted when it is 1. A negative scale inverts an axis. |
| `axisComponent` | no | `"x"` or `"y"`: the binding feeds one component of a 2D action. Omitted for a digital binding. |
| `context` | no | A name. Default `"default"`, which is omitted. Two actions conflict only in the same context. |

## Controls

Every control is an object with a `type`.

| `type` | Fields | Example |
|---|---|---|
| `key` | `key`: a key (see Conventions). | `{ "type": "key", "key": "KeyW" }` |
| `mouse` | `button`: `left`, `middle` or `right`. | `{ "type": "mouse", "button": "left" }` |
| `wheel` | `direction`: `up`, `down`, `left` or `right`. | `{ "type": "wheel", "direction": "up" }` |
| `pad-button` | `button`: a pad button name (below). | `{ "type": "pad-button", "button": "a" }` |
| `pad-axis` | `axis`: `leftX`, `leftY`, `rightX` or `rightY`. | `{ "type": "pad-axis", "axis": "leftX", "deadzone": 0.25 }` |
| `pad-stick` | `stick`: `left` or `right`; `direction`: `up`, `down`, `left` or `right`. | `{ "type": "pad-stick", "stick": "right", "direction": "up" }` |

The optional fields of the pad controls:

| Field | Applies to | Meaning |
|---|---|---|
| `pad` | all pad controls | A whole number from 0 up: the index of one pad. Omitted: any connected pad. |
| `threshold` | all pad controls | Above 0 and at most 1. The level at which the control is active. Default 0.5. |
| `deadzone` | `pad-axis`, `pad-stick` | At least 0 and below 1. The travel that is ignored. Default 0.2. |

Pad button names follow the standard gamepad mapping: `a`, `b`, `x`, `y`, `leftShoulder`, `rightShoulder`,
`leftTrigger`, `rightTrigger`, `select`, `start`, `leftStick`, `rightStick`, `dpadUp`, `dpadDown`,
`dpadLeft` and `dpadRight`. A trigger is a button with an analog value.

## Reading a file

A reader:

1. parses the text as JSON, or takes the object if it already has one;
2. refuses a file whose `format` is not `"velvet-input-bindings"` (`corrupt`);
3. refuses a `version` that is not a whole number from 1 up (`corrupt`), and a version above the one that it
   reads (`newer-version`);
4. migrates an older file, one version at a time, with the migrations that the game passes in, keyed by the
   version they start from. A step without a migration is `missing-migration`;
5. checks the shape of the current version: an `actions` list, action names that are unique, slot lists,
   controls that are valid, and optional fields of the right type. Any failure is `corrupt`.

The errors are the save errors of `@velvet/core` (`SaveError`), with the codes above.

## Applying a file

A map that applies a file, for the actions that the game lists:

- clears every slot that the action holds now;
- sets each slot of the file in its own number, with the conflict policy `allow`, so that the file's
  conflicts are kept and can be seen with `ActionMap.conflicts()`;
- counts the slots of an action that the game does not list as ignored, and leaves them out.

An action that the file does not mention keeps its current bindings.

## Where the file is kept

The store chooses a place in this order:

1. the settings of the Velvet Desktop bridge (`window.velvet.settings`), under the key `input.bindings`;
2. localStorage, as JSON text under the key `velvet-input:input.bindings`;
3. memory, which lasts until the page reloads.

A game can pass its own settings backend. Saving writes the whole file; clearing writes `null`, which reads
back as nothing saved.

## Display keys

`describeBinding` gives each control its device, a short code, and a stable key id for its name. The game
supplies the text of each id in its locale catalogs (see `LOCALE.md`). A key without a text is shown by its
code.

| Control | Device | Key id |
|---|---|---|
| `key` | `keyboard` | `input.key.<code>`, for example `input.key.Space` or `input.key.KeyW` |
| `mouse` | `mouse` | `input.mouse.left`, `input.mouse.middle`, `input.mouse.right` |
| `wheel` | `mouse` | `input.wheel.up`, `input.wheel.down`, `input.wheel.left`, `input.wheel.right` |
| `pad-button` | `gamepad` | `input.pad.<button>`, for example `input.pad.a` or `input.pad.dpadUp` |
| `pad-axis` | `gamepad` | `input.pad.axis.<axis>`, for example `input.pad.axis.leftX` |
| `pad-stick` | `gamepad` | `input.pad.stick.<stick>.<direction>`, for example `input.pad.stick.left.up` |

The device names that a UI shows are `input.device.keyboard`, `input.device.mouse` and `input.device.gamepad`.

## Fixtures

| File | Expected result |
|---|---|
| `fixtures/input-bindings/v1.json` | Reads as version 1. Applied to the actions `confirm`, `move`, `attack` and `zoom`, it sets 9 slots and ignores 1 (the action `retired-action`). It has every control type and every optional field. |
| `fixtures/input-bindings/invalid-control.json` | Refused as `corrupt`: `dpadCenter` is not a pad button name. |
| `fixtures/input-bindings/newer-version.json` | Refused as `newer-version`. |

The core tests (`input-store.test.ts`) read these files and check each result.
