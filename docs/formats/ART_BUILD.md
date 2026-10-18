# Art build v1 (`velvet-art-build`): source pictures, the build config and the report

Art build is tooling, not runtime. It fills a game's `art/` folder from **source pictures**: AI
generations, drawings and photographs of any size. Each source is fitted to the box of its art id in the
art manifest (`docs/formats/ART_MANIFEST.md`), written at the output size in the formats that the runtime
prefers, and reported. The game never reads the build config, the sources, the stamps or the report. It
reads `art/manifest.json` and the files of `art/`.

This document is normative for version 1. The reference implementation is `crates/velvet-artbuild`
(`src/` holds the rules; `tests/` holds the checks of this document, and the manifest fixtures of
`docs/formats/fixtures/art/`). Where this text and the crate disagree, fix both in the same change.

## Conventions

- JSON is UTF-8. Field names are `snake_case`. A reader refuses a name that the format does not define
  (`unknown-field`). This is stricter than the runtime formats: every name here changes a picture, so a
  misspelt name must not pass silently.
- Ids, variant names and file names follow the art manifest: an id matches `^[a-z0-9_-]+(\.[a-z0-9_-]+)*$`
  (at most 128 characters), a variant matches `^[a-z0-9_-]+$` (at most 32 characters).
- A **box** is the `size` of an entry: logical pixels of the 1920×1080 stage.
- An **output** is a file in `art/`. Its size is the box times the **scale**, in pixels.
- A **name** is the stem of a file: the id (the base picture), or `<id>.<variant>` (a variant).

## Folders

```text
game/
  art/
    manifest.json                          the art manifest (read by the build, and by the game)
    .build-stamps.json                     the stamps of the outputs (written by the build)
    portrait/character.lyra.portrait.avif  an output
    card/card.wood.webp
    card/card.wood.png
  art-src/
    art-build.json                         the build config (optional)
    character.lyra.portrait.png            a source for the base picture of an id
    character.lyra.portrait.happy.jpg      a source for a variant
    background/bg.forest.png               a source found in the kind's folder
```

