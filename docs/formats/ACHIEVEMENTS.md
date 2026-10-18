# Achievements v1 (`velvet-achievements`): counters, goals, unlocks and saved progress

velvet-achievements describes the achievements of a game: the counters that the game advances, the goal
that each achievement needs, the progress notices on the way to it, and the progress that is saved
between runs. It is data-driven and carries no genre and no platform. A game decides what its counters
count; a platform bridge (Steam or another) receives the unlocks through the runtime's `onUnlock` hook.

This document is normative for version 1. The reference implementation is `@velvet/achievements`
(`js/packages/achievements`). The conformance fixtures are in `docs/formats/fixtures/achievements/`.
Where this text and the code disagree, the fixtures decide.

## Conventions

- JSON is UTF-8. Field names are `snake_case`. Readers ignore fields that the format does not define.
- An **id** is a lowercase ASCII letter, then up to 63 lowercase letters, digits or `_`:
  `^[a-z][a-z0-9_]{0,63}$`. Ids name counters, achievements and groups. Counters and achievements have
  separate id spaces, so a counter and an achievement may share an id.
- A **key** is an i18n key (`LOCALE.md`): text of 1 to 200 characters, counted in UTF-16 code units. Names,
  descriptions and hints are keys. The format never reads their text.
- An **art id** names an entry of the art manifest (`ART_MANIFEST.md`). It has 1 to 128 characters and
  matches `^[a-z0-9_-]+(\.[a-z0-9_-]+)*$`. The format checks only this shape.
- A **whole number** is an integer. A **safe whole number** is a whole number from 0 to 9007199254740991
  (2^53 - 1), the largest integer that a double holds exactly.
