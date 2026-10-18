# Guion v1 — dialogue scenes in Markdown

A guion is a file of dialogue written the way a writer drafts it: Markdown, with one scene per
`##` heading and one box per line. The same file plays as a dialogue box in a 3D world and, for
scenes marked `vn`, as a visual novel.

Reference implementation: `crates/velvet-guion` (the parser, the conversation rules and the
visual-novel compiler). `crates/velvet-wasm` exposes it to JavaScript as JSON, and
`js/packages/guion` is the JavaScript API. This document is normative for version 1. Where it
and the code disagree, the fixtures in `docs/formats/fixtures/guion/` decide.

## Conventions

- Files are UTF-8 text. Line endings may be LF or CRLF.
- Line numbers in problems count from 1.
- Keywords, attribute names and labels are case-insensitive, because they are slugged (below).
  The text that the characters say is kept as written.

## Slugs

A **slug** turns a label, a name or a file name into an identifier:

1. Trim the text, and lower-case the ASCII letters.
2. Fold the accents `á à ä â é è ë ê í ì ï î ó ò ö ô ú ù ü û ñ` to their plain letters, and the
   capitals `Á À É Í Ó Ú Ñ` too. Other accented capitals (`Ä Â È Ê Ë Ì Ï Î Ò Ö Ô Ù Ü Û`) are not
   folded: they count as separators, so write those names in lower case.
3. Keep ASCII letters and digits. Every run of any other character becomes one `_`.
4. Drop leading and trailing `_`. An identifier that starts with a digit gets an `s` in front.

Examples: `La Capitana` is `la_capitana`, `Ana María` is `ana_maria`, `¿Qué?` is `que`, and `2da` is
`s2da`.

## The file

A guion may begin with **front matter**: a first line `---`, then `key: value` lines, then a line
`---`. The keys are:

| Key | Meaning |
|---|---|
| `id` | The file id. Default: the file name without its extension, slugged. |
| `sala` | The room of the file's visual-novel scenes (a `@sala` directive changes it within a scene). Without it, the game's default room is used. |
| `titulo` (or `title`) | The title of the file. |

These three keys are matched as slugs, so `Titulo` and `título` are the same key. Any other key is
**metadata** (below). A line in the front matter with no colon is ignored.

### Metadata

Any other `key: value` line of the front matter is the file's metadata: the game declares what it needs
per file, such as a category, the participants or an illustration. The metadata is an open string map,
kept in the order of the file:

- A **key** is written in lower case: a letter first, then lower-case letters, digits or `_`
  (`categoria`, `participantes`, `ilustracion_2`). A key starts with a letter, not a digit, because a
  JavaScript object lists number-like keys first, whatever the file's order. A key that breaks the rule
  is a problem: `clave no válida en la cabecera: <key>`.
- A key given **twice** is a problem: `clave repetida en la cabecera: <key>`. The first value is kept.
  `titulo` and `title` count as one key, so giving both is a problem too.
- A **value** is the rest of the line after the first colon, trimmed, and kept as written. It may be
  empty, and it may hold colons (`hora: 10:30`).
- A **list** is written with commas and stays one string: `participantes: Lira, Nyxara, Nemi` has the
  value `Lira, Nyxara, Nemi`. The game splits it, and trims the parts. velvet-guion does not split lists.

```text
---
id: escena_bano
sala: termas
titulo: El baño
categoria: escena
participantes: Lira, Nyxara, Nemi
ilustracion: lira_bano
---
```

reads as the metadata `categoria` = `escena`, `participantes` = `Lira, Nyxara, Nemi`, and `ilustracion` =
`lira_bano`, in that order. A file with no front matter, or with front matter that has no other keys,
has no metadata.

Reading it:

- **Rust:** `guion.meta` is the list of `(key, value)` pairs in file order, and `guion.meta_value("categoria")`
  finds one value (`None` when the key is missing).