The source folder is `art-src/` in the game folder, unless the command names another one (see
[Command](#command)). The build config is read from the source folder.

## Documents

Every document has this envelope:

| Field | Value |
|---|---|
| `format` | `"velvet-art-build"` |
| `version` | `1` |

A reader refuses a document whose `format` or `version` it does not read (`format`, `version`).

## Build config (`art-src/art-build.json`)

The file is optional. Without it, every field has its default. It is one object:

| Field | Type | Default | Meaning |
|---|---|---|---|
| `format` | `"velvet-art-build"` | required | The envelope. |
| `version` | `1` | required | The envelope. |
| `formats` | array of 1 to 3 distinct names, each `avif`, `webp` or `png` | `["avif", "webp", "png"]` | The formats to write. `--formats` on the command line replaces it. |
| `quality` | object with `avif` and `webp`, each an integer from 1 to 100 | `{ "avif": 60, "webp": 85 }` | The quality of the lossy formats. PNG is lossless and has no quality. |
| `scale` | integer from 1 to 4 | `1` | Output pixels per logical pixel. |
| `max_bytes` | integer, at least 1 | `1048576` (1 MiB) | The size limit of one output file, for the report. |
| `ids` | object: art id → override | `{}` | Per-id overrides, see below. |

An override (a value of `ids`) is an object with these optional fields:

| Field | Type | Default | Meaning |
|---|---|---|---|
| `fit` | `"cover"`, `"contain"` or `"stretch"` | `"cover"` | How the source fills the box. See [Fit](#fit). |
| `focus` | `[x, y]`, two numbers from 0 to 1 | `[0.5, 0]` for `portrait` and `fullbody`; `[0.5, 0.5]` for the other kinds | Which part of the picture a `cover` crop keeps. Ignored by the other fits. |
| `source` | a file name such as `lyra-final.png` | none | The source of the **base picture** only. The file is looked up as described in [Sources](#sources). Variants keep the default names. The name has no folder part, and its extension is one of `png`, `jpg`, `jpeg` or `avif`, in lower case. |

Rules:

- Every key of `ids` must be an id that the art manifest declares (`unknown-id`).
- `quality.avif` is turned into the encoder's CRF, see [AVIF](#avif).
- A `scale` above 1 makes the outputs larger. It does not change the box, the sources or the aspect checks.

Example:

```json
{
  "format": "velvet-art-build",
  "version": 1,
  "formats": ["avif", "webp", "png"],
  "quality": { "avif": 60, "webp": 85 },
  "scale": 1,
  "max_bytes": 1048576,
  "ids": {
    "character.lyra.portrait": { "fit": "cover", "focus": [0.5, 0] },
    "bg.forest": { "fit": "contain", "source": "forest-hd.jpg" }
  }
}
```

## Sources

The source of a name is the first file found by these rules.

1. **Default names.** For the name `N` (the id or `<id>.<variant>`), look for `art-src/N.png`,
   `art-src/N.jpg`, `art-src/N.jpeg` and `art-src/N.avif`, in that order. If none exists, look for the same
   four names in `art-src/<kind>/`. The first file found is the source.
2. **Explicit name.** For a base picture whose id has a `source` override, the only candidates are
   `art-src/<source>` and then `art-src/<kind>/<source>`. Variants ignore `source`.

A name with no source is **missing**: its id shows a placeholder in the game, unless `art/` already has a
file for it (see [Report](#report)).

When more than one candidate exists, the first in the order above is used, and the others are ignored.

A source is read from its bytes, so a wrong extension does not matter. AVIF is decoded by `velvet-avif`. A
PNG or JPEG with an EXIF orientation is turned upright. A PNG with more than 8 bits a channel is reduced
to 8; an AVIF with more than 8 bits is refused. A source that cannot be read, or that is empty, is listed
as failed; the other names go on.

## Fit

The box is the entry's `size`. The output is `width × height` pixels, where the box is multiplied by the
scale. Every resize uses the Lanczos3 filter, on premultiplied alpha.

| Fit | What it does |
|---|---|
| `cover` | Takes the largest window of the source that has the box's proportions, and places it on each axis by the focus: `x = round((source_width − window_width) × focus_x)`, and the same for `y`. The window is resized to the output. Parts of the picture outside the window are not shown. |
| `contain` | Scales the whole source to fit inside the output without changing its proportions, centres it, and leaves the bands transparent. |
| `stretch` | Resizes the whole source to the output, changing its proportions. |

A source smaller than the output is enlarged. The build does not report enlargement.

## Outputs

- **Path.** `art/<kind>/<name>.<format>`, where `<format>` is `avif`, `webp` or `png`. The file is found by the
  runtime with the rules of `ART_MANIFEST.md`.
- **Size.** The box times the scale. An output whose side is more than 16383 pixels (the largest WebP side)
  is not written: it is listed as skipped.
- **Pixels.** RGBA. The picture keeps its transparency in every format.

### PNG

Written by the PNG encoder of the `image` crate: RGBA, 8 bits a channel, the default compression and an
adaptive filter. PNG needs no external program.

### WebP and AVIF (ffmpeg)

WebP and AVIF are written by **ffmpeg**, which must be on the PATH, and must have the encoders `libwebp`
(for WebP) and `libaom-av1` (for AVIF). The build asks `ffmpeg -hide_banner -encoders` once per run. When
ffmpeg is missing, or lacks an encoder, the format is skipped for every output, with the reason in the
report. The other formats are still written.

The picture goes to ffmpeg on standard input, as raw RGBA. The output path is the last argument. Words in
braces are filled in by the build: `{W}` and `{H}` are the output size, `{Q}` the quality of the
format, `{CRF}` the CRF of AVIF (see below), and `{OUTPUT}` the output path.

WebP:

```text
ffmpeg -hide_banner -loglevel error -nostats -y -f rawvideo -pix_fmt rgba -video_size {W}x{H} -i pipe:0 -frames:v 1 -c:v libwebp -quality {Q} -pix_fmt yuva420p -f webp {OUTPUT}
```

AVIF with transparency: the build uses this form when any pixel has an alpha below 255. The alpha is a
second stream, as one plane at full range, which is the AVIF alpha item:

```text
ffmpeg -hide_banner -loglevel error -nostats -y -f rawvideo -pix_fmt rgba -video_size {W}x{H} -i pipe:0 -filter_complex "[0:v]split=2[c][a];[c]format=yuv420p,setparams=color_primaries=bt709:color_trc=iec61966-2-1:colorspace=bt470bg:range=tv[color];[a]alphaextract,format=gray,setparams=colorspace=bt709:range=pc[alpha]" -map [color] -c:v:0 libaom-av1 -crf {CRF} -b:v 0 -cpu-used 6 -still-picture 1 -map [alpha] -c:v:1 libaom-av1 -crf {CRF} -b:v 0 -cpu-used 6 -still-picture 1 -frames:v 1 -f avif {OUTPUT}
```

AVIF without transparency: one stream.

```text
ffmpeg -hide_banner -loglevel error -nostats -y -f rawvideo -pix_fmt rgba -video_size {W}x{H} -i pipe:0 -vf "format=yuv420p,setparams=color_primaries=bt709:color_trc=iec61966-2-1:colorspace=bt470bg:range=tv" -c:v libaom-av1 -crf {CRF} -b:v 0 -cpu-used 6 -still-picture 1 -frames:v 1 -f avif {OUTPUT}
```

The CRF of AVIF is `round((100 − quality) × 63 / 100)`, so the default quality 60 gives CRF 25 and quality
100 gives CRF 0.

### Atomic writes

Every output is written to a temporary dot file in the same folder, named `.<file name>.<pid>-<n>.tmp`, and
then moved to its final name, which replaces any file there. A failed write removes its temporary file. The
build deletes nothing else.

## Stamps (`art/.build-stamps.json`)

The stamps make the build incremental. The file is one object:

```json
{
  "format": "velvet-artbuild-stamps",
  "version": 1,
  "outputs": {
    "card/card.wood.webp": {
      "source": "card.wood.png",
      "source_sha256": "9f86d081…",
      "settings_sha256": "3a7bd3e2…"
    }
  }
}
```

- `source_sha256` is the SHA-256 of the source file's bytes, in lower-case hex.
- `settings_sha256` is the SHA-256 of the output's **effective settings**, written as JSON with its keys in
  sorted order. The settings are: `box`, `fit`, `focus` (`null` unless the fit is `cover`), `format`,
  `pipeline`, `quality` (`null` for PNG), `scale`. The `pipeline` number is `1` in this version.

An output is **up to date** when its file exists, its stamp exists, both hashes match, and `--force` is not
given. Otherwise it is built, and its stamp is written after the file. Stamps of outputs whose files are gone
are dropped when the stamps are saved. A missing or damaged stamps file means that nothing is up to date.

## Report

A build or a check ends with a report. The JSON form is an object with the envelope `format`
`"velvet-art-build-report"`, `version` `1`, and these lists:

| List | Each item | Meaning |
|---|---|---|
| `built` | `path`, `format`, `source`, `width`, `height`, `bytes` | Outputs written by this build. A check lists the outputs a build would write, with `bytes` null. |
| `up_to_date` | the same fields | Outputs that were not made again. |
| `missing` | `id`, `kind`, `name`, `variant`, `has_file` | Names with no source picture. `has_file` is true when `art/` already has a file for the name, which the game then shows. |
| `orphans` | a path inside `art/` | Files that no entry resolves to. See below. |
| `aspect` | `name`, `source`, `source_size`, `box_size`, `fit`, `difference_percent` | Sources whose proportions differ from the box by more than 15 percent. |
| `heavy` | `path`, `bytes`, `limit` | Files that the game may show, and that are larger than `max_bytes`. |
| `skipped` | `path`, `format`, `reason` | Outputs not written, because ffmpeg or its encoder is missing, or the output is too large. |
| `failed` | `path`, `message` | Sources that could not be read or decoded, outputs that failed to encode or write, and a failed stamps file. |

The top-level fields are `dry_run` (true for a check) and the lists above.

Rules that the report applies:

- **Orphans.** A file inside `art/` (at any depth) is an orphan unless it is `manifest.json`, the stamps
  file, or `<kind>/<name>.<extension>` where `<kind>` is the kind of an entry, `<name>` is that entry's id
  or one of its `<id>.<variant>` names, and `<extension>` is `avif`, `webp`, `png` or `svg`. A file is an
  orphan even when it is a leftover temporary file. The build never deletes an orphan.
- **Aspect.** Checked for `cover` and `stretch`. The source is compared with the box as
  `|source_width × box_height − source_height × box_width| × 100 > 15 × source_height × box_width`, in whole
  numbers. A `contain` fit shows the whole picture, so it is not checked.
- **Heavy.** Every file that resolves to an entry (as for orphans), including files that were not built by
  this run, and up-to-date files. A check measures the files that exist now.
- **Failed.** A failure does not stop the build. A check reports a failed source as a failure too.

The text form is for people. It starts with a summary line, then one section for each list that is not
empty, one line per item.

## Command

The `velvet` CLI runs the build through `velvet art`:

```text
velvet art build [--game <dir>] [--src <dir>] [--formats avif,webp,png] [--force] [--json]
velvet art check [--game <dir>] [--src <dir>] [--formats avif,webp,png] [--json]
```

- `--game` is the game folder, which holds `art/`. The default is the current folder.
- `--src` is the source folder. The default is `art-src/` in the game folder. The build config is read from
  the source folder.
- `--formats` lists the formats to write, separated by commas. It replaces the `formats` of the config.
- `--force` rebuilds every output. A check ignores it.
- `--json` prints the JSON report instead of the text.

`build` writes the outputs and the stamps. `check` writes nothing, not even the stamps.

Exit codes: `build` exits with 0, or with 1 when a source or an output failed. `check` exits with 0, or with 1
when the report has any problem: a missing source, an orphan, an aspect warning, a heavy file, a skipped output
or a failure.

## Errors

A build config or a manifest that breaks a rule is refused as a whole, and nothing is built. The refusal
lists every problem, each with its path (for example `ids["bg.forest"].fit`), and the code is the code of the
first one:

| Code | When |
|---|---|
| `json` | The text is not JSON. |
| `format` | The format tag is not `velvet-art-build`, or the manifest's is not `velvet-art`. |
| `version` | The version is not 1. |
| `structure` | A field has the wrong JSON type. |
| `value` | A value breaks its rule: a range, a name, a format, a file name, an anchor or a size. |
| `duplicate` | A format is listed twice in `formats`, or the manifest repeats a name (see `ART_MANIFEST.md`). |
| `unknown-field` | A name that this format does not define. |
| `unknown-id` | An override names an id that the manifest does not declare. |

A failure to read a folder or to write the report is an error of the command, not a code of this table.

## Versioning

This document is version 1. A reader refuses any other version. A new name is a new version, because a
misspelt name must not pass silently. A change to a default, to the fit rules, to the naming of outputs, or to
the settings that the stamps hash needs version 2. A change to the pipeline's pixels that does not change the
format raises the `pipeline` number in the stamps, so that every output is built again.

## Outside the format

The format does not define:

- where the source pictures come from, or which tool made them; a source is any picture that the rules above
  find;
- how the game shows an id, or which placeholder it draws when no file exists: those are in
  `ART_MANIFEST.md`;
- how a picture is chosen from several candidates: a person places the chosen file in `art-src/`, under the
  name that the rules find;
- the installation of ffmpeg, and the choice of its encoders.

## Conformance

The crate's tests are the conformance suite of this document (`crates/velvet-artbuild/tests/`). They build
pictures in a temporary folder, run `build` and `check`, and read the outputs back: the box and scale, the
fits and the focus, variants and missing names, the stamps, orphans, the check that writes nothing, the aspect
and size warnings, the skipped formats, the refusals, and the command's exit codes. The WebP and AVIF cases
need ffmpeg with `libwebp` and `libaom-av1`; without it they print a note and pass. The manifest fixtures of
`docs/formats/fixtures/art/` are read by the same crate.
