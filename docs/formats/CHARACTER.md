# Characters v1: stats, needs, professions, relationships and traits

velvet-character is the format of the characters of a game: the stats they hold, the needs that
change with time, the professions they learn, the affinity between them, their traits, and the state
that a save keeps of each one. It is data-driven and carries no genre. A management game, a card game
and a visual novel can all use it: the game decides which stats, needs and professions exist, and this
format says how they behave.

This document is normative for version 1. The reference implementation is `@velvet/characters`
(`js/packages/characters`). The conformance fixtures are in `docs/formats/fixtures/characters/`. Where
this text and the code disagree, the fixtures decide.

## Conventions

- JSON is UTF-8. Field names with more than one word are `snake_case` (`xp_thresholds`, `history_limit`).
- Readers ignore fields that the format does not define, in every document and in every section. The
  exception is the names inside `stats`, `needs`, `professions` and `traits` of a character document, and
  the target of a trait modifier: a name that the schema does not have is an error, because it is most
  likely a typo.
- An **id** is a lowercase ASCII letter, then up to 63 lowercase letters, digits or `_`:
  `^[a-z][a-z0-9_]{0,63}$`. Ids name stats, needs, thresholds, levels, professions, traits, tags and
  characters. The names `min`, `max`, `clamp` and `floor` are not stat ids, because formulas use them as
  function names.
- A **key** is an i18n key: text of 1 to 200 characters. Labels, names, titles and reasons are keys. The
  engine never reads their text; the game looks them up in its locale catalogs (`LOCALE.md`).
- An **art id** is written as the art manifest writes its ids (`ART_MANIFEST.md`): 1 to 128 characters,
  segments of `a`-`z`, `0`-`9`, `_` and `-` joined by single dots, as in `^[a-z0-9_-]+(\.[a-z0-9_-]+)*$`.
  It names an entry of the art manifest, which says what the id means. This format checks only the shape.
- Numbers are finite IEEE 754 double-precision values. A number that reads as infinite is an error.
- A reader applies arithmetic in the order that this document gives, so that its values agree with the
  fixtures. The fixtures use values that double precision holds exactly, and compare to 9 decimal places.

## Documents

Every document has the same envelope:

| Field | Value |
|---|---|
| `format` | `"velvet-character"` |
| `version` | `1` |
| `kind` | `"schema"`, `"character"` or `"state"` |