- **JavaScript:** `parseGuion` and `GuionLibrary.add` return `meta`, an object with the keys in file
  order. Use `Object.hasOwn(meta, key)` to tell a missing key from an empty value.
- **Splitting a list** (JavaScript):

  ```ts
  const names = (file.meta["participantes"] ?? "").split(",").map((name) => name.trim()).filter(Boolean);
  ```

Metadata does not change how the file plays: the game reads the keys it needs from `meta`.

Outside the front matter:

- A line that starts with `>` is a note for the writer. It is ignored.
- A block `<!-- ... -->` is a comment. It may span lines. It is ignored.
- A blank line, and a line that is only `---`, are ignored.
- A line `# Title` sets the title of the file, if the front matter has none. It starts no scene.

Blank lines and notes do not end a choice (below).

## Scenes

A scene starts at a heading of two or more `#`, followed by its label and, in braces, its attributes:

```text
## llegada {en=puerta, si=oro>=2&noche}
```

- The label is the heading text before the braces, slugged. The scene's **id** is
  `<file id>.<label>`, such as `posadera.llegada`. The id is what other files, the game and
  `start` use.
- Lines before the first heading form an unnamed scene: label `inicio`, title `Inicio`.
- Two scenes with the same label in one file: `escena repetida: <label>`. Both are kept.
- A heading with no label: `escena sin nombre`.

### Attributes

Attributes are separated by commas, inside the braces.

| Attribute | Meaning |
|---|---|
| `quien=<npc>` | The scene starts when the player talks to that NPC. |
| `en=<place>` | The scene starts when the player uses that place. |
| `si=<conditions>` | The scene starts only while the conditions hold. Join several with `&`, because a comma separates attributes. |
| `una_vez` | The scene starts only the first time: once it has been seen, `entry` skips it. |
| `vn` | A visual-novel scene: velvet-guion compiles it into the visual novel's nodes. |

Anything else in the braces is a problem: `atributos ilegibles en: <heading>`. The scene is kept
with its default attributes.

A scene with `quien`, `en` or `vn` is an **entry**. A scene that a jump, an option or a `@vn` of
the same file leads to is a **branch**. A scene that is neither an entry nor a branch **follows on** from
the scene before it in the file: when that scene runs out of steps, play goes on into this one.

## Lines

Each line of a scene is one box. These forms are tried in this order:

| Form | Kind | Who speaks |
|---|---|---|
| `**Name:** text` | `dialogue` | The character id of `Name` (see the cast). |
| `**Name (mood):** text` | `dialogue` | As above; `mood` is how they look (slugged), for the portrait. |
| `*text*` | `sound` | `narrator`, with no name. |
| `(text)` | `thought` | The thinker of the cast, shown under its name. Without a thinker, nobody. |
| anything else | `narration` | `narrator`, with no name. |

In a dialogue, the dash that the writers put before the words (`—`, `–` or `-`) is dropped:
`**Greta:** —Pasa.` says `Pasa.`. A bold name with no closing `**` is narration.

A line with no text, such as `**Greta:**` alone, is a problem: `línea sin texto`. The line is
dropped.

## Choices

A `?` line opens a choice. Its text is the prompt (it may be empty). Each following line that
starts with `- ` is an option:

```text
? ¿Qué le ofreces?
- Un caldo caliente -> caldo {greta_confianza +1}
- Una habitación [oro >= 5] -> habitacion {oro -5}
- Nada, sigo camino -> fin
```

An option has the form `label [conditions] -> target {effects}`. Every part after the label is
optional, and they come in this order:

- `[conditions]` (see below): the option is shown, but closed, while they do not hold.
- `-> target`: the scene to go to. `fin` ends the conversation. Without a target, play goes on
  with the next step of the scene. A target is a label of this file, or `file.label`.
- `{effects}`: changed when the option is taken (see below).

