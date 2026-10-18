# @velvet/characters

The characters of a game, in the character sheet format (`docs/formats/CHARACTER.md`): the stats a
character holds, the needs that change with game time, the professions it learns, the affinity between
characters, its traits, and the state that a save keeps. The format carries no genre. A management game,
a card game or a visual novel can all use it, because the game writes the schema: the stats, needs,
professions, levels and traits it wants.

It has four parts:

- `parseStatSchema(json)` reads the schema (the definitions that every character shares) and checks
  every rule, including the formulas of derived stats.
- `parseCharacter(json, schema)` reads one character sheet against the schema and fills in its starting
  values.
- `Character` is a character at runtime: `stat`, `need`, `level`, `affinity` and the methods that change
  them. Each change reports what it did as events.
- `Roster` holds several characters and answers queries: by id, by tag, and the affinities between them.
  Characters join and leave it (`join`, `leave`), and its snapshot and restore follow those changes.

## Example

```ts
import { Character, Roster, parseCharacter, parseStatSchema } from "@velvet/characters";

const schema = parseStatSchema(await readText("schema.json"));
const mara = new Character(schema, parseCharacter(await readText("mara.json"), schema));
const bram = new Character(schema, parseCharacter(await readText("bram.json"), schema));
const roster = new Roster([mara, bram]);

// Each frame, pass the game seconds that the clock moved on. The game converts its own clock.
function frame(gameSeconds: number): void {
  for (const event of mara.tickNeeds(gameSeconds)) {
    // A need crossed a threshold, for example { event: "threshold_crossed", need: "hunger", threshold: "hungry", ... }.
    if (event.event === "threshold_crossed") showNeedWarning(mara.name, event.need, event.threshold);
  }
}

// A gift: the affinity moves, and the event says when it moved to another level.
for (const event of mara.changeAffinity(bram, 25, "gift.flowers")) {
  if (event.event === "affinity_level") showRelationship(event.from, event.to, event.direction);
}

mara.gainXp("farming", 150); // the events list the levels reached, one level_up each

console.log(mara.stat("work_power")); // a derived stat: its formula reads the effective stats
console.log(roster.byTag("villager").map((character) => character.id)); // sorted by id
console.log(roster.relationsOf("mara", { minLevel: "friendly" }));

// Saves: the snapshot is plain JSON, and the save store takes it as it is.
await saves.save("slot-1", roster.snapshot());
const loaded = await saves.load("slot-1");
roster.restore(loaded?.data); // checked and migrated; refused whole when it is wrong
```

`readText`, `showNeedWarning`, `showRelationship` and `saves` are the game's own. The save store is
`createSaveStore` from `@velvet/core`, which wraps the snapshot in its envelope.

## The API

| Call | What it does |
|---|---|
| `parseStatSchema(json)` | Reads a schema document and checks every rule. Returns the schema in its normal form. |
| `parseCharacter(json, schema)` | Reads a character document against the schema. Fills in the starting values. |
| `new Character(schema, definition)` | A character at its starting values. |
| `stat(id)`, `stats()` | The effective value of one stat, or of all of them in schema order. |
| `baseStat(id)`, `changeBase(id, delta)` | The base value of a base stat, and a change to it (clamped). |
| `need(id)`, `rate(id)` | The current value of a need, and its effective change per game second. |
| `tickNeeds(seconds)` | Moves every need by its rate. Returns the thresholds that the move crossed. |
| `changeNeed(id, delta)` | Adds to a need (a meal, a fight). Returns the thresholds that the change crossed. |
| `xp(profession)`, `level(profession)` | The experience of a profession, and its level. |
| `gainXp(profession, amount)` | Adds experience. Returns one `level_up` event for each level reached. |
| `traits`, `hasTrait(id)`, `addTrait(id)`, `removeTrait(id)` | The traits a character has now (sorted), and changes to them. |
| `affinity(other)`, `affinityLevel(other)` | The affinity toward another character, and the name of its level (or `null`). |
| `affinities()`, `history` | The affinities the character holds, sorted by id, and its recent changes. |
| `changeAffinity(other, delta, reason)` | Moves an affinity, clamped. Returns an `affinity_level` event when the level changed. |
| `snapshot()`, `restore(state)` | The character's runtime state, as plain JSON; and a checked restore of such a state. |
| `new Roster(characters)` | A set of characters that share one schema. Their ids must differ. |
| `join(character)` | Adds a character, checked like the constructor: its id must be new, and its schema must be the roster's. |
| `leave(id)` | Removes a character and returns it. Unknown ids fail with `unknown-character`. |
| `get(id)`, `all()`, `byTag(tag)` | One character by id (or `undefined`); every character sorted by id; the characters that carry a tag. |
| `relationsOf(id, { minLevel? })` | The affinities a character holds toward others. |
| `opinionsOf(id, { minLevel? })` | The affinities that other roster members hold toward a character. |
| `snapshot()`, `restore(states)` | The states of the members; a restore that applies all of its states or none. A state for a character that left brings it back. |

Errors are `CharacterError`s with a `code` (see `src/error.ts` and the error table in
`docs/formats/CHARACTER.md`): for example `unknown-stat`, `derived-stat`, `division-by-zero`,
`out-of-range`, `unsupported-document` and `argument`.

## Conventions

- **Ids** are lowercase, and count as ids: `strength`, `hungry`, `farming`. Stats `min`, `max`, `clamp` and
  `floor` are reserved, because formulas use them as function names.
- **Texts are i18n keys**: `name`, `title`, `label` and the `reason` of an affinity change. The package
  never reads their text; the game looks them up in its catalogs (`docs/formats/LOCALE.md`).
- **Formulas** are a small safe language (`docs/formats/CHARACTER.md`, *Formulas*), read by a parser of
  its own. No `eval`, no `Function`. A formula reads the effective values of the stats it names.
- **Events** are tagged by `event`: `threshold_crossed`, `level_up` and `affinity_level`. A call returns
  them in order, and a change that moves nothing returns none.
- **Saves**: `snapshot()` writes the runtime values only, with `format`, `version` and `kind`. The sheet and
  the schema are not in a save. A restore refuses a state of another character, a newer version, or a value
  of the wrong type; it clamps numbers to their ranges, and takes a missing entry from the sheet.
- **Membership**: only `join` and `leave` change who is in a roster. A restore never removes a member, so a
  character that joined after a save stays until it leaves. A state of a character that left brings it back,
  with the state's values; the roster keeps the characters that left for this, and nothing else about them.
- **Events**: a roster has no event listeners, so `join` and `leave` raise no events.

## Tests

`pnpm test` (from `js/`) runs the shared fixtures in `docs/formats/fixtures/characters` (the cases, the
formula language and the refused documents) and the unit tests next to the code. The fixtures are the
conformance suite of the format: a Rust reader runs the same files.