A reader refuses a document whose `format`, `version` or `kind` it does not read
(`unsupported-document`). Schema and character documents must have version 1 exactly. Authoring tools
migrate those documents, not the readers. A state is migrated by the reader (see [Save state](#save-state)).

## Schema document (`kind: "schema"`)

The schema holds the definitions that all the characters of a game share: stats, needs, professions,
the rules of affinity, and traits. Every list is optional, and empty when it is left out.

### Stats

A stat is **base** when it has a `default`: the character holds its value, and events and traits change
it. A stat is **derived** when it has a `formula`: its value comes from a formula over other stats.

| Field | Type | Meaning |
|---|---|---|
| `id` | id | Unique among the stats. |
| `label` | key | Required. |
| `min`, `max` | number | Required, with `min` at most `max`. The effective value stays in this range. |
| `default` | number | Required on a base stat, within `min` to `max`. Not allowed on a derived stat. |
| `formula` | text | Required on a derived stat, which must not be blank. Not allowed on a base stat. |

Stats keep the order in which the schema lists them. Every formula is checked when the schema is read:
its syntax, its function names, the number of arguments of each call, the stats it names (they must
exist), and whether the derived stats depend on each other in a loop.

### Formulas

A formula is an expression of numbers and stat ids. It has no strings, comparisons, assignments,
property access or calls to anything but the four functions below.

```text
sum       := product (("+" | "-") product)*
product   := unary (("*" | "/") unary)*
unary     := "-" unary | atom
atom      := number | name | name "(" arguments ")" | "(" sum ")"
arguments := sum ("," sum)*
```

- A number is digits with an optional fraction: `12`, `0.5`. A name is a stat id.
- The operators `+ - * /` are left-associative. `*` and `/` bind tighter than `+` and `-`.
- Functions: `min(a, b, ...)` and `max(a, b, ...)` take one or more arguments; `clamp(x, low, high)` is
  `min(max(x, low), high)` and takes three; `floor(x)` takes one.
- A formula reads the **effective** values of the stats it names (see [Stats](#stat-values)).

Errors: a syntax error is `expression`; a call to a name that is not a function is `unknown-function`; a
call with the wrong number of arguments is `arity`; a name that is not a stat is `unknown-stat`; a
divisor that is a literal zero is `division-by-zero` when the formula is read. A divisor that is zero
when the formula is evaluated is also `division-by-zero`; a result that is infinite or not a number is
`non-finite`. A runtime may run *lenient* for evaluation-time division by zero, so one bad divisor
cannot break the frame that reads the stats: the stat then takes its minimum and the error is reported
once per stat. Tools and conformance tests run strict.

### Needs

A need is a value that changes with game time (hunger, energy, mood).

| Field | Type | Meaning |
|---|---|---|
| `id` | id | Unique among the needs. |
| `label` | key | Required. |
| `min`, `max` | number | Required, with `min` at most `max`. |
| `default` | number | Required, within `min` to `max`. The value of a new character, unless its sheet gives one. |
| `rate` | number | The change per game second. Default `0`: the need changes only by events. |
| `thresholds` | list | Default `[]`. See below. |

A threshold is `{ "id", "at", "when" }`. Its `id` is unique within its need and names its event. Its
`at` lies within the need's range. Its `when` is `below` or `above` (see [Thresholds](#thresholds)).

### Professions

A profession is a skill with a level, such as farming. Its experience curve is set by the game.

| Field | Type | Meaning |
|---|---|---|
| `id` | id | Unique among the professions. |
| `label` | key | Required. |
| `xp_thresholds` | list of numbers | Required, not empty. The total experience at which each level starts. The first is `0` (level 1), and each one is more than the one before. The list's length is the number of levels. |

### Relationships

The `relationships` object sets the rules of affinity between characters. It is optional; without it,
the values below apply.

| Field | Type | Default | Meaning |
|---|---|---|---|
| `min` | number | `-100` | The lowest affinity. |
| `max` | number | `100` | The highest affinity. |
| `default` | number | `0` | The affinity of a pair that has no value. Within `min` to `max`. |
| `history_limit` | whole number, 1 to 1000 | `50` | How many changes a character keeps in its history. |
| `levels` | list | `[]` | Named levels of affinity, see below. |

Each level is `{ "id", "label", "at" }`. Levels are optional. When there are any, the first one starts at
`min`, each `at` is higher than the one before, every `at` is within the range, and every `id` is unique.
A level covers the affinities from its `at` up to the next level's `at`.

### Traits

A trait is a named set of modifiers that a character may have:

```json
{ "id": "brave", "label": "trait.brave", "modifiers": [{ "stat": "strength", "mul": 1.25 }] }
```

Each modifier names exactly one target, a stat (`"stat": id`) or the rate of a need (`"rate": id`), and
exactly one operation, `"add": number` or `"mul": number`. A `mul` is zero or more. Its effect is given
in [Runtime rules](#runtime-rules).

## Character document (`kind: "character"`)

A character document is the static definition of one character: who it is, and where its values start.
It is read against one schema.

| Field | Type | Meaning |
|---|---|---|
| `id` | id | Required. Unique among the characters of a roster. |
| `name` | key | Required. |
| `title` | key | Optional. |
| `art` | object | Optional. Its `portrait` and `fullbody` fields are art ids. |
| `tags` | list of ids | Default `[]`. Each tag once. A roster finds characters by tag. |
| `stats` | object | Starting base values, by stat id. A stat that is left out starts at its `default`. |
| `needs` | object | Starting values, by need id. A need that is left out starts at its `default`. |
| `professions` | object | Starting experience, by profession id. A profession that is left out starts at 0. |
| `traits` | list of ids | The traits the character starts with, each once. |
| `relationships` | object | Starting affinity toward other characters, by their id. A pair that is left out starts at the default. |

The rules of the sheet:

- A stat in `stats` must be a base stat (`derived-stat` otherwise), and every value is within its range
  (`out-of-range`).
- A value in `needs` is within the need's range. An experience value in `professions` is from 0 to the
  last threshold of the profession's curve.
- A trait in `traits` must be in the schema (`unknown-trait`).
- An affinity in `relationships` names another character: its id is valid and is not the sheet's own id.

A reader fills in every starting value that the document leaves out, so the normal form has every base
stat, every need and every profession.

## Save state

A save keeps the **runtime** values of a character, not its definition. The rules of the schema and the
sheet are not in a save; a save is restored into a character that is made from its sheet.

| Field | Type | Meaning |
|---|---|---|
| `id` | id | The character. Must be the id of the character that the state is restored into. |
| `stats` | object | The current base value of every base stat. Derived stats are not saved. |
| `needs` | object | The current value of every need. |
| `xp` | object | The current experience of every profession. |
| `traits` | list of ids | The traits the character has now, sorted by id. |
| `affinity` | object | The affinity toward each other character that holds a value, sorted by id. Pairs at the default may be left out. |
| `history` | list | The affinity changes, oldest first. Each entry is `{ "other", "delta", "value", "reason" }`. |

A writer lists the stats, needs and professions in schema order. A reader does not depend on the order of
keys.

Reading a state:

- The envelope is checked. Another format or kind, or a version newer than this reader's, is
  `unsupported-document`. A version that is not a whole number from 1 is `invalid`.
- The `id` must be the id of the character. All six sections must be present, and be of the right type.
- A stat, need or profession that the state leaves out takes its starting value (from the sheet).
- Every number is clamped to its range: base stats, needs, experience (to 0 up to the last threshold),
  affinity, and the `value` of a history entry.
- A trait, stat, need or profession that the schema no longer has is dropped; the schema changed.
- An affinity key must be an id, and not the character's own id. A history entry's `other` must be an id,
  its `delta` and `value` must be numbers, and its `reason` must be a key.
- The history keeps only its newest `history_limit` entries.

A state that fails a check is refused whole: the character keeps its values.

### Migrations

The state carries the version of the format that wrote it. A reader brings an older state up to date one
version at a time: a state of version *n* goes through the migration from *n* to *n* + 1, and so on up to
the current version. Version 1 is the first version, so there are no migrations yet. A version 2 adds the
migration from 1, and old saves keep loading. A state of a newer version is refused, never guessed at.

## Runtime rules

These rules say what a character does when the game changes it. They apply to the values of a character
that the state holds.

### Stat values

The effective value of a stat is computed in one way for base and derived stats:

1. **raw**: the base value of a base stat, or the value of its formula for a derived stat, where the
   formula reads the effective values of the stats it names;
2. **add**: the sum of the `add` modifiers, of the traits the character has, that target this stat, in the
   order the schema lists the traits;
3. **mul**: the product of the `mul` modifiers of the same traits, in the same order (the empty product
   is 1);
4. the effective value is `clamp((raw + add) * mul, min, max)`.

`changeBase(stat, delta)` adds `delta` to a base stat and clamps the result to its range. A derived stat
cannot be changed (`derived-stat`).

### Rates

The effective rate of a need is `(rate + add) * mul`, using the modifiers that the traits make on the
need's rate in the same way as for stats. A rate is not clamped.

### Needs

`tickNeeds(dt)` advances game time by `dt` game seconds. `dt` is a finite number, zero or more
(`argument` otherwise). The caller converts its clock to game seconds. For each need, in schema order,
whose effective rate is not zero: `value = clamp(value + rate * dt, min, max)`.

`changeNeed(need, delta)` sets `value = clamp(value + delta, min, max)`.

### Thresholds

A threshold with line `at` is crossed by a move of a need from `before` to `after`:

- when `below`: `before >= at` and `after < at`;
- when `above`: `before < at` and `after >= at`.

A single move crosses a threshold at most once, and the events report only the crossings of that move.
The events come in schema order of the needs, then in the order of the thresholds. A value that stands
exactly on a line is not below it: it is at or above it.

### Professions

The level of a profession is the number of its thresholds that the experience reaches: a new character is
at level 1.

`gainXp(profession, amount)` takes a finite `amount`, zero or more (`argument` otherwise). The experience
becomes `min(xp + amount, last threshold)`: experience stops at the top of the curve, and the excess is
lost. It reports one `level_up` for each level reached, in order.

### Affinity

The affinity of one character toward another is the stored value, or the default when the pair holds
none. A character has no affinity toward itself (`argument`).

`changeAffinity(other, delta, reason)` takes a finite `delta` and a key `reason`. Its steps:

1. `after = clamp(before + delta, min, max)`, and `applied = after - before`.
2. When `applied` is zero, nothing changes: no event, no history entry.
3. Otherwise the new value is stored, and the entry `{ other, delta: applied, value: after, reason }`
   is appended to the history. The oldest entries go when the history is longer than `history_limit`.
4. The level of a value is the highest level whose `at` is at or below it. A value on a line is on the
   level of that line. When the level changed, the event `affinity_level` is reported, with `direction`
   `up` when the new level is higher.

A roster answers queries about affinity. Its lists are sorted by id, and a minimum level keeps the
affinities at that level or higher.

### Traits

Adding a trait that the character has, or removing one it does not have, changes nothing. A trait that
the schema does not have is `unknown-trait`.

### Membership of a roster

A roster is a set of characters by id, and all of them follow one schema. Its constructor joins each
character of its list, and after that the membership changes only through two calls:

- `join(character)` adds a character. The id must not be in the roster (`duplicate-id`), and the schema
  must be the roster's (`argument`). A roster that is empty takes the schema of its first character, and
  keeps it when its members leave.
- `leave(id)` removes a member and returns it (`unknown-character` when there is none).

The state of a roster is the list of the states of its members, sorted by id, so a snapshot holds exactly
the members at the time it was taken. A restore follows these rules:

- Each state applies to the character with its id: a member, or a character that left the roster. A state
  of an id that is neither is `unknown-character`.
- A state of a character that left brings that character back into the roster, with the values of the
  state. A save taken while a character was a member therefore loads with that character in the roster.
- A restore never removes a member. A character that has no state in the list keeps its values and its
  place, so a character that joined after the save stays until it leaves.
- Each state is read as a restore of one character is (see [Save state](#save-state)). Either every state
  applies, or none does: the characters keep their values, and the roster keeps its members.

A roster keeps the characters that left, for as long as it lives, so that a restore can bring them back.
Version 1 has no call that forgets one.

## Events

Events are the reports of the runtime. Each one is tagged by `event`, and names its `character`.

| Event | Fields | Raised by |
|---|---|---|
| `threshold_crossed` | `need`, `threshold`, `when`, `value` (the value after the move) | `tickNeeds`, `changeNeed` |
| `level_up` | `profession`, `level` (the level reached) | `gainXp` |
| `affinity_level` | `other`, `from`, `to` (level ids), `direction` (`up` or `down`) | `changeAffinity` |

## Errors

A refused call or document fails with a code that names the rule that broke. The codes are the same in
every reader.

| Code | When |
|---|---|
| `json` | The text is not JSON. |
| `unsupported-document` | The format, kind or version (of a document, or of a newer state) is not one this reader reads. |
| `invalid` | A field is missing, has the wrong type or value, or breaks a rule that has no code of its own. |
| `bad-id` | An id or an art id does not match its pattern, or a stat id is a reserved name. |
| `duplicate-id` | Two entries of one list share an id. |
| `expression` | A formula is not an expression. The message gives the position. |
| `unknown-function` | A formula calls a name that is not a function. |
| `arity` | A function is called with the wrong number of arguments. |
| `unknown-stat` | A stat id, or a variable of a formula, names no stat. |
| `unknown-need` | A need id, or a trait's rate, names no need. |
| `unknown-profession` | A profession id names no profession. |
| `unknown-trait` | A trait id names no trait. |
| `unknown-level` | A minimum level names no level of the schema. |
| `unknown-character` | A roster has no character with the id. |
| `derived-stat` | A value is set on a derived stat. |
| `out-of-range` | A starting value of a sheet is outside its range. |
| `cycle` | Derived stats depend on each other in a loop. |
| `division-by-zero` | A formula divides by zero: a literal zero when it is read, any other zero when it is evaluated. |
| `non-finite` | A formula gives a value that is infinite or not a number. |
| `argument` | A call gives a value that it does not accept: a time step or a delta that is not finite, a negative experience, a character's own id as the other side of an affinity, an empty or long reason, or a character of another schema joining a roster. |

## Versioning

This is version 1. A reader refuses any other version. New optional fields may be added within version 1,
because readers ignore fields they do not know. A change to what a document means needs version 2: for
example, a different formula language, a different order of arithmetic, or a different rule for crossing
a threshold.

## Conformance fixtures

`docs/formats/fixtures/characters/` holds the conformance suite. Every reader runs all of it.

| File | What it holds |
|---|---|
| `schema.json` | The schema of the fixtures: five stats (three base, two derived), three needs with thresholds, three professions, the affinity levels, and three traits. |
| `sheets/mara.json`, `sheets/bram.json`, `sheets/ines.json` | Three characters of a village (a farmer, a guard and a scholar), with starting values, a trait and affinities. |
| `cases/01_farm_day.json` | Ticks, eating and a fall in mood: thresholds below and above, and the order of events. |
| `cases/02_threshold_edges.json` | A value that stands exactly on a `below` line is not yet under it; one that reaches an `above` line crosses it. Values clamp at both ends. A zero step changes nothing. |
| `cases/03_farming_xp.json` | Experience across several levels, the cap at the top of the curve, and the refusals. |
| `cases/04_affinity.json` | Affinity across levels, clamping, the history limit, and a change that moves nothing. |
| `cases/05_traits.json` | Traits on stats and rates, adding and removing them, and base values against effective values. |
| `cases/06_save_round_trip.json` | A reload from a state, checked against the state before it. |
| `cases/07_restore.json` | A restored state: clamping, dropped names, starting values for what is missing, and the refusals. |
| `cases/08_roster.json` | Queries by tag, affinities and opinions, a minimum level, and the refusals of the queries. |
| `expressions.json` | The formula language: precedence, functions, values, and each error code. |
| `invalid.json` | Documents that must be refused, one for each rule of the schema, the sheet and the state. |

A case file is a document with `kind: "case"`. It names its `characters` (sheets, by id), and has `steps`
that a reader runs in order. A step has a `do`, the character it acts on, and the fields of that step:

| `do` | Fields | Meaning |
|---|---|---|
| `tick` | `dt` | `tickNeeds(dt)`. |
| `change_need` | `need`, `delta` | `changeNeed`. |
| `gain_xp` | `profession`, `amount` | `gainXp`. |
| `change_affinity` | `other`, `delta`, `reason` | `changeAffinity`. |
| `change_base` | `stat`, `delta` | `changeBase`. |
| `add_trait`, `remove_trait` | `trait` | Gives or takes a trait. |
| `restore` | `state` | `restore` with the state given inline. |
| `reload` | none | Saves the character as JSON, makes a new character from its sheet, restores the state into it, and requires that the new state equals the old one. |
| `check` | any of `base`, `stats`, `rates`, `needs`, `xp`, `levels`, `affinity`, `affinity_levels`, `traits`, `history` | The values that the character has now. |

Any step may give `events`, which are the events it must raise, in order. A step with `events: []` must
raise none. A step may give `error`, which is the code that the step must fail with; it then changes
nothing. A case may end with `queries`, which are checked against a roster of the case's characters:
`by_tag` (`tag`, `expect` as the ids, sorted), `relations` and `opinions` (`of`, an optional `min_level`,
and `expect` as the list of entries). A query may give `error` instead of `expect`.

`expressions.json` gives each formula with its variables (`vars`) and either its `value` or its `error`.
`invalid.json` gives each refused document, either as `text` or as a `document`, with `as` (`schema`, or
`character` read against `schema.json`) and its `error`.

## Outside the format

The format does not define:

- what the game shows: names, titles, labels and reasons are keys, and the locale catalogs give their text;
- which pictures belong to a character: the art manifest (`docs/formats/ART_MANIFEST.md`) maps art ids to files;
- what a stat, need or profession means in the game, or how a game uses the events;
- the game's clock: the caller converts its time to game seconds;
- the container of a save: the save store wraps a state in its own envelope;
- the storage of a roster, the order of its characters on screen, and any text formatting of numbers.
