# @velvet/achievements

The achievements of a Velvet JS game: counters that the game advances, goals that unlock achievements (a
counter that reaches a number, all or any of other achievements, or a manual unlock by the game), progress
notices on the way, hidden achievements, gallery order, points, and the progress saved between runs. The
format is in `docs/formats/ACHIEVEMENTS.md`; the shared conformance cases are in
`docs/formats/fixtures/achievements/`.

The package has no platform code. A Steam or other bridge receives every unlock through the `onUnlock` hook of
the runtime, and nothing else in this package knows about a platform.

## The definitions file

A game ships one definitions file, checked when it is read:

    {
      "format": "velvet-achievements",
      "version": 1,
      "kind": "definitions",
      "counters": [{ "id": "kills", "max": 100, "persist": true }],
      "achievements": [
        {
          "id": "veteran",
          "name": "ach.veteran.name",
          "description": "ach.veteran.desc",
          "icon": "ach.veteran",
          "group": "combat",
          "order": 2,
          "points": 25,
          "progress_steps": [0.25, 0.5, 0.75],
          "goal": { "counter": "kills", "at_least": 40 }
        }
      ]
    }

The names, descriptions and hints are i18n keys, and the icons are art ids: the package never reads their
texts or files. A file that breaks a rule is refused as a whole, with every problem and its path.

## Usage

    import { createAchievements } from "@velvet/achievements";

    const achievements = createAchievements({
      definitions: achievementsJson, // the JSON text, or the parsed object (a JSON import)
      clock: Date.now,               // the time of each unlock; a test passes its own clock
      dev: false,                    // true in a dev build: enables force and resetAll
    });

    await achievements.load();       // once, at start: reads the saved progress and merges it
    achievements.increment("kills"); // returns the new value; clamped to the counter's max
    achievements.setCounter("gold", 500);
    achievements.unlock("prologue"); // manual goals only; returns false when already unlocked

    achievements.isUnlocked("veteran");
    achievements.progress("veteran"); // { current, target, fraction }
    achievements.list();              // the gallery, in order, with masked hidden achievements
    achievements.points;              // the points that are unlocked
    achievements.unlockedCount;

Calls with a value that is not a safe whole number, a negative amount, or a name that the file does not declare
are refused with an `AchievementError`. Its `code` says which rule broke (`argument`, `unknown-counter`,
`unknown-achievement`, `not-manual`, `dev-only`).

## Events

    achievements.on("unlocked", ({ id, time }) => { /* an achievement unlocked */ });
    achievements.on("progress", ({ id, step }) => { /* a locked achievement reached a progress step */ });
    achievements.on("changed", () => { /* counters or unlocks changed: redraw the gallery */ });

Each call sends its events after its whole change is in place: the unlocks in the order of the cascade, then the
progress notices, then `changed`. A listener that throws does not stop the others; the first error is thrown to
the caller at the end of the call.

A notice for each unlock, with `createNotices` from `@velvet/ui` (shown here as an example only; this package
does not import the UI):

    import { createNotices } from "@velvet/ui";

    const notices = createNotices({ host: gameElement });
    achievements.on("unlocked", ({ id }) => {
      notices.notify({ message: translate(`ach.${id}.name`), key: id });
    });

`translate` stands for the game's own text lookup, such as `@velvet/i18n`.

## Goals and cascades

- A counter goal is met when the counter holds `at_least`.
- An `all` goal is met when every listed achievement is unlocked.
- An `any` goal is met when `count` of the listed achievements are unlocked.
- A manual goal is unlocked by the game with `unlock(id)`.

An unlock can complete the goals that depend on it, and so on. The cascade runs in a fixed order (the order of
the file, and the order of the unlocks), so the same progress always gives the same events. The definitions may
not contain loops: `all` and `any` goals that refer to each other are refused.

## Hidden achievements

A hidden achievement that is still locked is masked. `list()` gives it no name, description or icon, and no
progress; it gives its `hint` key when it has one, so the gallery can show "?" with a hint. A hidden achievement
sends no progress events. `progress(id)` still gives its numbers, and the game decides what to show with them.

## Saved progress

The progress is saved as a versioned document through the first of these that is there:

1. the `backend` option, a storage with `get(key)` and `set(key, value)` (the same shape as the save and settings
   backends of `@velvet/core`);
2. the bridge of Velvet Desktop, when it is there (or the `bridge` option, `null` to skip it);
3. localStorage, under `velvet-achievements:` and the key (or the `storage` option);
4. memory.

The `key` option (default `achievements`) keeps two games on one storage apart.

- A change saves after `debounceMs` of quiet (250 ms by default), so a burst of changes is one write.
- `save()` writes now and resolves `true`, or `false` when the storage refuses. It never throws.
- `load()` resolves to a status: `fresh` (nothing saved), `loaded`, `damaged` (the save could not be used; it is
  reported once, with its issues, and the store starts from the progress it has), or `unavailable` (the storage
  refused the read). A damaged save never stops the game.
- A load merges with the progress of the run. A goal that the saved counters meet unlocks at once, which covers a
  game that stopped between a change and its save.
- Counters with `persist: false` are not saved: they start at 0 in each run.

## The platform hook

    createAchievements({
      definitions,
      onUnlock: (unlock) => {
        // A Steam or other bridge reports the unlock here: unlock.id and unlock.time.
      },
    });

`onUnlock` is called once for each unlock made in this run, before its `unlocked` event. It is not called for the
unlocks that a load restores. A bridge that must sync at start can read `list()` after `load()` and report every
unlocked entry.

## Dev builds

With `dev: true`, `unlock(id, { force: true })` unlocks any achievement, and `resetAll()` clears the counters and
the unlocks and saves the empty progress. Elsewhere both are refused with `dev-only`.

## Tests

From `js/`, run `pnpm exec vitest run packages/achievements`.

- `fixtures.test.ts` runs the shared suite: the normal form of the valid file, the refused documents with their codes
  and paths, and the scripted cases, step by step, with the exact events of each step.
- `runtime.test.ts` covers the cascade order, the listeners and the clock, the gallery order, the options and the
  progress of each goal.
- `persistence.test.ts` covers the saved round trip, the order of writes, a refusing storage, the merge of a load,
  the debounce (with fake timers), the three places the progress can go, and the dev reset.

`test-support.ts` holds the in-memory storage, the manual clock and the fixture reader. Only the tests import it.
