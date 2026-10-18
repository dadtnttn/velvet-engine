# velvet-locale v1 — localization catalogs

velvet-locale is the format of a game's translated text. A game keeps one folder of JSON catalog
files per language. A reader loads them, looks a key up, fills the holes of the text and falls back
to another language when a text is missing. The format describes no game, screen or script.

This document is normative for version 1. Two readers follow it: the TypeScript package
`@velvet/i18n` (`js/packages/i18n`) and the Rust reader that the Rust games will use. Both must pass
every case in the conformance fixtures, `docs/formats/fixtures/locale/` (see *Conformance*).

## Conventions

- Catalog files are JSON text in UTF-8 without a byte order mark. A leading BOM is an error.
- Keys and texts are compared as written: case-sensitive, not trimmed, not normalised. Writers should
  use Unicode NFC.
- A *letter* is a character of Unicode general category L.
- The order of entries in a file has no meaning. Readers must not depend on it.
- A reader ignores what the format does not define, and reports it with a warning (see *Loading*).

## Folder layout

```text
locales/                one folder per language
  es/                   the source language
    00_menus.json
    10_shop.json
    _notes.json         a tooling file: skipped
  en/
    00_menus.json
    50_patch.json
  pt-BR/
```

- A folder is named by its language tag, in canonical case (see *Languages*): `pt-BR`, never
  `pt-br`. Windows and macOS file systems do not tell those two apart.
- A reader loads every file in the folder whose name ends in `.json` and does not start with `_`.
  Other files and subfolders are ignored.
- Files are read in **name order**: the order of their names by Unicode code point, whatever order
  the file system lists them in. Zero-padded prefixes (`00_`, `10_`) set the order by hand. When two
  files set the same key, the file read later wins.

## Catalog file

```json
{
  "format": "velvet-locale",
  "version": 1,
  "language": "pt-BR",
  "note": "Translated in October. Readers ignore this field.",
  "meta": { "name": "Português (Brasil)", "script": "Latn", "direction": "ltr", "font": "sans" },
  "entries": {
    "menu.start": "Começar",
    "shop.coins": { "one": "{count} moeda", "other": "{count} moedas" }
  }
}
```

| Field | Required | Meaning |
|---|---|---|
| `format` | yes | The text `velvet-locale`. |
| `version` | yes | The integer `1`. Any other value is an error. |
| `language` | yes | The folder's language tag. Every file of a folder must name the same tag. |
| `note` | no | Any JSON value, for people. Readers ignore it without a warning. |
| `meta` | no | Facts about the language (below). Merged over the folder's files in name order: a field set later wins. |
| `entries` | yes | An object from key to value (below). |

Any other top-level field is an `unknown-field` warning and is ignored.

### meta

