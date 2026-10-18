# Art preparation v1 (`velvet-artprep`): prompts, profiles, jobs and the selection gallery

Art preparation is tooling, not runtime. A project writes a few JSON documents: the generation
**profiles**, the **style** of the project, one **identity** per character, and the **shots** that
say which pictures to make. The preparation step composes each picture's prompt from three layers,
writes one **job** per shot for an external batch runner, and writes a **gallery** page where a
person chooses the candidates. It runs no model and writes no art file. The game never reads these
documents; it reads the art manifest (`docs/formats/ART_MANIFEST.md`), which the chosen pictures
enter.

This document is normative for version 1. The reference implementation is `js/packages/artprep`
(`@velvet/artprep`). The conformance fixtures are in `docs/formats/fixtures/artprep/`. Where this text
and the code disagree, the fixtures decide.

## Conventions

- JSON is UTF-8. Field names are `snake_case`. A reader refuses a name that the format does not define
  (`unknown-field`). This is stricter than the runtime formats: every name here changes a picture, so a
  misspelt name must not pass silently.
- An **id** of a profile or a shot matches `^[a-z0-9][a-z0-9_-]{0,63}$`.
- A **character id** matches `^[a-z][a-z0-9_]{0,63}$`, as in `docs/formats/CHARACTER.md`. An identity
  has the id of its character in the character sheet, so both describe the same character.
- An **art id** and a **variant name** follow the art manifest (`docs/formats/ART_MANIFEST.md`): an art
  id has at most 128 characters; a variant has 1 to 32 characters of `a`-`z`, `0`-`9`, `_` and `-`.
- A **line of text** is trimmed, has between 1 and a stated number of characters, and sits on one line.
- A **joiner** is a string of up to 8 characters on one line. It may be empty, and it is not trimmed.
- A **phrase** is either a line of text, which has the weight 1, or an object `{ "text": ..., "weight": n }`.
  The weight is a number above 0 and at most 2. The text is at most 200 characters.

## Documents

Every document has this envelope:

| Field | Value |
|---|---|
| `format` | `"velvet-artprep"` |
| `version` | `1` |
| `kind` | `"profiles"`, `"style"`, `"identity"`, `"shots"`, `"job"` or `"results"` |

A reader refuses a document whose format is wrong (`format`), whose version is not 1 (`version`), or
whose kind is not the one it reads (`kind`). These three checks come first, and a wrong envelope stops
the reading.

| Kind | Holds | Read by | Written by |
|---|---|---|---|
| `profiles` | The generation profiles of a project. | `parseProfiles` | the author |
| `style` | The style layer of a project. | `parseStyle` | the author |
| `identity` | The identity layer of one character. | `parseIdentity` | the author |
| `shots` | The pictures to make. | `parseShots` | the author |
| `job` | One job for the batch runner. | (the runner) | `buildJobs` |
| `results` | What the runner made, shot by shot. | `parseResults` | the runner |

## Profiles (`kind: "profiles"`)

A profile is a named generation setup for one model family. The document holds a `profiles` array of
at most 64 profiles, in the order of the file. Ids are unique.

| Field | Type | Default | Meaning |
|---|---|---|---|
| `id` | id | required | The profile's name. Jobs and results carry it. |
| `label` | text, 1 to 80 | the id | A name for people. |
| `mode` | `"generate"` or `"edit"` | required | `generate` makes new pictures. `edit` changes one picture that exists. |
| `generation` | object | required | The sampling settings, see below. |
| `size` | object | required | How a job's size follows the art box, see below. |
| `max_references` | integer, 0 to 8 | `0` | The most input images a job may use. In an edit, the picture being edited counts as one. |
| `max_identities` | integer, 1 or 2 | `2` | The most characters that one picture may show. |
| `negative` | boolean | required | Whether the profile takes a negative prompt. |
| `trigger_words` | up to 8 texts of 1 to 64 characters | `[]` | Keywords that the profile's LoRAs need. They lead the prompt. |
| `prompt` | object | required | How the prompt is written, see below. |
| `edit_guards` | object | `{}` | Extra text for a variation field that an edit changes. Only edit profiles may have it. |

`generation` holds:

