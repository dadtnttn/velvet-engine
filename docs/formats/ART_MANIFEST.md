# Art manifest v1 (`velvet-art`): art ids, files and placeholders

A game asks for art by **id** (`character.lyra.portrait`), never by file path. The manifest says what
each id is: its kind, the box it is drawn in, its pivot, and its variants (`happy`, `night`). Files in
`art/` provide the pictures. An id with no file yet shows a generated **placeholder**, so provisional
art can be replaced later without touching code.

This document is normative for version 1. The reference implementation is `js/packages/assets`
(`@velvet/assets`). The conformance fixtures live in `docs/formats/fixtures/art/`: `manifest.json`,
`cases.json` and `invalid/*.json`. Where this text and the code disagree, the fixtures decide.

## Conventions

- JSON is UTF-8. Field names are `snake_case`. Readers ignore fields they do not know.
- Ids, variant names and file names are ASCII and lowercase.
- Sizes and anchors are JSON numbers. Sizes are whole numbers. All sizes are logical pixels of the
  1920×1080 stage.

## Folder layout

```text
game/
  art/
    manifest.json                          the manifest
    portrait/character.lyra.portrait.webp
    portrait/character.lyra.portrait.happy.png
    fullbody/character.lyra.fullbody.avif
    card/card.wood.png
    icon/icon.coin.svg
    background/bg.forest.webp
    background/bg.forest.night.webp
    prop/prop.well.png
```

A file is named `art/<kind>/<id>[.<variant>].<ext>`, where `<ext>` is `avif`, `webp`, `png` or `svg`.

Readers build file names from the manifest and look for them. They never parse a file name back into
an id and a variant, because ids contain dots. A file counts only in the folder of its entry's kind.

## Documents

Every document has this envelope:

| Field | Value |
|---|---|
| `format` | `"velvet-art"` |
| `version` | `1` |

A reader refuses a document whose `format` or `version` it does not read (`format`, `version`).

## Manifest

The manifest has two top-level fields: `format` and `version` (above), and `entries`, an array that
may be empty. Entries keep the order of the file.

### Entry

