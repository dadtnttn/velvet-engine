# Audio manifest v1 (`velvet-audio`): sounds, files, buses and playback

A game asks for a sound by **id** (`sfx.door.open`), never by file path. The manifest says what each id
plays: the files to try in order, the bus it goes to, its volume, whether it loops, how many copies may
sound at once, how long it waits before it can play again, and whether it has random variants. Music
also gets loop points.

This document is normative for version 1. The reference implementation is `js/packages/audio`
(`@velvet/audio`). The conformance fixtures live in `docs/formats/fixtures/audio/`. Where this text and
the code disagree, the fixtures decide.

## Conventions

- JSON is UTF-8. Field names are `snake_case`. Readers ignore fields they do not know.
- A **sound id** has 1 to 128 characters and matches `^[a-z0-9_-]+(\.[a-z0-9_-]+)*$`, the same grammar
  as art ids. Examples: `ui.click`, `sfx.door.open`, `music.village.day`.
- A **bus name** is a built-in name (`music`, `sfx`, `voice`, `ui`) or a custom name that the manifest
  declares. A custom name matches `^[a-z][a-z0-9_-]{0,31}$`. The name `master` is reserved for the
  master bus.
- A **file path** is relative to the audio folder, uses `/` as its separator, and follows the grammar in
  [Files](#files).
- Offsets into a file are in **seconds** (`loop_points`). Waits are in **milliseconds** (`cooldown_ms`).

## Documents

Every document has this envelope:

| Field | Value |
|---|---|
| `format` | `"velvet-audio"` |
| `version` | `1` |

A reader refuses a document whose `format` or `version` it does not read (`format`, `version`).

## Manifest

| Field | Type | Default | Meaning |
|---|---|---|---|
| `buses` | array of bus names | `[]` | Custom buses that sounds may use. Each name is listed once, and is not a built-in name. |
| `sounds` | array of sounds | required | The sounds. May be empty. Entries keep the order of the file. |

### Sound

| Field | Type | Default | Meaning |
|---|---|---|---|
| `id` | sound id | required | Unique in the manifest. |
| `files` | array of file paths | required, unless `variants` is given | The candidate files, best format first. From 1 to 8 paths. See [Files](#files). |
| `variants` | array of variant objects | none | Alternative versions of the sound, one of which is picked at random at each play. A variant object has `files`, a list of candidates as above. At least 2 variants. A sound has `files` or `variants`, not both. |
| `bus` | bus name | `sfx` | The bus that the sound plays on. |
| `volume` | number from 0 to 1 | `1` | The sound's own level. See [Volume](#volume). |
| `loop` | boolean | `false` | The sound repeats until it is stopped. |
| `loop_points` | object `{ "start", "end" }` | none | The part that repeats, in seconds. Requires `loop: true`. See [Loop points](#loop-points). |
| `max_voices` | integer from 1 | none (no limit) | How many copies of the sound may play at once. See [Voice limits](#voice-limits). |
| `cooldown_ms` | integer from 0 | `0` | The least time between two plays of the sound. See [Cooldowns](#cooldowns). |
| `preload` | boolean | `false` | The sound is decoded by a preload that names no ids. |
| `meta` | object | `{}` | Notes for tools. The runtime never reads them. |

Example (from `fixtures/audio/manifest.json`):

```json
{
  "id": "sfx.footstep.stone",
  "variants": [
    { "files": ["sfx/stone_1.opus", "sfx/stone_1.wav"] },
    { "files": ["sfx/stone_2.opus", "sfx/stone_2.wav"] }
  ],
  "cooldown_ms": 250,
  "max_voices": 2
}
```

## Files

A candidate is a path such as `ui/click.opus`. The grammar is
`^(?:[a-z0-9_-]+/)*[a-z0-9_-][a-z0-9_.-]*\.(opus|ogg|mp3|wav)$`, at most 256 characters. A path is
lowercase, has no leading slash and no `.` or `..` segment, and ends in one of four extensions:

| Extension | Format | Browser test (`canPlayType`) |
|---|---|---|
| `.opus` | Ogg Opus | `audio/ogg; codecs="opus"` |
| `.ogg` | Ogg Vorbis | `audio/ogg; codecs="vorbis"` |
| `.mp3` | MP3 | `audio/mpeg` |
| `.wav` | WAV (PCM) | `audio/wav; codecs="1"` |

The lowercase rule keeps a file that is found on Windows found on a case-sensitive server too. The
candidates of one list must be different paths. The list starts with the best format: a game usually
writes `.opus` first, and `.mp3` or `.wav` last.

## Buses

| Bus | Meaning |
|---|---|
| `music` | Background music. The music channel plays from this bus. |
| `sfx` | Sound effects. The default. |
| `voice` | Spoken lines. A voice ducks the music (see [Outside the format](#outside-the-format)). |
| `ui` | Interface sounds, such as a button tick. The game's pause does not mute them (see [Outside the format](#outside-the-format)). |
| custom | Any other name declared in `buses`, such as `ambience`. |

The format gives buses their names only. Volumes, mutes and the mix are runtime state.

## Playback rules

These rules are normative: every reader gives the same answers.

### Choosing a file

1. Choose a variant (see [Variants](#variants)). A sound without variants has one.
2. Try the candidates of that variant in order. A reader may skip a format that it cannot play, and keeps
   the order of the rest.
3. A candidate that cannot be fetched or decoded is a failure. The next candidate is tried.
4. When no candidate is left, the sound fails to load. Nothing plays, and the reader reports the failure
   with the sound's id.

### Variants

A sound with variants plays one of them at each play, chosen at random, with no immediate repeat: the
variant that played last is not chosen again. The choice is uniform among the other variants.

### Cooldowns

A play of a sound is skipped when less than `cooldown_ms` has passed since the last play of that sound
that was not skipped. A skipped play starts nothing. The time is counted from the request, so a request
that waits for its file still counts.

### Voice limits

When a play would make more copies of a sound than `max_voices` allows, the oldest copy that is playing
is stopped (stolen), and the new copy starts. Copies are counted when they start, not when they are
requested.

### Loop points

With `loop: true` and `loop_points`, the sound plays from its start to `end` once, and then repeats the
part from `start` to `end`. A reader clamps both points to the length of the file. When the clamped
`end` is not after `start`, the points are ignored and the whole file repeats.

### Volume

A volume `v`, from 0 to 1, is a gain on a perceptual curve of 60 decibels per decade:

    gain(v) = v³        decibels(v) = 60 · log10(v)        gain(0) = 0, which is silence

| Volume | Gain | Decibels |
|---|---|---|
| 0 | 0 | silence |
| 0.1 | 0.001 | -60 |
| 0.5 | 0.125 | -18.06 |
| 1 | 1 | 0 |

The levels of a sound, of a play, of a bus and of the master multiply, so in decibels they add up.

## Lenient reading

A strict reader refuses a manifest that has any problem (see [Validation](#validation)). A lenient
reader, for a shipped build that must keep running, does this instead:

- It still refuses the envelope: text that is not JSON, a wrong `format` or `version`, and a `sounds` or
  `buses` that is not an array.
- It leaves out each custom bus name that breaks a rule, and each sound that has a problem. It keeps
  every other entry.
- It reports each left-out entry with the same codes and paths as a strict reader. A sound on a bus that
  was left out is itself left out.
- A sound whose id repeats the id of an earlier sound is left out, even when that earlier sound was left out
  too. The first use of an id is the one that counts.

The result is a manifest in its normal form, and the list of issues that were found.

## Validation

A manifest is refused as a whole. A strict reader reports every problem it finds, each with its path (for
example `sounds[2].files[1]`), and the first problem gives the error code.

| Code | When |
|---|---|
| `json` | The text is not JSON. |
| `format` | The format tag is not `velvet-audio`. |
| `version` | The version is not 1. |
| `structure` | A field is missing or has the wrong JSON type: `sounds` or `buses` is not an array, a sound is not an object, a sound has neither `files` nor `variants`, a candidate is not a string, `loop` or `preload` is not a boolean, `volume` is not a number, `meta` is not an object, and so on. |
| `value` | A value breaks its grammar or range: an id, a bus name, a path or an extension, a list with no candidate or more than 8, a volume outside 0 to 1, `loop_points` without `loop: true`, or with `end` not after `start`, a `max_voices` below 1, a negative `cooldown_ms`, a `variants` list of fewer than 2, a sound with both `files` and `variants`, a sound on a bus that is neither built in nor declared, or the reserved name `master` declared as a custom bus. |
| `duplicate` | Two sounds share an id. Two candidates of one list are the same path. A custom bus repeats a name, or repeats a built-in name. |

## Versioning

This document is version 1. A reader refuses any other version. An optional field may be added within
version 1, because readers ignore fields they do not know. A change to a grammar, a default, a playback
rule or the volume curve needs version 2.

## Errors

The names that readers use: `json`, `format`, `version`, `structure`, `value` and `duplicate`. The
reference implementation also reports three names for calls on a parsed manifest: `unknown-id` (no sound
has the id), `unknown-bus` (the bus is neither built in nor declared) and `wrong-bus` (a music call names a
sound that is not on the `music` bus).

## Conformance fixtures

`docs/formats/fixtures/audio/` holds the suite. Every reader runs all of it.

- `manifest.json`: a valid manifest with each kind of sound: candidates in several formats, a preloaded
  click, variants with a cooldown and a voice limit, looped music with loop points, a voice line with
  variants, and a sound on a custom bus.
- `lenient.json` and `lenient-bus.json`: manifests with problems, which a lenient reader repairs by
  leaving out the bad entries.
- `invalid/*.json`: manifests that a strict reader must refuse, one for each rule above.
- `cases.json`:
  - `manifest`: the manifest to load, and `buses`: its custom buses.
  - `invalid`: a list of `{ file, code, path }`. The first issue has the code, and one of the issues has
    the path.
  - `lenient`: a list of `{ file, kept, codes }`: the ids that a lenient reader keeps, and the codes of its
    issues in order.
  - `sounds`: for each sound to check, its `id` and the `fields` that its normal form must have. A field
    that is not listed is not checked.
  - `curve`: the gain of each volume, and its decibels where given.

## Outside the format

The format does not define:

- what the game plays, and when: the game asks for sounds by id;
- the mix at a given moment: the volume and mute of each bus, the ducking of music under a voice, pauses,
  and the rule that browsers use to allow audio to start. The reference implementation has its own
  defaults for these;
- the decoding of files, streaming, spatial audio and effects;
- where settings are kept, which the runtime chooses.