| Field | Type | Default | Meaning |
|---|---|---|---|
| `model` | text, 1 to 120 | required | The checkpoint, as the runner knows it. |
| `loras` | up to 8 objects `{ "name", "weight" }` | `[]` | The LoRAs. `name` is text of 1 to 120 characters; `weight` is from 0 to 2 and defaults to 1. |
| `sampler` | text, 1 to 60 | required | The sampler. |
| `scheduler` | text, 1 to 60 | `null` | The scheduler, when the profile names one. |
| `steps` | integer, 1 to 200 | required | The number of steps. |
| `cfg` | number, 0 to 30 | required | The guidance scale. |
| `shift` | number, 0 to 20 | `null` | The shift of a flow model, when the profile sets one. |

`size` holds `long_side` (an integer from 64 to 4096, required) and `multiple_of` (an integer from 8 to
256, required). `long_side` must be a multiple of `multiple_of`. The size rule is in [Jobs](#jobs).

`prompt` holds:

| Field | Type | Default | Meaning |
|---|---|---|---|
| `style` | `"prose"` or `"tags"` | required | Prose writes sentences. Tags write a list of phrases. |
| `order` | the three layer names, each once | `["style", "identity", "variation"]` | The order of the layers. |
| `quality_tags` | up to 16 texts of 1 to 64 characters | `[]` | Tags that lead the prompt. Tag profiles only. |
| `weights` | `"paren"` or `"none"` | `"paren"` for tags, `"none"` for prose | `paren` writes a weight other than 1 as `(text:1.2)`. Tag profiles only. |
| `phrase_joiner` | joiner | `", "` | The text between two phrases of one layer. |
| `layer_joiner` | joiner | `" "` for prose, `", "` for tags | The text between two layers. |
| `layer_end` | joiner | `"."` for prose, `""` for tags | The text that closes each layer. |

Rules for profiles:

- A prose profile takes no `quality_tags`, and its `weights` may only be `"none"`. Prose has no weight syntax.
- An edit profile has `max_references` of at least 1: the picture it edits.
- `edit_guards` is allowed only in an edit profile. Its keys are variation field names
  (see [Shots](#shots)), and each value is text of 1 to 200 characters.

### Example profiles

The fixtures hold these four profiles as examples. A project writes its own; the package has no
built-in profiles.

| Profile | Mode | Prompt | Negative | `max_references` | `max_identities` | `long_side` |
|---|---|---|---|---|---|---|
| `krea2-niji-base` | generate | prose, trigger word `@NJSW33T` | no | 0 | 1 | 1024 |
| `krea2-reference` | generate | prose | no | 2 | 2 | 1024 |
| `krea2-edit` | edit | prose, guard for a moved pose | no | 1 | 2 | 1024 |
| `generic-sd` | generate | tags, quality tags, paren weights | yes | 1 | 2 | 768 |

The two Krea2 generate profiles carry the settings of a working pipeline (RedCraft Krea2 with the Niji
LoRA: euler/simple, 15 steps, CFG 1, shift 1.31 for new pictures; `er_sde`, 12 steps with references).
The sizes, the edit profile and the generic one are placeholders, and a project checks every value
against its own pipeline before it uses it.

`krea2-niji-base` as the fixtures write it:

```json
{
  "id": "krea2-niji-base",
  "mode": "generate",
  "generation": {
    "model": "RedCraft Krea2 3.0 bf16",
    "loras": [{ "name": "Niji Sweet Spot Krea2 v2A", "weight": 1 }],
    "sampler": "euler",
    "scheduler": "simple",
    "steps": 15,
    "cfg": 1,
    "shift": 1.31
  },
  "size": { "long_side": 1024, "multiple_of": 64 },
  "max_references": 0,
  "max_identities": 1,
  "negative": false,
  "trigger_words": ["@NJSW33T"],
  "prompt": { "style": "prose" }
}
```

## The three layers

A picture's prompt has three layers. Each has its own document or part of a shot.

1. **Style** (per project). The `style` document holds `phrases` (up to 60) and `negative` (up to 60
   texts of 1 to 64 characters). The phrases are the look of the whole project. The negatives are used
   only by profiles that take a negative prompt.
2. **Identity** (per character). The `identity` document holds what stays the same in every picture of
   one character:

   | Field | Type | Meaning |
   |---|---|---|
   | `id` | character id | The character. Required. Unique among the identities of a run. |
   | `name` | text, 1 to 80 | The name the prompt uses. Required. |
   | `species`, `body`, `skin`, `hair`, `eyes` | text, 1 to 200 | The permanent traits. Optional. |
   | `outfit` | text, 1 to 200 | Who wears what: the character's usual clothes. Optional. |
   | `has` | up to 30 phrases | Features that the character has, such as horns or a tail. |
   | `lacks` | up to 30 texts of 1 to 64 characters | Features that the character does not have, such as horns or a tail. |
   | `fixed` | up to 30 phrases | Phrases that must appear in every picture of the character, in this order, such as `matte dry skin`. |

3. **Variation** (per shot). The `variation` object of a shot holds what changes from picture to picture:
   `subject`, `framing`, `pose`, `expression`, `outfit`, `setting`, `light` and `palette`. Each is a
   phrase. Unknown names are refused (`unknown-field`).

### Reading the art manifest

An art entry's `meta` block holds notes for generation (`docs/formats/ART_MANIFEST.md`). The runtime
never reads it. A generation tool reads three of its keys:

- `meta.prompt` is the **subject** of the picture. It is the first phrase of the variation layer, unless
  the shot gives its own `variation.subject`, which then replaces it.
- `meta.style` (text items) are extra **style** phrases of that entry. They follow the project's style
  phrases.
- `meta.negative` (text) is extra **negative** text, split at commas. It follows the style's negatives.

`meta.notes` and any other key are for people; the reader ignores them.

## Composition

`composePrompt` writes the prompt of one picture from its layers, in this order:

1. **Leading tags.** A tag profile writes its `quality_tags`, then its `trigger_words`, as one body,
   joined with `phrase_joiner`. A prose profile writes only its trigger words, as one body. Nothing is
   capitalised and no `layer_end` is added to this body.
2. **The layers, in the profile's `order`.** A layer with no phrases is left out.
   - The **style** layer is the style phrases of the project, then the entry's `meta.style` phrases.
   - The **identity** layer of each character is one layer, written in the order of the shot's
     `characters`. Its phrases are, in this order: the name; the species, body, skin, hair and eyes
     that are given; each `has` phrase; the outfit; each lacked feature written as `no <feature>`; each
     `fixed` phrase. A lacked feature is written as `no <feature>` only when the profile takes no negative
     prompt (see below).
   - The **variation** layer is the shot's fields in the order `subject`, `framing`, `pose`, `expression`,
     `outfit`, `setting`, `light`, `palette`. In an edit profile, the guard of a field follows the field,
     when the profile gives one for it.
3. **Each layer** is written as its phrases joined with `phrase_joiner`, then closed with `layer_end`.
   In a prose profile the first character of the layer is a capital letter. In a tag profile a phrase
   with a weight other than 1 is written as `(text:w)` when `weights` is `"paren"`, where `w` is the weight
   with at most two decimals and no trailing zeros, such as `(bright eyes:1.2)`. With `weights` set to
   `"none"`, the weight is not written.
4. **The layers** are joined with `layer_joiner`. A composed prompt may be empty. A job is refused when
   its prompt is empty.

The **negative prompt** is written only when the profile takes one. It is the style's negative phrases,
then the entry's `meta.negative` phrases, then each identity's lacked features in the order of the shot's
characters. A lacked feature is left out when a character in the same picture has a `has` phrase or a
species that contains its word (case-insensitive), so a negative never removes a feature that the picture
asks for. Repeats are removed, keeping the first. The phrases are joined with `phrase_joiner`. When the
result is empty, the negative is `null`. A profile that takes no negative prompt has the negative `null`,
and its lacked features are written in the text as `no <feature>`.

A prose example, from the fixtures, with the profile `krea2-niji-base` and the shot `ada-master`:

```text
@NJSW33T Soft-edged illustration, warm morning light, flat painted colours, clean ink outlines. Ada, human, slim build, light olive skin, short curly dark brown hair tied back with a red ribbon, green eyes, a small silver ring on her left hand, a cream apron over a blue linen dress with rolled sleeves, no horns, no tail, matte dry skin. Full body, standing, arms relaxed at her sides, plain white background, even soft studio light, cream, blue and brown.
```

The same shot in the tag profile `generic-sd` has the leading tags, the layers joined by commas, the paren
weights, and the negative `photo, 3d render, blurry, watermark, text, extra arms, horns, tail`. The
fixtures hold both in full.

## Shots (`kind: "shots"`)

A shots document holds a `shots` array of at most 500 shots. A shot describes one picture:

| Field | Type | Default | Meaning |
|---|---|---|---|
| `id` | id | required | The shot's name. Unique in the document. |
| `art` | art id | required | The art id that the picture fills. |
| `variant` | variant name | none | The variant that the picture fills. It must be declared by the art entry. |
| `characters` | up to 8 character ids, each once | `[]` | The identities that the picture shows, in the order of the prompt. |
| `references` | up to 8 items | `[]` | The input images, by art id: a string, or `{ "id", "variant" }`. |
| `variation` | object | `{}` | The variation layer. Its names are the variation fields. |
| `edit` | object or `null` | `null` | Set for an edit: `{ "base": reference, "change": field }`. |

A **variation field** is one of `subject`, `framing`, `pose`, `expression`, `outfit`, `setting`, `light`
and `palette`.

A shot is built by a profile, and the build checks it (see [Jobs](#jobs)):

- Its art id is in the manifest, and its variant is declared by that entry.
- No two shots of one build make the same picture (the same art id and variant).
- Its characters have identity layers, and there are no more than the profile's `max_identities`.
- Its references and its edit base are in the manifest, and the number of input images (the references,
  plus one for an edit) is no more than the profile's `max_references`.
- In a `generate` profile a shot has no edit. In an `edit` profile a shot has an edit, and its own
  `variation` sets exactly one field: the field named by `edit.change`. An edit changes one thing.

## Jobs

`buildJobs` makes one job per shot. A job is a `job` document with the following fields. It is data for
an external batch runner; the runner makes the candidates and reports them in a results document.

| Field | Meaning |
|---|---|
| `shot`, `profile` | The shot's id and the profile's id. |
| `art`, `variant` | The art id, and the variant or `null`. |
| `characters` | The identities that the picture shows. |
| `prompt`, `negative` | The composed prompt, and the negative prompt or `null`. |
| `references` | The input images as `{ "id", "variant" }`, by art id. The runner resolves them through the manifest. |
| `edit` | `null`, or `{ "base": { "id", "variant" }, "change": field }`. |
| `size` | `[width, height]` in pixels, from the size rule below. |
| `generation` | A copy of the profile's `generation`: `model`, `loras`, `sampler`, `scheduler`, `steps`, `cfg` and `shift`. |
| `seeds` | The seeds of the shot. |
| `outputs` | One `{ "seed", "name" }` per seed. |

**Seeds.** A shot makes six candidates by default: the seeds 101 to 106. A caller may give other seeds: a
list of 1 to 64 whole numbers from 0 to 4294967295, each once.

**Output names.** The name of an output is the art id, then `.` and the variant when there is one, then
`_` and the seed: `<art-id>[.<variant>]_<seed>`, such as `character.ada.portrait.happy_101`. The runner
adds the file extension.

**Size.** The long side of the art box becomes `long_side` pixels. The short side keeps the box's
proportion, rounded down to a multiple of `multiple_of`, and is never less than one multiple. The
orientation of the box is kept. The fixtures give, with `long_side` 1024 and `multiple_of` 64: a box of
480×640 gives 768×1024; a box of 1920×1080 gives 1024×576.

The build reports every problem of a batch at once, each with its path, and then throws. The code of the
error is the code of the first problem.

## Selection gallery (`kind: "results"`)

The runner reports a results document: a `title` (optional, 1 to 200 characters) and a `shots` array.
Each shot has its `shot`, `art`, `variant`, an optional `prompt` (free text of up to 4000 characters, for
the reader), and `candidates`. A candidate is `{ "seed", "name", "image" }`. Its `name` must be the output
name of its art, variant and seed, so the code that a person copies is the file that the pipeline expects.

`galleryHtml` writes one static HTML page from the shots:

- The page has a title, and a summary line that counts the shots and the candidates.
- Each shot is a section, in the order of the results. It has its name, its picture (art id and variant),
  its prompt in a collapsed block, and a grid of its candidates.
- Each candidate shows its picture, its **code** (its output name), and a **Copy code** button. The code
  is the file name without the extension. A shot with no candidates says so.
- The page is self-contained: its style and its one script are inline, and it loads nothing else. The
  script only copies the code of a button to the clipboard.
- Every text and attribute is escaped (`&`, `<`, `>`, `"`, `'`).
- An image source is refused unless it is a relative path with `/` separators, an `https` or `http`
  address, or a `data:image/` URL of a common picture format. A `javascript:` URL, an absolute path, and a
  Windows drive path are refused.

## Pipeline rules

These are the rules that the format and its tools follow. A project's own notes may add to them.

1. **References.** Consistency comes from reference images, not from the prompt alone. The master
   reference of a character is generated on a plain white background, and the shot's `setting` says so.
   A character LoRA, when the project uses one, is named in the profile.
2. **Two identities at most.** A picture made in one generation, and so a reference image, shows at most
   two characters (`max_identities`, 1 or 2). More characters are added by an edit of a picture that
   already has them, not by a longer prompt.
3. **One change per edit.** An edit names one variation field. It changes only that field.
4. **Moved parts.** A pose edit that moves a limb says where the limb goes. The profile's guard, such as
   `nothing left where the moved part was`, says that nothing stays where it was, so that no extra limb
   appears.
5. **No inpainting fixes.** A picture with a flaw is generated again, or it is edited as a whole with an
   edit shot. Masked inpainting is not part of this format.
6. **Background removal by shape.** A background is removed by shape segmentation (for example BiRefNet),
   not by colour keying, so that light clothes and light details are kept.
7. **A person chooses.** Six candidates per shot, shown in the gallery, each with its code. A candidate
   enters the art folder only after a person chooses it, and each chosen picture is checked against the
   identity of its character (the fixed phrases, the outfit, and the features that the character has or
   lacks) before it is used. The format never writes into the art folder.
8. **Fixed traits in every picture.** The skin, the clothes, and the features that a character has or
   lacks (horns, tail) are written in every picture of that character, through its identity layer.
9. **Prose or tags.** A prose profile writes sentences with no quality tags and no weights. A tag profile
   writes a list of tags, and it may use the paren weights.

## Errors

The codes name the rule that broke. The same codes are used by every reader.

| Code | When |
|---|---|
| `json` | The text is not JSON. |
| `format` | The format tag is not `velvet-artprep`. |
| `version` | The version is not 1. |
| `kind` | The document is not the kind that the reader reads. |
| `structure` | A required field is missing, or a field has the wrong JSON type. |
| `value` | A value breaks a rule: a spelling, a range, a combination of fields, an empty prompt, or an unsafe image source. |
| `duplicate` | Two profiles, shots or results share an id; two shots make the same picture; a seed or a value is listed twice. |
| `unknown-field` | A name that the format does not define. |
| `unknown-id`, `unknown-variant` | An art id or a variant is not in the art manifest. |
| `unknown-character` | A shot names a character that has no identity layer. |
| `identities` | A shot shows more characters than its profile allows. |
| `references` | A shot uses more input images than its profile allows. |
| `edit` | An edit does not fit its profile: a missing edit, an edit in a profile that makes new pictures, or an edit that changes more than one thing. |

## Versioning

This document is version 1. A reader refuses any other version. Because a reader refuses the names that it
does not know, a new name is a new version: it needs version 2. A change to a rule of composition, to a
default, or to the size or naming rule needs version 2 too.

## Outside the format

The format does not define:

- how a batch runner loads a job, runs it, or reports its results; a runner reads the job and writes a
  results document;
- which model runs, where it runs, or how much it costs;
- the authoring tool, the page where a person edits the documents, or the folder where chosen pictures go;
- the art manifest's entries, which the art manifest format defines;
- the character sheets, which `docs/formats/CHARACTER.md` defines; an identity only shares their ids.

## Conformance fixtures

`docs/formats/fixtures/artprep/` holds the conformance suite. Every reader runs all of it.

| File | What it holds |
|---|---|
| `profiles.json` | The four example profiles. |
| `style.json` | The project style of the fixtures. |
| `identities/ada.json`, `identities/bex.json` | Two characters: a human baker, and a goat-folk courier with horns and a tail. |
| `manifest.json` | An art manifest with the art ids that the shots use. |
| `shots.json` | Nine shots: masters, portraits with references, a pair of two characters, a prop, and two edits. |
| `expected/<profile>.json` | The jobs that each profile builds from its shots, with the composed prompts, negatives, sizes and outputs. |
| `cases.json` | The builds to run, the builds that must be refused with a code, and the gallery checks. |
| `invalid.json` | Documents that must be refused, each with the reader it is read by and the code it must give. |
| `gallery/results.json` | A results document for the gallery checks. |

Each case is named in plain words in `cases.json` and `invalid.json`. The tests of `@velvet/artprep` run
every file in this folder.