Options are read only directly after the `?` line. Blank lines and notes may come between them;
any other line ends the choice. An option that cannot be read is a problem: `opción ilegible:
<text>`, and it is dropped. A choice with no options is a problem, `una elección sin opciones`,
reported once per scene. Outside a choice, a `- ` line is narration.

## Conditions

A condition is `<variable> <operator> <integer>`:

| Operator | Meaning |
|---|---|
| `>=`, `<=`, `>`, `<` | The usual comparisons. |
| `==` or `=` | Equal. |
| `!=` | Not equal. |

A bare variable name means *set*: its value is above 0. Several conditions in an option or a
`@si` are joined with `,` or `&`, and all must hold. A variable that has never been set reads as 0.

## Variables and effects

An effect changes a variable:

| Written | Meaning |
|---|---|
| `name +N` | Add N. |
| `name -N` | Subtract N. |
| `name = N` (or `name=N`) | Set it to N. |
| `name` | Set it to 1. |

Separate several effects with `,`. Effects appear in `{...}` on an option, and after `@var`.
A variable is an integer. The story keeps two things: the **values** of its variables, and the
**seen** scenes, the ids of the scenes that play has entered. Both are saved with the game.

## Directives

A line that starts with `@` is a directive. Its word is slugged, so `@Fin` and `@fin` are the same:

| Directive | Meaning |
|---|---|
| `@fin` | Ends the conversation. |
| `@salta <label>` (or `@ir <label>`) | Jumps to a scene, unconditionally. |
| `@si <conditions> -> <label>` | Jumps when the conditions hold. Otherwise play goes on. |
| `@var <effects>` | Changes variables. |
| `@vn <label>` | Hands the conversation to the visual novel, at that scene. |
| stage directives | Handed to the game's stage (below). |

Any other directive is a problem: `orden desconocida o ilegible: @<text>`. An `@si` with no `->`
is a problem too.

A target of a jump, an option or `@vn` must be a label of the same file, or a `file.label` of
another file. For a label of this file, a missing target is a problem, reported on the line of
the jump (or of the `?` line, for an option): `no hay ninguna escena «<target>» en este guion`.
A target in another file is not checked here; at run time, see Playing below. The target `fin`
is allowed for a jump and for an option: it ends the conversation.

### Stage directives

These directives are for the game's stage. The 3D dialogue box does not show them; the
conversation hands them over in order, in the `stage` of the next box. velvet-guion knows these
words:

`sala`, `fondo`, `cg`, `quitar`, `sprite`, `musica`, `izq`, `der`, `pose`, `titulo`, `sello`,
`sacude`, `bote`, `destello`, `barrido`, `objeto`, `panel`, `revela`, `giro`, `desbloquea`,
`cierre`, `teclas`, `llevar`, `recuerdo`.

What each one means is up to the game. Their argument is the rest of the line, as written. For
example, `@fondo posada/entrada` has the command `fondo` and the argument `posada/entrada`, and
`@cierre` has no argument.

## The cast

A game gives the guiones a **cast**, which decides who a speaker is:

- `characters`: each character id with the names that the guiones use for it. Names are slugged,
  so `La Capitana` and `la capitana` are the same name. A name that the cast does not list is its
  own id, slugged: `Ana María` is `ana_maria`. By default `narrador` and `narrator` are the narrator.
- `thinker`: the character whose thoughts are the bracketed lines. Their thoughts are spoken by
  `<id>_thought`, and shown under the name given.

## Portraits

The faces of the dialogue box are listed in a `retratos.json`: for each character, one entry per
mood. The mood `normal` is the default. A mood may name another mood as a string, which makes it a
synonym. A thought shows its thinker's face.

```json
{
  "ana": {
    "normal": { "imagen": "retratos/ana", "recorte": [120, 40, 512, 512] },
    "enfadada": { "imagen": "retratos/ana_enfadada", "recorte": [120, 40, 512, 512] },
    "enojada": "enfadada"
  }
}
```