| Field | Type | Meaning |
|---|---|---|
| `name` | string | The language's name in the language itself, for a language picker: `Deutsch`, `日本語`. |
| `script` | string | The writing system: an ISO 15924 code, four letters with a capital first, such as `Latn`, `Hans`, `Hant`, `Hebr`, `Jpan`. |
| `direction` | `"ltr"` or `"rtl"` | The main direction of the text. The default is in *Direction*. |
| `font` | string | A hint for the platform's font choice, such as `sans`, `serif` or `cjk`. The engine decides what it means. |
| `shaping` | boolean | `true` when the text needs complex shaping: joined or reordered letters, combining marks (Devanagari, Thai, Hebrew with points). Default `false`. |
| `same_as_source` | list of keys | The keys that this language keeps as the source text on purpose (see [Same as source](#same-as-source)). |

A field of the wrong type or value is a `meta` error and is ignored; the other fields still apply.

### Same as source

A text that is the same in two languages is not always untranslated: a key name such as `Esc`, a letter such as
`F`, or a brand name is written alike on purpose. A language lists those keys in `meta.same_as_source`, an array of
keys:

```json
"meta": { "name": "English", "same_as_source": ["game.key.menu", "game.key.fast"] }
```

- Each item is a non-empty text. Any other value, and an array with any other item, is a `meta` error, and the
  field is ignored.
- The lists of all the files of one language are joined, so that each file may list its own keys. This is the one
  `meta` field that does not follow "a field set later wins".
- The list changes no lookup. A listed key is looked up as any other key.
- In the source language the list has no effect.
- Tooling does not report a listed key as untranslated. It reports a listed key as a warning
  (`stale-same-as-source`) when the language has no entry for it, when the source has no entry for it (an id key),
  or when the language's text is not the source's. Such a key should come off the list.
- A reader that predates the field reports it as `unknown-field`, which is only a warning.

## Keys

A key is any string. A game uses one of two styles for its texts. The style is the game's choice,
and the file does not say which one it uses. Both styles are looked up the same way, so one game can
mix them.

- **Id keys** are short names with dots: `menu.start`, `shop.coins`. The source language's catalogs
  hold every id the game uses, with the source text as the value. Tooling that scans the game's code
  for ids reports any id the source lacks.
- **Text keys** are the source text itself, as the game draws it, written in template syntax (so a
  literal brace is doubled). The source language needs no entry: its text is the key. A translation
  is an entry whose key is the source text, such as `"¿Cuánto cuesta?": "How much does it cost?"`.

Ids are the safer choice for a project that will change its source text: with a text key, a changed
source text becomes a new key with no translation. Keys built from a file name and a position (such
as the engine's string extractor makes) change whenever lines move, so they should not be used.

## Values and templates

A value is either:

- a **string**: a template, or
- a **plural object**: `{ "one": "…", "other": "…" }`, whose values are templates (see *Plurals*).

Any other JSON type is a `value` error, and that entry is ignored.

### Template syntax

| Written | Meaning |
|---|---|
| any text | itself |
| `{0}`, `{1}`, … | a positional hole: the argument at that index |
| `{gold}`, `{item_name}` | a named hole: the argument with that name |
| `{{` | a literal `{` |
| `}}` | a literal `}` |

- A positional name is a canonical decimal number: `0`, `12`, but not `01`, `+1` or `-1`.
- A named hole matches `[A-Za-z_][A-Za-z0-9_]*`.
- Anything else inside braces, an unclosed `{` and a `}` that is not doubled are a `template` error,
  and that entry is ignored.
- `count` is reserved for plurals (below).
- Argument values are inserted as they are. Their braces are not parsed.
- A hole with no argument is written as `{name}` and reported at lookup time (*Loading*).

### The hole rule

A translation must use exactly the holes of its source text: the same set of names. A hole may appear
more than once, and in any order, because word order differs between languages. A translation that
drops a hole, or adds one, has a `holes` error and its entry is ignored, so the lookup falls back.

The source text of a key is the source language's entry for that key. A key that the source language
has no entry for (every text key) has the key itself as its source text. For a plural entry, the
holes are those of all its forms together.

## Plurals

A plural object's keys are the CLDR plural categories: `zero`, `one`, `two`, `few`, `many` and
`other`. `other` is required; it is the form used when no other category applies. Any other key is a
`plural` error, as is a form that is not a string or an object without `other`.

- The category comes from `Intl.PluralRules` (CLDR data) for the language whose catalog holds the
  entry. A category the entry lacks falls back to `other`.
- A plain string behaves like an object with only `other`.
- `plural(key, count, args)` fills the hole `{count}` with `count`, and the other holes from `args`.
- `t(key, args)` on a plural entry uses `other`.
- Exact-number forms such as `=0` are not part of version 1. Use `zero` where the language has it,
  or a separate key.

Rust readers use the same CLDR categories (through ICU4X or a table with the same rules).

## Languages and fallback

To look up key `K` for the requested language `T`, the reader tries these steps in order. The first
catalog that has `K` answers, with its whole entry:

1. `T`.
2. The base of `T`: its tag without the last subtag (`pt-BR` gives `pt`, `zh-Hant-TW` gives
   `zh-Hant`), when `T` has one.
3. The fallback language `F`, which the game chooses. This step is skipped when `T` or its base is
   the source language `S`.
4. `S`, the source language.
5. `K` itself, read as a template. A text key shows its text; an id shows the id.

- A language with no catalog is skipped at its step.
- Holes and plural forms are never mixed between languages. A plural entry uses the categories of the
  language that answered.
- If no catalog has `K`, step 5 answers and the reader reports `K` as missing for the requested
  language, once per language and key. The source language never reports a missing key: its text is
  the key, and tooling checks its ids. In a text-keyed game, each untranslated text is reported in
  every other language: that is the list translators need.
- A key found only in `S` or `F` is not missing. It is untranslated; validation lists it.
- The steps depend on the tags alone, not on the order in which catalogs were loaded.

### Languages

A tag in version 1 is `language`, `language-Script`, `language-REGION` or
`language-Script-REGION`. `language` is two or three lowercase letters, `Script` is four letters with
a capital first, and `REGION` is two uppercase letters or three digits: `en`, `pt-BR`, `zh-Hans`,
`zh-Hant-TW`, `es-419`. Variants and extensions are not supported in version 1. Tags are compared
exactly, in canonical case.

The base of a tag is the tag without its last subtag: `zh-Hant-TW` gives `zh-Hant`, then `zh`. A tag
with one subtag has no base.

Mapping a platform's locale to a tag (a Windows language id, a system setting, or a regional choice
such as `pt-PT` read as `pt-BR`) belongs to the game, not to the format.

## Direction

A language's direction is its `meta.direction` when it has one. Otherwise it is `rtl` for the
primary languages `he`, `ar`, `fa` and `ur`, and `ltr` for every other language. Other right-to-left
languages declare `direction` in their meta block. The direction of a requested language without a
catalog comes from its primary subtag: `he-IL` is `rtl`.

The format stores text in logical order. Bidirectional layout (reordering runs, mirrored brackets)
belongs to the renderer.

## Loading: errors and warnings

Each problem is reported with a code. An **error** means the data is not used: a file or an entry is
ignored and the lookup falls back. A **warning** means the data is used, or that the problem is
harmless; it should still be fixed. Tooling fails a build on any error.

| Code | Severity | When | Ignored |
|---|---|---|---|
| `json` | error | the text is not JSON, or starts with a byte order mark | the file |
| `structure` | error | the top level is not an object, or `entries` is missing or not an object | the file |
| `format` | error | `format` is not `velvet-locale` | the file |
| `version` | error | `version` is not `1` | the file |
| `language` | error | `language` is missing, or is not a tag in canonical case | the file |
| `language-mismatch` | error | a file names a language other than its folder's | the file |
| `meta` | error | `meta` is not an object, or a field has the wrong type or value | that field |
| `unknown-field` | warning | a top-level or `meta` field the format does not define | nothing |
| `value` | error | an entry's value is neither a string nor a plural object | the entry |
| `plural` | error | a plural object has an unknown category, no `other`, or a form that is not a string | the entry |
| `template` | error | a template has bad syntax | the entry |
| `empty` | warning | an empty string for a key that has letters (the text would show nothing) | the entry |
| `holes` | error | a translation's holes differ from its source's (checked where the source is known) | the entry |
| `duplicate-key` | error | the same key twice in one file | the file |

- An empty string for a key without letters is valid: punctuation such as an opening exclamation mark
  that some languages do not write. The empty text is then the translation.
- `duplicate-key` is reported by readers that can see duplicates. A JSON parser that keeps the last
  value cannot see them, so such a reader cannot report this code.
- A reader that is given a folder whose name is not a tag in canonical case refuses the folder. That
  is a packaging error, not a file error.

At lookup time a reader reports two things, each once. A hole that had no argument is a
`missing-argument` warning, once per language, key and hole name; the text shows `{name}`. A key with
no text in any catalog of the chain is reported as missing, once per language and key, as described
under *Languages and fallback*. Neither report changes the text a lookup returns.

## Validation (tooling)

Validation checks a translation against its source. The translation is loaded without the holes check,
so that each hole is checked once. Validation reports the translation's own load issues (any code in
the table above, such as a bad `version`), and also:

- `holes` (error): the hole rule, for each key the two share.
- `missing-key` (warning): a source key with no translation. Lookups fall back for it.
- `unknown-key` (warning): a translation key the source does not have. A text-keyed game usually has
  an empty source, so it validates against its extraction list: a catalog that maps each text to
  itself.

## Conformance

`docs/formats/fixtures/locale/` holds the conformance suite:

- `locales/` is a small example game in five languages: `es` (the source), `en` (the fallback), `pt`
  (the base of `pt-BR`), `pt-BR` and `he` (right to left, with a plural form for two).
- `invalid/files/` holds one file per kind of error, and `invalid/holes/pt-BR/` is a complete
  translation with two hole mistakes.
- `cases.json` lists the expected results, in three sections:
  - `files`: one file read as one language. `expect.keys` is the set of keys that remain, `issues`
    the problems, and `meta` (when given) the merged meta block.
  - `validations`: a translation folder checked against a source folder, with the expected issues.
  - `scenarios`: language folders and a list of lookups. A `lookups` entry has `language`, `key`,
    optional `args` (positional as a list, named as an object) and optional `count`. Its `expect` has
    the `text`, the chain step that answered (`via`: a tag, or `key`), `missing` (whether this lookup
    made the missing report, default `false`) and the `issues` codes that lookup raised (default none).
    A scenario may read its files in reverse order (`"order": "reversed"`), to check that name order
    decides. `loadIssues` lists the issues raised while the scenario's languages load. `directions`
    lists each language's direction.

Each reader runs every case. Issues are compared as a set on severity, code and, when given, key and
language: order does not matter.

## Compatibility

- A reader refuses any `version` other than `1`.
- Within version 1, new optional fields may be added to `meta` and at the top level. Older readers
  ignore them, with an `unknown-field` warning.
- A new version is needed for a new value type, a change to the template syntax, a new fallback step,
  or a different plural rule.

## Outside the format

The format does not define:

- text composition: splitting sentences, separators, quotation marks, upper-case variants, or
  translating a text line by line. An engine may offer these, but the text they produce is not the
  format's answer;
- finding a key from a text that is already filled in, by matching it against templates. A lookup
  takes the key and its arguments, so a caller that holds a finished sentence must keep its key;
- pictures that carry words (signs, maps): a separate image catalog, if a game needs one;
- other catalog formats (`.po`, `.properties`), and the converters between them: tooling;
- number, date and currency formats: the caller formats a value and passes the text as an argument;
- line breaking, font loading and text measurement;
- notes for translators and glossaries: they belong in the `note` field or in `_` tooling files.