| Field | Type | Default | Meaning |
|---|---|---|---|
| `id` | string | required | The art id. Unique in the manifest. See [Ids and variants](#ids-and-variants). |
| `kind` | string | required | One of the [kinds](#kinds). |
| `size` | two integers | required | The box the art is drawn in: `[width, height]`, each a whole number from 1 to 7680. |
| `anchor` | two numbers | `[0, 0]` | The pivot inside the box, as fractions of the size from the top-left corner: each from 0 to 1. `[0.5, 1]` is the bottom middle. |
| `variants` | string array | `[]` | The variant names of this id. Unique within the entry. |
| `meta` | object | `{}` | Notes for later art generation. The runtime never reads them. See [Metadata](#metadata). |

Example (from `fixtures/art/manifest.json`):

```json
{
  "id": "character.lyra.portrait",
  "kind": "portrait",
  "size": [480, 640],
  "anchor": [0.5, 1],
  "variants": ["neutral", "happy", "worried"],
  "meta": { "prompt": "a friendly young baker with flour on her apron", "style": ["soft lines"] }
}
```

### Ids and variants

- An **id** has 1 to 128 characters and matches `^[a-z0-9_-]+(\.[a-z0-9_-]+)*$`: segments of
  lowercase letters, digits, `_` and `-`, joined by single dots. Examples: `card.wood`,
  `character.lyra.portrait`, `bg.forest`.
- A **variant name** has 1 to 32 characters and matches `^[a-z0-9_-]+$`. It has no dot, so that a
  file name `<id>.<variant>` is unambiguous for the entry that owns it.

### Kinds

| Kind | Use | Placeholder shape |
|---|---|---|
| `portrait` | A character's picture for dialogue or menus. | Standing figure |
| `fullbody` | A whole character. | Standing figure |
| `card` | A card face. | Framed card |
| `icon` | A small round icon. | Circle icon |
| `background` | A scene behind the play area. | Gradient landscape |
| `prop` | An object in a scene or on a board. | Dashed tile |

The kind list is closed in version 1. Adding a kind changes what readers must do, so it needs
version 2. The sizes in the fixtures are examples, not rules: `portrait` `[480, 640]`, `fullbody`
`[640, 1080]`, `card` `[180, 260]`, `icon` `[64, 64]`, `background` `[1920, 1080]`, `prop` `[128, 160]`.

## Resolution

A request is an id and, optionally, one of its variants.

1. An id that the manifest does not declare is refused (`unknown-id`). A variant that the entry does
   not declare is refused (`unknown-variant`). Nothing else is refused: a missing file is not an error.
   A runtime may run *lenient* instead of strict, so a typo shows on screen rather than breaking the frame that
   draws it: it reports the refusal once, draws a `prop` placeholder of 128×128 for an unknown id, and answers
   an unknown variant with the base picture. Tools and conformance tests run strict.
2. The candidate names are, in this order: the variant's name `<id>.<variant>` (only when a variant is
   requested), then the base name `<id>`.
3. For each candidate name, in turn, the formats are tried in this order: **AVIF, WebP, PNG, hand-made
   SVG**. The first file that exists, `art/<kind>/<name>.<format>`, answers.
4. When no file answers, the answer is the **placeholder** of the id. It is the same for every variant.

Rules that follow from this order:

- A variant falls back to the base id. A variant's own file, in any format, beats the base id's file
  in any format, even a better one. Nothing falls back from the base to a variant.
- The format order is a preference among files of the same name. It is not a preference between a
  variant and a base.
- A hand-made SVG is an ordinary file. It is art, not a placeholder.
- A reader may leave out formats it cannot show, for example AVIF on an older system webview. The
  order of the formats that remain does not change.
- A reader that fails to show a file may mark it broken. The file then counts as absent for the rest of
  the session, and resolution goes on to the next file.

A resolution holds:

| Field | Meaning |
|---|---|
| `source` | The format that answered: `avif`, `webp`, `png`, `svg`, or `placeholder`. |
| `path` | The file's path inside the art folder, such as `portrait/character.lyra.portrait.webp`. Null for a placeholder. |
| `from_variant` | The variant whose own file answered. Null when the base id's file or a placeholder answered. (The JavaScript API calls it `variant`.) |

A reader is given the paths of the files that exist, relative to the art folder. Paths use `/`. A `\`
is read as `/`, and a leading `./` or `/` is ignored.

## Placeholders

A placeholder is a generated SVG made from the id alone. It is the answer when no file is found for the
base name. Its text depends only on the id, the kind, the size and the label: the same arguments always
give the same text.

### Label

The **short label** of an id is the text that a placeholder shows:

1. The first segment (the namespace) is dropped when the id has more than one segment.
2. The last segment is dropped too when it repeats the kind, and at least one segment remains.
3. The remaining segments are joined with spaces. Each run of `_` or `-` becomes one space, and the
   text is trimmed.
4. A label longer than 24 characters keeps its first 21 characters, then `...`.
5. An empty label gives the id itself, cut the same way.

| Id | Kind | Short label |
|---|---|---|
| `character.lyra.portrait` | portrait | `lyra` |
| `card.wood` | card | `wood` |
| `prop.old_well` | prop | `old well` |
| `icon.gold.coin` | icon | `gold coin` |
| `portrait` | portrait | `portrait` |
| `prop.the_old_stone_well_at_the_edge_of_town` | prop | `the old stone well at...` |

### Colour (normative)

1. Hash the UTF-8 bytes of the id with 32-bit **FNV-1a**: start from `2166136261`; for each byte,
   XOR it into the hash, then multiply by `16777619` modulo 2^32. The hash of `a` is `0xe40c292c`, and
   the hash of `foobar` is `0xbf9cf968`.
2. The **hue** is the hash modulo 360, in degrees.
3. The **saturation** and **lightness** come from the kind:

   | Kind | Saturation | Lightness |
   |---|---|---|
   | portrait | 40% | 60% |
   | fullbody | 40% | 55% |
   | card | 45% | 42% |
   | icon | 55% | 48% |
   | background | 35% | 45% |
   | prop | 40% | 45% |

4. Convert HSL to RGB with the formula that CSS `hsl()` uses. Round each channel to the nearest
   integer from 0 to 255, and write the colour as `#rrggbb` in lowercase.

Examples: `character.lyra.portrait` is `#70c272`; `card.wood` is `#8b3b9b`; `bg.forest` (background)
is `#9b814b`.

### Ink

The **ink** is the colour of text and lines on the placeholder. It is `#1a1a1a` when the relative luma
of the fill is above 0.5, and `#ffffff` otherwise. The relative luma is `0.2126 R + 0.7152 G + 0.0722 B`,
with each channel from 0 to 1 and not linearised.

### Drawing (described, not byte-exact)

The SVG is as large as its box: `width` and `height` are the size, and `viewBox` is `0 0 width height`.
It has no scripts and no external references. Text is XML-escaped. Coordinates and lengths are whole
numbers. Each kind draws:

- **portrait, fullbody**: the fill over the whole box, and a standing figure in ink at 30% opacity: a
  head, a torso and two legs. The label sits at the bottom.
- **card**: a rounded face in the fill, a thin frame in ink, a band across the top, and the label in the
  middle.
- **icon**: a disc in the fill with a ring in ink, and the first letter of the label in capitals in the
  middle.
- **background**: a sky gradient from the fill, 25 lightness points lighter at the top to 20 points
  darker at the bottom, two hills in ink at low opacity, and the label in white with a dark outline.
- **prop**: a rounded tile in the fill with a dashed ink outline, and the label in the middle.

The colour and the label are normative: another reader must produce the same ones. The drawing is
described so that placeholders look alike; it is not a byte-level contract.

A runtime may supply its own placeholder generator in place of the one above, for example one that draws
stick figures from a character sheet. Its picture is still a placeholder: its source is `placeholder`, it has no
file path, and a missing file is still not an error. Such a generator is the runtime's own code, so the colours,
labels and drawing of this section do not bind it. The generator described here remains the reference, and a
runtime falls back to it wherever its own generator gives no picture.

## Missing-art report

The report is a JSON document. It lists the ids that still need art.

```json
{
  "format": "velvet-art-report",
  "version": 1,
  "missing": [
    { "id": "character.lyra.portrait", "kind": "portrait", "size": [480, 640], "base": true, "missing_variants": ["neutral", "worried"] }
  ]
}
```

| Field | Meaning |
|---|---|
| `id`, `kind`, `size` | The entry's id, kind and box. |
| `base` | True when the base name has a file. False when the id shows a placeholder. |
| `missing_variants` | The declared variants that have no file of their own, in manifest order. |

Rules:

- An entry has a row when its base has no file, or when at least one declared variant has no file of its
  own. An entry with every file present has no row.
- A file counts when its format is accepted and the reader has not marked it broken. A hand-made SVG
  counts as a file.
- Rows are sorted by id, in byte order. With nothing missing, `missing` is an empty array.

## Metadata

`meta` holds notes for art generation: prompts, style tags, and so on. The runtime never reads it.
These keys are conventional and are not checked: `prompt` (text), `negative` (text), `style` (an array
of text) and `notes` (text). Tools that generate art read them to build prompts, and write the file that
the entry names.

## Validation

A manifest is refused as a whole. A reader reports every problem it finds, each with its path (for
example `entries[2].size`), and the first problem gives the error code.

| Code | When |
|---|---|
| `json` | The text is not JSON. |
| `format` | The format tag is not `velvet-art`. |
| `version` | The version is not 1. |
| `structure` | A field is missing, or has the wrong JSON type: `entries` is not an array, an entry is not an object, an id is not a string, `size` is not an array, `meta` is not an object, and so on. |
| `value` | A value breaks its grammar or range: an id, a variant name, a kind, a size or an anchor. |
| `duplicate` | Two entries share an id. A variant is listed twice. Two entries would use one file name, for example an entry `card.wood` with variant `red` and an entry `card.wood.red`, both of kind `card`. Entries of different kinds never clash, since their folders differ. |

Strict lookups refuse with `unknown-id` and `unknown-variant`; lenient ones report the same codes once and draw on.

## Versioning

This document is version 1. A reader refuses any other version. An optional field whose absence means
the old behaviour may be added within version 1, because readers ignore fields they do not know.
Changing a kind, a grammar, a default, the resolution order or the placeholder colour rule needs
version 2.

## Errors

The names that readers use: `json`, `format`, `version`, `structure`, `value`, `duplicate`,
`unknown-id`, `unknown-variant`.

## Conformance fixtures (`fixtures/art/`)

- `manifest.json`: a valid manifest with six entries, covering every kind, anchors, variants and meta.
- `invalid/*.json`: manifests that must be refused. `cases.json` names the code of each one.
- `cases.json`:
  - `manifest`: the manifest to load.
  - `invalid`: a list of `{ file, code }`.
  - `cases`: each has a `name`, the art-folder `files` that are present, an optional `accept` list of
    formats, the `lookups`, and an optional `report` with the expected missing rows. A lookup gives
    either `expect` with `source`, `path` and `from_variant`, or `expect.error` with the code it must
    raise.
  - `placeholders`: each has an `id`, a `kind`, the short `label` and the placeholder `color`.