`imagen` is the image's asset key, without its extension. `recorte` is the square
`[x, y, width, height]`, in the image's pixels, that holds the head and shoulders. A mood that is
not listed shows the normal face.

## Playing a conversation

A conversation starts at a scene. Before its first line or choice, it runs the scene's variables,
jumps and stage directives, in order. Then:

- A **line** waits for the player to go on. Going on runs what follows it, up to the next line,
  choice or end.
- A **choice** waits for the player to take an option. Taking one that is closed is refused. An
  option that is taken applies its effects, then goes to its target, or on to the next step.
- A **jump** whose condition holds moves play to its scene. A jump to a scene that does not exist
  ends the conversation.
- `@vn` hands the conversation to the visual novel. The 3D conversation does not go on.
- `@fin`, `-> fin`, and the end of a scene that does not run on into another, end the conversation.

A scene is marked **seen** when play enters it: when the conversation starts there, when a jump or
an option leads there, and when play runs on into it.

### Choosing the scene to play

`entry` finds the scene to play when the player talks to an NPC or uses a place. It takes the first
scene, in file order (the files in the order they were added), that:

- has `quien` equal to the NPC, or `en` equal to the place (either may be left out of the query);
- has all its `si` conditions true;
- is not a spent `una_vez` (a scene that is already seen).

## Problems

A file with problems still plays what it can. Each problem names its file and line.

| Message | Cause |
|---|---|
| `clave no válida en la cabecera: <key>` | A front matter key (other than `id`, `sala`, `titulo`) that does not follow the metadata's name rule. |
| `clave repetida en la cabecera: <key>` | A front matter key given twice (`titulo` and `title` count as one key). |
| `atributos ilegibles en: <heading>` | Braces that are not valid attributes. |
| `escena sin nombre` | A scene heading with no label. |
| `escena repetida: <label>` | Two scenes with the same label in one file. |
| `orden desconocida o ilegible: @<text>` | A directive that is not known, or whose argument is not valid. |
| `línea sin texto` | A line with no text after its speaker. |
| `opción ilegible: <text>` | An option whose conditions or effects are not valid. |
| `no hay ninguna escena «<target>» en este guion` | A target label that this file does not have. |
| `una elección sin opciones` | A choice with no options (reported once per scene). |

## Compatibility

A reader that meets a directive or an attribute it does not know reports it as a problem and carries
on, skipping that directive or attribute. A file written for a newer version therefore still
plays, with those parts left out.

## Playing from JavaScript

`js/packages/guion` (`@velvet/guion`) gives the format as a typed API: `parseGuion` reads one file,
`GuionLibrary` holds a game's files and finds scenes, `Conversation` plays a scene box by box, and
`Portraits` finds the faces of the dialogue box.
The boxes are the JSON that `crates/velvet-wasm` returns: `type` is `line`, `choice`, `vn` or `end`,
and each box has its `scene` and its `stage`. The field names are camelCase. A file's summary (what
`parseGuion` and `add` return) has the fields `file`, `id`, `title`, `sala`, `scenes`, `problems` and
`meta`, where `meta` is the object of the metadata (see [Metadata](#metadata)).

## Conformance

The fixtures in `docs/formats/fixtures/guion/` are the shared tests of this format. Each fixture is a
guion file `<name>.md` and an `<name>.expected.json` that gives:

- `parse`: the summary of the file (its id, title, room, scenes with their attributes, problems and
  metadata). `cabecera.md` and `cabecera-errores.md` check the metadata: its order, its values, and the
  keys that break the rule or repeat;
- `entries`: what `entry` finds for given NPCs, places and variables;
- `runs`: for each run, the variables, the start scene, the box after each action (`next`, or
  `choose` with an option index), the code of each refused action, and the variables at the end.

The Rust test (`cargo test -p velvet-wasm`) and the JavaScript tests (`pnpm test` in `js/`) both
run every fixture. A runtime that disagrees with a fixture does not conform.