- A **time** is a finite number of milliseconds, taken from the clock that the game injects (see
  [Clock](#clock)).

## Documents

Every document has this envelope:

| Field | Value |
|---|---|
| `format` | `"velvet-achievements"` |
| `version` | `1` |
| `kind` | `"definitions"` or `"progress"` |

A reader refuses a document whose `format`, `kind` or `version` it does not read (`format`, `version`).
A definitions document is the file that a game ships. A progress document is what a runtime saves (see
[Saved progress](#saved-progress)).

## Definitions (`kind: "definitions"`)

| Field | Type | Default | Meaning |
|---|---|---|---|
| `counters` | array of counters | `[]` | The counters that the game advances. |
| `achievements` | array of achievements | `[]` | The achievements. Their order in the file breaks the ties of [Lists](#lists). |

### Counters

| Field | Type | Default | Meaning |
|---|---|---|---|
| `id` | id | required | Unique among the counters. |
| `max` | safe whole number from 1 | none | The highest value. A counter stops at its maximum. |
| `persist` | boolean | required | `true`: the value is saved between runs. `false`: the value starts at 0 in each run. |

Example (from `fixtures/achievements/achievements.json`):

```json
{ "id": "kills", "max": 100, "persist": true }
```

### Achievements

| Field | Type | Default | Meaning |
|---|---|---|---|
| `id` | id | required | Unique among the achievements. |
| `name` | key | required | The name. Shown while the achievement is visible. |
| `description` | key | required | The description. Shown while the achievement is visible. |
| `icon` | art id | required | The picture of the achievement. |
| `hidden` | boolean | `false` | A hidden achievement is masked until it is unlocked (see [Hidden achievements](#hidden-achievements)). |
| `hint` | key | none | Text for a hidden achievement that is still locked. Allowed only with `hidden: true`. |
| `points` | whole number from 0 to 1000000 | `0` | The points that the achievement is worth. |
| `group` | id | `"default"` | The gallery group. |
| `order` | whole number from -1000000 to 1000000 | `0` | The place inside the group. |
| `progress_steps` | list of numbers | `[]` | Fractions of the goal, each above 0 and below 1, strictly increasing, for example `[0.25, 0.5, 0.75]`. A progress event is sent for each step reached (see [Progress notices](#progress-notices)). Allowed only with a counter, `all` or `any` goal, and only when `hidden` is not `true`. |
| `goal` | goal | required | What unlocks the achievement (see [Goals](#goals)). |

Example (from `fixtures/achievements/achievements.json`):

```json
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
```

### Goals

A goal has exactly one of four kinds. A goal with none of them, or with more than one, is refused.

| Kind | Form | Unlocks when |
|---|---|---|
| counter | `{ "counter": id, "at_least": n }` | The counter holds `n` or more. `n` is a whole number from 1 that is not above the counter's `max`. The counter must be declared. |
| all | `{ "all": [ids] }` | Every listed achievement is unlocked. The list has 1 or more distinct ids. |
| any | `{ "any": [ids], "count": n }` | At least `n` of the listed achievements are unlocked. The list has 1 or more distinct ids. `n` is a whole number from 1 up to the length of the list. |
| manual | `{ "manual": true }` | The game unlocks it with a call. No check of the goals unlocks it. |

The `all` and `any` goals refer to other achievements of the file. A reference that names no achievement
is refused (`unknown-achievement`). The references must form no loop, a reference to itself included
(`cycle`).

### Lists

Entries keep the order of the file. The gallery order of the achievements is given by `group` and `order`:
the groups come in the order in which their first achievement appears in the file; inside a group, the
achievements come by `order`, and then by their place in the file.

In the normal form (what a reader gives back), the list of an `all` or `any` goal is named `of`, in the
order of the file.

## Validation

A definitions document is refused as a whole. A strict reader reports every problem it finds, each with
its path (for example `achievements[2].goal.any[1]`), and the first problem gives the error code.

| Code | When |
|---|---|
| `json` | The text is not JSON. |
| `format` | The format tag is not `velvet-achievements`, or the kind is not `definitions`. The path is `format` or `kind`. |
| `version` | The version is not 1. |
| `structure` | A field is missing or has the wrong JSON type: a list that is not an array, a counter or an achievement that is not an object, a missing `persist`, `name`, `description`, `icon` or `goal`, a flag that is not a boolean, and so on. |
| `bad-id` | An id, a group or an art id does not match its pattern. |
| `value` | A value breaks its range or rule: a key outside 1 to 200 characters, a number that is not whole or is outside its range, a `max` below 1, an `at_least` below 1 or above the counter's `max`, a `count` outside 1 to the length of its list, an empty `all` or `any` list, a `manual` that is not `true`, a goal with more than one kind, a `hint` without `hidden: true`, `progress_steps` that are not strictly increasing or lie outside 0 to 1, or `progress_steps` on a manual goal or a hidden achievement. |
| `duplicate` | Two counters, two achievements, or two entries of one `all` or `any` list share an id. |
| `unknown-counter` | A counter goal names no counter of the file. |
| `unknown-achievement` | An `all` or `any` list names no achievement of the file. |
| `cycle` | The `all` and `any` goals form a loop. The path is the reference that closes it. |

## Runtime rules

These rules say what a running game does. They are normative: every runtime gives the same answers and
the same events, in the same order.

### Counters

- `increment(counter, by)` adds `by`, a safe whole number. The default is 1. A negative `by` is refused
  (`argument`).
- `setCounter(counter, value, { allowDecrease })` sets the value, a safe whole number. A value below the
  current one changes nothing, unless `allowDecrease` is true. A decrease never takes back an unlock.
- A value is clamped to the counter's `max`, when it has one.
- A call that leaves the value as it was (a zero step, or a value already at the maximum) changes nothing:
  no event and no save.
- A counter that the definitions do not declare is refused (`unknown-counter`). A refused call changes
  nothing.

### Goals and cascades

A counter goal is met when its counter holds `at_least`. An `all` goal is met when every listed
achievement is unlocked. An `any` goal is met when at least `count` of its listed achievements are
unlocked. A manual goal is never met by a check: only `unlock` unlocks it.

After each change, the runtime checks the goals as follows. The order that this produces fixes the events.

1. The **candidates** are: after a counter change, the achievements with a counter goal on that counter; after
   an unlock (by a call or by a cascade), the achievements whose `all` or `any` goal lists the unlocked one;
   after a load, every achievement that is not manual. Each group of candidates is in file order.
2. The runtime works through a queue that starts with the candidates. A candidate that is still locked, is not
   manual, and whose goal is met is unlocked. The achievements that list it in an `all` or `any` goal (in
   file order) are then added to the end of the queue.
3. The queue runs until it is empty. Each unlock is recorded in the order in which it happens.

A candidate that is already unlocked, or is manual, is skipped. Because an unlock is never taken back and
the references form no loop, the queue always ends.

### Unlocking

- `unlock(id)` unlocks a manual achievement. It returns `true` when this call unlocked it, and `false` when
  it was already unlocked (then nothing changes).
- `unlock(id)` on an achievement whose goal is not manual is refused (`not-manual`).
- `unlock(id, { force: true })` unlocks any achievement, but only in a dev build (the `dev` option). Elsewhere
  it is refused (`dev-only`).
- An unlock stays. No change of a counter takes it back.
- Every unlock made by one call has the time of the clock at that call. The unlocks of a cascade (see above)
  have the same time.

### Hidden achievements

A hidden achievement that is still locked is **masked**. `list()` gives it `name`, `description`, `icon` and
`progress` as `null`. It gives its `hint` (or `null` when it has none). Its progress events are not sent.
`progress(id)` still gives the numbers, and the game decides what to show with them.

### Progress notices

- A locked achievement that is not hidden, and has `progress_steps`, sends a `progress` event for each step
  that its fraction reaches (see [Progress](#progress)). The events come in step order.
- A jump that crosses several steps sends one event for each step crossed, in order.
- A step is sent once. After a decrease and an increase back to the step, it is not sent again.
- An unlocked achievement sends no progress events.
- The progress events of one call come after the unlock events of that call.

### Progress

`progress(id)` gives `{ current, target, fraction }`, where `fraction` is `current / target`.

| Goal | `target` | `current` |
|---|---|---|
| counter | `at_least` | the counter's value, at most `at_least` |
| all | the number of listed achievements | the number of them that are unlocked |
| any | `count` | the number of listed achievements that are unlocked, at most `count` |
| manual | 1 | 0 while locked |

An unlocked achievement gives its `target` as `current`, and a `fraction` of 1.

### Events and their order

Events are sent after the state of the call is complete: every counter change and every unlock of the call
is in place when the first listener runs. The events of one call come in this order:

1. `unlocked`, once for each unlock, in the order of the cascade. Before each `unlocked`, the `onUnlock` hook
   is called with the same payload.
2. `progress`, for each notice, in file order and in step order.
3. `changed`, once, when the counters or the unlocks changed.

| Event | Payload | Sent when |
|---|---|---|
| `unlocked` | `{ id, time }` | An achievement unlocks, by a call or by a load. |
| `progress` | `{ id, step }` | A locked achievement reaches a progress step. |
| `changed` | none | The counters or the unlocks change: by a call, by a load that changed something, or by a reset. |

A listener that throws does not stop the other events. Every event is sent, and the first error is thrown to
the caller at the end of the call.

### Clock

Unlock times come from the injected clock. The clock returns a finite number of milliseconds. Its default
is `Date.now`. A clock that returns anything else refuses the call (`argument`). A test passes its own
clock, so that replays give the same times.

### Lists

- `list({ includeHidden })` gives one entry for each achievement, in the order of [Lists](#lists).
  `includeHidden` defaults to `true`: a masked achievement is then listed. With `false`, masked achievements
  are left out.
- An entry has these fields, named as in the conformance cases: `id`, `group`, `order`, `points`, `hidden`,
  `unlocked`, `unlocked_at` (the time, or `null`), `masked`, `name`, `description`, `icon`, `hint` and
  `progress`. The text fields, `icon` and `progress` are `null` while the entry is masked.
- The totals are the number of unlocked achievements, the points of the unlocked achievements, the number
  of achievements and the points of all of them. The points of all of them include the hidden ones.

## Saved progress

A progress document is what a runtime saves:

```json
{
  "format": "velvet-achievements",
  "version": 1,
  "kind": "progress",
  "counters": { "kills": 40, "gold": 120 },
  "unlocked": { "first_blood": 1739836800000 }
}
```

| Field | Type | Meaning |
|---|---|---|
| `counters` | object | The value of every counter with `persist: true`, by id, in the order of the definitions. |
| `unlocked` | object | The unlock time of every unlocked achievement, by id, in the order of the definitions. |

A writer writes every persisted counter (even at 0) and every unlock. A reader does not depend on the order
of the keys.

### Reading

- Nothing stored: the status is `fresh`, and nothing changes.
- The save is **damaged** when it is not an object, when its `format` or `kind` is wrong, when its `version`
  is not 1 (a newer version is never guessed at), when `counters` or `unlocked` is missing or is not an
  object, when a counter value is not a number (`structure`) or is not a safe whole number (`value`), or when
  an unlock time is not a finite number (`structure`). A damaged save is used whole or not at all: nothing
  from it is applied, the status is `damaged`, and the issues list each problem with its path.
- Counters and achievements that the definitions do not declare, and counters with `persist: false`, are
  dropped. They never refuse a save.
- A value above a counter's `max` is clamped, not refused.

### Merging on load

A loaded save joins the progress that the run already has, so that a call made before the load keeps its
effect:

- a counter takes the larger of its two values, clamped to its maximum;
- an unlock from the save is added. When the achievement is already unlocked, the earlier of the two times is
  kept;
- then the goals are checked again, as in [Goals and cascades](#goals-and-cascades), with every achievement
  that is not manual as the candidates. A goal that the saved counters meet, but whose unlock was not saved
  (for example, because the game stopped before the save was written), unlocks now, with the time of the
  load. Its unlock events are sent;
- the progress steps that the loaded values already reached are not sent again.

### Saving

- A change schedules a save after the debounce, which is 250 ms by default. Each change restarts the wait, so
  a burst of changes is one write.
- `save()` writes now. It waits for a pending load first, so that it never replaces saved values that were not
  read yet. It cancels the pending save. It resolves `true` when the storage took the document, and `false`
  when it refused.
- Writes happen in the order of the calls. The last state that is set is the last state that is written.

### Load results

`load()` resolves to one of these statuses. The store reads the save once.

| Status | Meaning |
|---|---|
| `fresh` | Nothing was stored. |
| `loaded` | The saved progress was read and merged. |
| `damaged` | The saved progress could not be used. It is reported once, with its issues. The store keeps the progress it has, and the next save replaces the stored value. |
| `unavailable` | The storage refused the read. The store keeps the progress it has. |

## Errors (runtime)

| Code | When |
|---|---|
| `unknown-counter` | A counter call names a counter that the definitions do not declare. |
| `unknown-achievement` | A call names an achievement that the definitions do not declare. |
| `not-manual` | `unlock` names an achievement whose goal is not manual, without `force`. |
| `dev-only` | `force` or `resetAll` is used outside a dev build. |
| `argument` | A value is not a safe whole number, a delta is negative, or the clock gives a time that is not finite. |

## Versioning

This is version 1. A reader refuses any other version. New optional fields may be added within version 1,
because readers ignore fields they do not know. A change to what a document means needs version 2. Saves of
version 1 are then migrated one version at a time, as the other formats do.

## Conformance fixtures

`docs/formats/fixtures/achievements/` holds the suite. Every reader runs all of it.

| File | What it holds |
|---|---|
| `achievements.json` | A valid definitions document: three counters, nine achievements with each kind of goal, masked ones, progress steps, groups, a tie in `order`, and an achievement with no group. |
| `cases.json` | The file it runs against (`definitions`), the `normal` form of some entries, the `invalid` list, and the `cases`. |
| `invalid/*.json` | Definitions that must be refused, one for each rule above. The code and path of each are in `cases.json`. |

`cases.json` lists:

- `normal`: for some counters and achievements, the fields that the normal form must have. A field that is
  not listed is not checked.
- `invalid`: a list of `{ file, code, path }`. The first problem has the code, and one of the problems has the
  path. Each file has one problem only.
- `cases`: scripted runs. Each case has a `name`, an optional `dev` (default `false`), and `steps`. A reader
  runs the steps in order, on a new store, with a clock that starts at 1000 ms and an empty storage.

| `do` | Fields | Meaning |
|---|---|---|
| `tick` | `now` | Sets the clock. |
| `increment` | `counter`, `by` (default 1) | `increment`. |
| `set_counter` | `counter`, `value`, `allow_decrease` (default false) | `setCounter`. |
| `unlock` | `id`, `force` (default false) | `unlock`. |
| `reset_all` | none | `resetAll`. |
| `load` | `saved` (optional) | Stores `saved` in the storage when it is given, then calls `load()`. |
| `restart` | none | Makes a new store on the same storage and loads it. |
| `save` | none | `save()`. |
| `wait` | `ms` | Runs the timers for `ms` milliseconds, and the writes that they start. |
| `check` | see below | Reads the state. |

Any step may give:

- `events`: the exact list of events that the step sends, in order. The forms are `{ "type": "unlocked", "id",
  "time" }`, `{ "type": "progress", "id", "step" }` and `{ "type": "changed" }`. A step without `events` is
  not checked for events. The `unlocked` events of a step must match the `onUnlock` calls of the same step.
- `error`: the code that the call must throw. Such a step sends no events and changes nothing.
- `returns`: the value that the call returns (a number for `increment` and `setCounter`, a boolean for
  `unlock` and `save`).

`load` and `restart` may give `status` and `codes` (the codes of the issues, in order).

A `check` may give any of these:

- `counters`: the value of each listed counter.
- `unlocked`: the exact map from the id of every unlocked achievement to its unlock time.
- `progress`: `{ current, target, fraction }` for each listed achievement.
- `points`, `unlocked_count`, `total_count` and `total_points`.
- `list`: the whole list, in order, with the entries given. Each entry gives the fields that it checks; the list
  must have as many entries as the array. `include_hidden` (default `true`) is the option of the call.
- `writes`: how many writes the storage has taken so far.
- `stored`: the document that the storage holds, or `null` when it holds nothing.

## Outside the format

The format does not define:

- what the counters count, and when a game advances them;
- the texts of the keys, and the pictures of the art ids: the locale catalogs and the art manifest give them;
- how a platform receives an unlock: a bridge uses the `onUnlock` hook of the runtime, and may read `list()` to
  synchronise at start (see the package README);
- where the progress is kept: the runtime uses the storage that it is given, or the one that the host provides;
- the notices, the gallery layout and the sounds that an unlock makes.
